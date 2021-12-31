use futures_util::SinkExt;
use hyper_tungstenite::hyper::upgrade::Upgraded;
use hyper_tungstenite::tungstenite::{self, Message};
use hyper_tungstenite::WebSocketStream;

use crate::steam::Unlock;

#[derive(Debug)]
pub enum MessageSendError {
    Serde(serde_json::Error),
    Transport(tungstenite::Error),
}

impl From<serde_json::Error> for MessageSendError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl From<tungstenite::Error> for MessageSendError {
    fn from(e: tungstenite::Error) -> Self {
        Self::Transport(e)
    }
}

pub async fn handle_emit(
    socket: &mut WebSocketStream<Upgraded>,
    unlock: Unlock,
) -> Result<(), MessageSendError> {
    let encoded = serde_json::to_vec(&unlock)?;
    let msg = Message::Binary(encoded);
    socket.send(msg).await?;

    Ok(())
}

pub fn handle_recv(msg: Message) -> bool {
    match msg {
        Message::Close(_) => {
            log::info!("received close, shutting down");
            true
        }
        _ => false,
    }
}
