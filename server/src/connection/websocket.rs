use super::*;

use futures_util::{stream::StreamExt, SinkExt};
use simd_json::serde::{from_slice_with_buffers as from_bytes, from_str_with_buffers as from_str};
use tokio_tungstenite::{
  accept_async_with_config,
  tungstenite::{protocol::WebSocketConfig, Error as WsError, Message},
  WebSocketStream,
};
use IoErrorKind::*;

pub struct WebSocketConnection<S: RawStream>(WebSocketStream<S>, simd_json::Buffers);

#[inline]
fn map_err(e: WsError) -> Error {
  match e {
    WsError::ConnectionClosed | WsError::AlreadyClosed => Error::Closed,
    WsError::Io(e) => Error::IoError(e),
    WsError::Capacity(_) => Error::IoError(IoError::new(InvalidData, "Outgoing message too large")),
    WsError::WriteBufferFull(_) => Error::ServerError("WebSocket write buffer full".into()),
    _ => Error::BadRequest(e.into()),
  }
}

impl<S: RawStream> WebSocketConnection<S> {
  pub async fn convert_stream(stream: S) -> Result<Self, WsError> {
    let cfg = WebSocketConfig {
      max_message_size: Some(CONFIG.max_body_size),
      ..Default::default()
    };

    let ws = accept_async_with_config(stream, Some(cfg)).await?;
    Ok(Self(ws, simd_json::Buffers::new(256)))
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for WebSocketConnection<S> {
  #[inline]
  async fn send(&mut self, res: Response) -> Result<(), Error> {
    let string = check!(srv: simd_json::to_string(&res))?;
    self.0.send(Message::Text(string)).await.map_err(map_err)
  }

  async fn recv(&mut self) -> Result<Request, Error> {
    match self.0.next().await {
      None => Err(Error::Closed),
      Some(msg) => match check!(req: msg)? {
        Message::Text(mut s) => check!(req: unsafe { from_str(&mut s, &mut self.1) }),
        Message::Binary(mut b) => check!(req: from_bytes(&mut b, &mut self.1)),
        Message::Close(_) => Err(Error::Closed),
        _ => Err(Error::BadRequest("Invalid WebSocket message type".into())),
      },
    }
  }

  #[inline]
  async fn close(&mut self) -> Result<(), Error> {
    self.0.close(None).await.map_err(map_err)
  }
}
