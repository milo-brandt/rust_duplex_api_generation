use axum::{Router, routing::{post, get}, Json, Server, extract::WebSocketUpgrade, response::Response};
use futures::{join, StreamExt};
use serde::{Serialize, Deserialize};
use tokio::{sync::mpsc, task::{LocalSet, spawn_local}};
use tower_http::{cors::{CorsLayer, Any}, catch_panic::CatchPanicLayer};
use std::{net::{Ipv4Addr, SocketAddr, IpAddr}, task::{Context, Waker}, cell::{UnsafeCell, RefCell}, sync::Mutex};

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

/*
async fn websocket(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|ws| async move {
        // TODO: Could add lifetimes various places to avoid the allocation here.
        struct MPSCSender(mpsc::UnboundedSender<(u64, String)>);
        impl channel_listener::Sender for MPSCSender {
            fn send(&self, channel: u64, message: String) {
                // ignore errors...
                drop(self.0.send((channel, message)))
            }
        }

        let (sender, receiver) = mpsc::unbounded_channel();
        // the mutex isn't really needed... but not sure of a way to tell Rust that it's okay that
        // this object and the references to it might get sent over thread boundaries _together_.
        let endpoint = Mutex::new(DuplexEndpoint::new(MPSCSender(sender)));
        let ws = RefCell::new(ws);
        join! {
            // sending loop
            async {

            },
            // receiving loop
            async {

            }
        };
        // drop the websocket
        drop(ws.into_inner().close().await);
    })
}
*/