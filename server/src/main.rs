use axum::{Router, routing::{post, get}, Json, Server, extract::{WebSocketUpgrade, ws}, response::Response};
use futures::{join, StreamExt, SinkExt, future::ready};
use protocol_util::{sender::Sender, receiver::{create_listener_full, FullListenerCreation}, communication_context::Context, channel_allocator::{ChannelAllocator, TypedChannelAllocator}};
use serde::{Serialize, Deserialize};
use tower_http::{cors::{CorsLayer, Any}, catch_panic::CatchPanicLayer};
use std::{net::{Ipv4Addr, SocketAddr, IpAddr}, task::{self, Waker}, cell::{UnsafeCell, RefCell}, sync::{Mutex, Arc}};
use futures::channel::mpsc;
use std::future::Future;

#[tokio::main]
async fn main() {
   let cors = CorsLayer::new()
      .allow_methods(Any)
      .allow_origin(Any)
      .allow_headers(Any);

   let app = Router::new()
      .route("/echo", post(echo))
      .route("/ws", get(websocket))
      .layer(CatchPanicLayer::new())
      .layer(cors);
   
   Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000))
      .serve(app.into_make_service())
      .await
      .unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
struct X {
   value: u64
}

async fn echo(
   message: Json<X>,
) -> Json<X> {
   println!("CALLED WITH {:?}", message);
   Json(X { value: message.value * 5 })
}

pub async fn run_service(context: Context, listener_future: impl Future<Output=()> + Unpin + Send) {
    let echo_channel = context.channel_allocator.incoming::<protocol_types::EchoMessage>();
    let echo_handle = context.controller.listen(echo_channel, {
        let context = context.clone();
        move |message| {
            println!("Received message: {:?}", message.message);
            let send_back = format!("{}{}", message.message, message.message);
            // Should really be detaching this; but since it's ready, it's okay to do this this way.
            let future = message.future.receive_feed(&context, ready(send_back));
            future
        }
    });
    listener_future.await;
    drop(echo_handle);
}

async fn websocket(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|ws| async move {
        // to share between the loops
        let (out_sender, mut out_receiver) = mpsc::unbounded();
        let sender = Sender::new(out_sender);
        let (mut ws_send, mut ws_receive) = ws.split();
        let FullListenerCreation {
            future,
            controller,
            sender: receiver,
        } = create_listener_full();
        let context = Context {
            channel_allocator: Arc::new(TypedChannelAllocator::new()),
            controller,
            sender,
        };
        let service_future = run_service(context, future);

        let (ws_send, ws_receive, _) = join! {
            async move {
                loop {
                    match ws_receive.next().await {
                        Some(Ok(ws::Message::Text(text))) => {
                            if let Some((channel, message)) = text.split_once(':') {
                                if let Ok(channel_id) = channel.parse() {
                                    receiver.send(channel_id, message.into());
                                }
                            }
                        },
                        // TODO: Handle close messages correctly
                        Some(_) => (),
                        None => break
                    }
                }
                ws_receive
            },
            async move {
                while let Some(next_message) = out_receiver.next().await {
                    drop(ws_send.send(ws::Message::Text(format!("{}:{}", next_message.0, next_message.1))).await);
                }
                ws_send
            },
            // run the listener
            service_future,
        };
        let ws = ws_send.reunite(ws_receive).unwrap();
        // drop the websocket
        drop(ws.close().await);
    })
}