use super::*;

use futures_util::{stream::StreamExt, SinkExt};
use simd_json::serde::{from_slice_with_buffers as from_bytes, from_str_with_buffers as from_str};
use tokio_tungstenite::{
  accept_async_with_config,
  tungstenite::{protocol::WebSocketConfig, Error as WsError, Message},
  WebSocketStream,
};

pub struct WebSocketConnection<S: RawStream>(WebSocketStream<S>, simd_json::Buffers);

impl<S: RawStream> WebSocketConnection<S> {
  #[inline] // we only call this once, just inline
  pub async fn convert_stream(stream: S) -> Result<Self, WsError> {
    let cfg = WebSocketConfig {
      max_message_size: Some(CONFIG.max_message_size),
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
    match self.0.send(Message::Text(string)).await {
      Ok(_) => Ok(()),
      Err(WsError::Capacity(_)) => Err(ResponseTooLarge.into()),
      Err(e) => Err(e.into()),
    }
  }

  async fn recv(&mut self) -> Result<Request, Error> {
    match self.0.next().await {
      None => Err(Closed.into()),
      Some(msg) => match msg.map_err(Error::from)? {
        Message::Text(mut s) => check!(req: unsafe { from_str(&mut s, &mut self.1) }),
        Message::Binary(mut b) => check!(req: from_bytes(&mut b, &mut self.1)),
        Message::Close(_) => Err(Closed.into()),
        _ => Err(Ignored.into()),
      },
    }
  }

  #[inline]
  async fn close(&mut self) -> Result<(), Error> {
    self.0.close(None).await.map_err(|_| Closed.into())
  }
}
