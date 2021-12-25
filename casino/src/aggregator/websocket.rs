use std::convert::Infallible;

use futures_util::SinkExt;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::WebSocketStream;

use crate::steam::Unlock;

pub async fn handle_emit(
    socket: &mut WebSocketStream<Upgraded>,
    unlock: Unlock,
) -> Result<(), Infallible> {
    let encoded = serde_json::to_vec(&unlock).unwrap();
    let msg = Message::Binary(encoded);
    socket.send(msg).await.unwrap();

    Ok(())
}

pub async fn handle_recv(msg: Message) -> Result<bool, Infallible> {
    Ok(match msg {
        Message::Close(_) => {
            eprintln!("received close, shutting down");
            true
        }
        _ => false,
    })
}
