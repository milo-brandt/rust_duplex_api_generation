use std::sync::Arc;

use futures::{StreamExt, join, SinkExt, FutureExt};
use futures::channel::{mpsc, oneshot};
use protocol_types::{EchoMessage, EchoResponseRx, EchoMessageTx};
//use protocol_types::{EchoMessage, EchoMessageTx, EchoResponseRx};
use protocol_util::base::{ChannelCoStream, Channel};
use protocol_util::channel_allocator::TypedChannelAllocator;
use protocol_util::communication_context::Context;
use protocol_util::generic::{Receivable, DefaultSendable};
use protocol_util::receiver::{FullListenerCreation, create_listener_full};
use protocol_util::sender::Sender;
use protocol_util::spawner::Spawner;
use reqwasm::websocket;
use sycamore::prelude::*;
use sycamore::futures::{create_resource, spawn_local};
use reqwasm::websocket::futures::WebSocket;
use web_sys::{Event, KeyboardEvent};
use wasm_bindgen::JsCast;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    println!("HELLO!");
    let ws = WebSocket::open("ws://localhost:3000/ws").unwrap();

    let (out_sender, mut out_receiver) = mpsc::unbounded();
    let sender = Sender::new(out_sender);
    let (mut ws_send, mut ws_receive) = ws.split();
    let FullListenerCreation {
        future: service_future,
        controller,
        sender: receiver,
    } = create_listener_full();
    let context = Context {
        channel_allocator: Arc::new(TypedChannelAllocator::new()),
        controller,
        sender,
        spawner: Spawner::new(spawn_local)
    };
    spawn_local(async move {
        let (ws_send, ws_receive, _) = join! {
            async move {
                loop {
                    match ws_receive.next().await {
                        Some(Ok(websocket::Message::Text(text))) => {
                            if let Some((channel, message)) = text.split_once(':') {
                                if let Ok(channel_id) = channel.parse() {
                                    log::debug!("RECEIVING: {:?}", (channel_id, message));
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
                    log::debug!("SENDING: {:?}", next_message);
                    drop(ws_send.send(websocket::Message::Text(format!("{}:{}", next_message.0, next_message.1))).await);
                }
                ws_send
            },
            // run the listener
            service_future,
        };
    });
    let echo_co_channel = ChannelCoStream::<EchoMessage>(Channel(0, Default::default()));

    let echo_send = echo_co_channel.receive_in_context(&context);



    sycamore::render(|cx| {
        /* spawn_local_scoped(cx, async move {
            loop {
                sleep(Duration::from_secs(1)).await;
                log::debug!("Hello world! {:?}", ws.state());
            }
        }); */

        let input_value = create_signal(cx, String::new());
        let values = create_signal(cx, Vec::<(String, RcSignal<Option<String>>)>::new());

        let keydown_handler = {
            let context = context.clone();
            move |event: Event| {
                let keyboard_event: KeyboardEvent = event.unchecked_into();
                if keyboard_event.key() == "Enter" {
                    let line = &*input_value.get();
                    let (sender, return_future) = oneshot::channel();
                    drop(echo_send.channel_send(EchoMessageTx {
                        message: line.clone(),
                        future: DefaultSendable(move |value| drop(sender.send(value))),
                    }));

                    log::debug!("Sending: {}", line);
                    /*
                    Potentially good optimization: wrap the returned RcSignal in something that stops polling once
                    a non-None result is returned. 
                    */
                    values.modify().push((line.clone(), create_resource(cx, return_future.map(|value| {
                        match value.unwrap().unwrap() {
                            EchoResponseRx::Alright(value) => format!("OK: {}", value),
                            EchoResponseRx::UhOh(err) => format!("ERR: {}", err)
                        }
                    }))));
                    input_value.set("".to_string());
                }
            }
        };
    
        view! { cx,
            Indexed(
                iterable=values,
                view=|cx, (send, received)| {
                    view! { cx, 
                        (send) ", " (format!("{:?}", received.get())) br{} 
                    }
                }
            )
            div(class="input") {
                input(class="input_box", type="text", on:keydown=keydown_handler, bind:value=input_value) {

                }
            }
        }
    });
}
