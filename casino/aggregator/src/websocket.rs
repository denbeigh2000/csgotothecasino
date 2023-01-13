use axum::extract::ws::{Message, WebSocket};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageSendError {
    #[error("ser/deserialisation error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("error sending message: {0}")]
    Transport(#[from] axum::Error),
}

pub async fn handle_emit<T: serde::ser::Serialize>(
    socket: &mut WebSocket,
    unlock: T,
) -> Result<(), MessageSendError> {
    let encoded = serde_json::to_string(&unlock)?;
    let msg = Message::Text(encoded);
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
