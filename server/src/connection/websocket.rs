use super::*;

use std::{io::ErrorKind as IoErrorKind, sync::atomic::Ordering::Relaxed};

use futures_util::{
  stream::{SplitSink, SplitStream, StreamExt},
  SinkExt,
};
use tokio_tungstenite::{
  accept_async_with_config,
  tungstenite::{protocol::WebSocketConfig, Error as WsError, Message},
  WebSocketStream as Stream,
};

pub struct WebSocketConnection<S: RawStream>(SplitSink<Stream<S>, Message>, SplitStream<Stream<S>>);

impl<S: RawStream> WebSocketConnection<S> {
  pub async fn convert_stream(stream: S) -> Result<Self, WsError> {
    let cfg = WebSocketConfig {
      max_message_size: Some(MAX_BODY_SIZE.load(Relaxed)),
      ..Default::default()
    };

    let ws = accept_async_with_config(stream, Some(cfg)).await?;
    let (write, read) = ws.split();
    Ok(Self(write, read))
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
    match res.status {
      Pong => Ok(()), // tungstenite sends pongs automatically, don't send
      _ => self.send_msg(Message::Text(res.to_string())).await,
    }
  }

  async fn recv(&mut self) -> Result<Request, Error> {
    match self.1.next().await {
      None => Err(Error::Closed),
      Some(msg) => match check_req!(msg) {
        Message::Text(s) => Ok(check_req!(Request::from_str(s.trim()))),
        Message::Binary(b) => {
          let request = check_req!(String::from_utf8(b));
          Ok(check_req!(Request::from_str(request.trim())))
        }
        // WebSocket spec forces payload to be <=125 bytes, don't check
        Message::Ping(payload) => Ok(Request::Ping { payload }),
        Message::Close(_) => Err(Error::Closed),
        _ => Err(Error::BadRequest),
      },
    }
  }
}
