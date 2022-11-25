use axum::extract::ws::{Message, WebSocket};
use thiserror::Error;

use steam::Unlock;

#[derive(Debug, Error)]
pub enum MessageSendError {
    #[error("ser/deserialisation error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("error sending message: {0}")]
    Transport(#[from] axum::Error),
}

pub async fn handle_emit(
    socket: &mut WebSocket,
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
