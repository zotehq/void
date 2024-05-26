use crate::{
  actions, logger,
  server::{connect, disconnect},
};
use futures_util::{
  stream::{SplitSink, SplitStream, StreamExt},
  SinkExt,
};
use protocol::{request::Request, response::Response};
use std::{fmt, io::Error as IoError, str::FromStr, sync::OnceLock};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_tungstenite::{
  accept_async_with_config,
  tungstenite::{protocol::WebSocketConfig, Error as WsError, Message},
  WebSocketStream,
};

// CONNECTION TRAIT

pub enum ReceiveError {
  ConnectionClosed,
  ConnectionError(IoError),
  MalformedRequest(bool), // response failed?
}

impl ReceiveError {
  #[inline]
  pub fn fatal(&self) -> bool {
    match *self {
      ReceiveError::MalformedRequest(failed) => failed,
      _ => false,
    }
  }
}

impl fmt::Display for ReceiveError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::ConnectionClosed => write!(f, "Connection closed")?,
      Self::ConnectionError(e) => write!(f, "Connection error: {}", e)?,
      Self::MalformedRequest(c) => {
        write!(f, "Malformed request from client")?;
        if *c {
          write!(f, " (response failed!)")?;
        }
      }
    }
    Ok(())
  }
}

#[async_trait::async_trait]
pub trait Connection: Send + Sync + Unpin {
  // if true, error occurred
  async fn send(&mut self, res: Response) -> bool;
  async fn recv(&mut self) -> Result<Request, ReceiveError>;
}

// IMPLEMENTATION HELPERS

pub struct ConfigCache {
  pub max_body_size: usize,
  pub ws_config: WebSocketConfig,
}

pub static CONFIG_CACHE: OnceLock<ConfigCache> = OnceLock::new();

#[inline(always)]
fn cfg() -> &'static ConfigCache {
  CONFIG_CACHE.get().unwrap()
}

pub trait RawStream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {}
impl<S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static> RawStream for S {}

macro_rules! wrap_malformed_req {
  ($self:ident, $in:expr) => {
    match $in {
      Ok(o) => o,
      Err(e) => {
        let responded = $self.send(Response::error("Malformed request")).await;
        let err = ReceiveError::MalformedRequest(!responded);

        logger::warn!("{}", err);
        logger::trace!(target: "conn_handler::handle_conn", "{}", e);

        return Err(err);
      }
    }
  };
}

// RAW TCP IMPLEMENTATION

pub struct TcpConnection<S: RawStream>(S);

impl<S: RawStream> From<S> for TcpConnection<S> {
  fn from(value: S) -> Self {
    Self(value)
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  async fn send(&mut self, res: Response) -> bool {
    match self.0.write_all(res.to_string().as_bytes()).await {
      Ok(_) => false,
      Err(e) => {
        logger::warn!("Connection error (specifically in writing response to client)");
        logger::trace!(target: "conn_handler::write_response", "{}", e);
        true
      }
    }
  }

  async fn recv(&mut self) -> Result<Request, ReceiveError> {
    let mut request: Vec<u8> = vec![0; cfg().max_body_size];

    match self.0.read(&mut request).await {
      Ok(0) => return Err(ReceiveError::ConnectionClosed),
      Ok(amt) => request.shrink_to(amt),
      Err(e) => {
        logger::warn!("Connection error while reading request from client");
        logger::trace!(target: "conn_handler::handle_conn", "{}", e);
        return Err(ReceiveError::ConnectionError(e));
      }
    };

    let request = wrap_malformed_req!(self, String::from_utf8(request));
    Ok(wrap_malformed_req!(self, Request::from_str(request.trim())))
  }
}

// WEBSOCKET IMPLEMENTATION

pub struct WebSocketConnection<S: AsyncRead + AsyncWrite + Send + Unpin>(
  SplitSink<WebSocketStream<S>, Message>,
  SplitStream<WebSocketStream<S>>,
);

impl<S: AsyncRead + AsyncWrite + Send + Unpin> WebSocketConnection<S> {
  pub async fn convert_stream(stream: S) -> Result<Self, WsError> {
    let ws = accept_async_with_config(stream, Some(cfg().ws_config)).await?;
    let (write, read) = ws.split();
    Ok(Self(write, read))
  }

  async fn pong(&mut self, ping: Vec<u8>) -> bool {
    self.0.send(Message::Pong(ping)).await.is_err()
  }
}

#[async_trait::async_trait]
impl<S: AsyncRead + AsyncWrite + Send + Unpin> Connection for WebSocketConnection<S> {
  async fn send(&mut self, res: Response) -> bool {
    self.0.send(Message::Text(res.to_string())).await.is_err()
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

// CONNECTION HANDLER

pub async fn handle_conn(conn: &mut dyn Connection) {
  connect();
  let mut authenticated = false;

  loop {
    let request = match conn.recv().await {
      Ok(r) => r,
      Err(ReceiveError::ConnectionClosed) => break,
      Err(e) if e.fatal() => return,
      _ => continue,
    };

    match request {
      Request::Auth { username, password } => {
        if authenticated {
          if conn.send(Response::error("Already authenticated")).await {
            return;
          }

          continue;
        }

        let auth_result = actions::auth(&username, &password);
        authenticated = auth_result.is_authenticated;
        if conn.send(auth_result.response).await {
          return;
        }
      }

      _ => todo!(),
    }
  }

  disconnect();
}
