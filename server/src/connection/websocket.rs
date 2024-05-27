use super::*;

use std::sync::atomic::Ordering::Relaxed;

use futures_util::{
  stream::{SplitSink, SplitStream, StreamExt},
  SinkExt,
};
use tokio_tungstenite::{
  accept_async_with_config,
  tungstenite::{protocol::WebSocketConfig, Error as WsError, Message},
  WebSocketStream,
};

pub struct WebSocketConnection<S: RawStream>(
  SplitSink<WebSocketStream<S>, Message>,
  SplitStream<WebSocketStream<S>>,
);

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

  #[inline]
  async fn write(&mut self, msg: Message) -> bool {
    if let Err(e) = self.0.send(msg).await {
      warn!("Connection error while writing response to client");
      trace!(target: "conn_handler::write_response", "{}", e);
      true
    } else {
      false
    }
  }

  async fn pong(&mut self, ping: Vec<u8>) -> bool {
    self.write(Message::Pong(ping)).await
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for WebSocketConnection<S> {
  async fn send(&mut self, res: Response) -> bool {
    self.write(Message::Text(res.to_string())).await
  }

  async fn recv(&mut self) -> Result<Request, ReceiveError> {
    match self.1.next().await {
      None => Err(ReceiveError::ConnectionClosed),
      Some(msg) => match wrap_malformed_req!(self, msg) {
        Message::Text(s) => Ok(wrap_malformed_req!(self, Request::from_str(s.trim()))),
        Message::Binary(b) => {
          let request = wrap_malformed_req!(self, String::from_utf8(b));
          Ok(wrap_malformed_req!(self, Request::from_str(request.trim())))
        }
        // handle pings, but still treat as a malformed request.
        Message::Ping(p) => Err(ReceiveError::MalformedRequest(self.pong(p).await)),
        Message::Close(_) => Err(ReceiveError::ConnectionClosed),
        _ => Err(wrap_malformed_req!(self, Err("Invalid message type"))),
      },
    }
  }
}
