pub mod generic;
use serde::{Serialize, Deserialize};

/*
#[derive(Serialize, Deserialize)]
pub enum Message {
    UserEnter(String),
    UserLeave(String),
    Message(String),
}
#[derive(Serialize, Deserialize)]
pub struct RoomResponse {
    messages: generic::ChannelStream<Message>, // parallel to this response.
    send_message: generic::ChannelCoStream<String>, // antiparallel; for user to send messages. sending None leaves channel.
}
#[derive(Serialize, Deserialize)]
pub struct RoomRequest {
    username: String,
    room_name: String,
    response: generic::ChannelCoFuture<RoomResponse>,
}
*/

#[derive(Serialize, Deserialize)]
pub struct EchoMessage {
    pub message: String,
    pub future: generic::ChannelCoFuture<String>,
}