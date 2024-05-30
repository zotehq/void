use super::*;

use std::io::ErrorKind as IoErrorKind;

use futures_util::{
  stream::{SplitSink, SplitStream, StreamExt},
  SinkExt,
};
use simd_json::serde::{from_slice_with_buffers as from_bytes, from_str_with_buffers as from_str};
use tokio_tungstenite::{
  accept_async_with_config,
  tungstenite::{protocol::WebSocketConfig, Error as WsError, Message},
  WebSocketStream as Stream,
};

pub struct WebSocketConnection<S: RawStream>(
  SplitSink<Stream<S>, Message>,
  SplitStream<Stream<S>>,
  simd_json::Buffers,
);

impl<S: RawStream> WebSocketConnection<S> {
  pub async fn convert_stream(stream: S) -> Result<Self, WsError> {
    let cfg = WebSocketConfig {
      max_message_size: Some(CONFIG.max_body_size),
      ..Default::default()
    };

    let ws = accept_async_with_config(stream, Some(cfg)).await?;
    let (write, read) = ws.split();
    Ok(Self(write, read, simd_json::Buffers::new(256)))
  }

  async fn send_msg(&mut self, msg: Message) -> Result<(), Error> {
    match self.0.send(msg).await {
      Ok(_) => Ok(()),
      Err(e) => Err(Error::IoError(match e {
        WsError::Io(e) => e,
        WsError::Utf8 => IoErrorKind::InvalidData.into(),
        _ => IoErrorKind::Other.into(),
      })),
    }
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for WebSocketConnection<S> {
  #[inline]
  async fn send(&mut self, res: Response) -> Result<(), Error> {
    self.send_msg(Message::Text(simd_json::to_string(&res).unwrap())).await
  }

  async fn recv(&mut self) -> Result<Request, Error> {
    match self.1.next().await {
      None => Err(Error::Closed),
      Some(msg) => match check_req!(msg) {
        Message::Text(mut s) => Ok(check_req!(unsafe { from_str(&mut s, &mut self.2) })),
        Message::Binary(mut b) => Ok(check_req!(from_bytes(&mut b, &mut self.2))),
        Message::Close(_) => Err(Error::Closed),
        _ => Err(Error::BadRequest),
      },
    }
  }
}
