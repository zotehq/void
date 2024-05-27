use crate::{config, logger, MapValue, MAP};
use protocol::{
  request::Request,
  response::{Response, ResponsePayload},
};

// std

use std::sync::atomic::{
  AtomicUsize,
  Ordering::{Relaxed, SeqCst},
};
use std::time::{Duration, SystemTime};
use std::{fmt, io::Error as IoError, str::FromStr};

// async (tokio/futures)

use futures_util::{
  stream::{SplitSink, SplitStream, StreamExt},
  SinkExt,
};
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

// CONNECTION IMPLEMENTATION HELPERS

// should be faster than running config::get() all the time
pub static MAX_BODY_SIZE: AtomicUsize = AtomicUsize::new(0);

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

// store max_body_size here too since its stored in the WebSocketConfig
pub struct TcpConnection<S: RawStream>(S, usize);

impl<S: RawStream> From<S> for TcpConnection<S> {
  fn from(value: S) -> Self {
    Self(value, MAX_BODY_SIZE.load(Relaxed))
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  async fn send(&mut self, res: Response) -> bool {
    if let Err(e) = self.0.write_all(res.to_string().as_bytes()).await {
      logger::warn!("Connection error while writing response to client");
      logger::trace!(target: "conn_handler::write_response", "{}", e);
      true
    } else {
      false
    }
  }

  async fn recv(&mut self) -> Result<Request, ReceiveError> {
    let mut request: Vec<u8> = vec![0; self.1];

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
    let request = request.trim_end_matches('\0').trim();
    Ok(wrap_malformed_req!(self, Request::from_str(request)))
  }
}

// WEBSOCKET IMPLEMENTATION

pub struct WebSocketConnection<S: AsyncRead + AsyncWrite + Send + Unpin>(
  SplitSink<WebSocketStream<S>, Message>,
  SplitStream<WebSocketStream<S>>,
);

impl<S: AsyncRead + AsyncWrite + Send + Unpin> WebSocketConnection<S> {
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
      logger::warn!("Connection error while writing response to client");
      logger::trace!(target: "conn_handler::write_response", "{}", e);
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
impl<S: AsyncRead + AsyncWrite + Send + Unpin> Connection for WebSocketConnection<S> {
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

// HANDLER IMPLEMENTATION HELPERS

pub static CURRENT_CONNS: AtomicUsize = AtomicUsize::new(0);

pub fn fmt_conns() -> String {
  let current_conns = CURRENT_CONNS.load(SeqCst);
  let max_conns = config::get().max_conns;

  format!(
    "({current_conns} {} / {max_conns} max)",
    if current_conns == 1 { "conn" } else { "conns" }
  )
}

async fn handle_map_value(key: String, val: MapValue) -> Option<ResponsePayload> {
  let expires_in = if let Some(st) = val.expiry {
    if let Ok(dur) = st.duration_since(SystemTime::now()) {
      Some(dur.as_secs())
    } else {
      let mut write = MAP.get().unwrap().write().await;
      write.remove(&key);
      drop(write);

      return None;
    }
  } else {
    None
  };

  Some(ResponsePayload {
    key,
    value: val.value,
    expires_in,
  })
}

#[macro_export]
macro_rules! wrap_overwrite {
  ($conn:ident, $key:expr, $write:ident => $expr:expr) => {
    let mut $write = MAP.get().unwrap().write().await;
    if let Some(val) = $expr {
      drop($write); // drop ASAP we don't need this lock anymore
      if let Some(payload) = handle_map_value($key, val.clone()).await {
        // value existed and hadn't expired, send as payload
        if $conn.send(Response::success_payload("OK", payload)).await {
          return;
        } else {
          continue;
        }
      }
      // value expired
    }
    // value already didn't exist

    if $conn.send(Response::success("OK")).await {
      return;
    }
  };
}

// CONNECTION HANDLER

pub async fn handle_conn(conn: &mut dyn Connection) {
  CURRENT_CONNS.fetch_add(1, SeqCst);
  logger::info!("Connection established {}", fmt_conns());

  let mut authenticated = false;

  loop {
    let request = match conn.recv().await {
      Ok(r) => r,
      Err(ReceiveError::ConnectionClosed) => break,
      Err(e) if e.fatal() => return,
      _ => continue,
    };

    match request {
      Request::Auth { .. } if authenticated => {
        logger::debug!("Redundant AUTH attempted");
        if conn.send(Response::error("Already authenticated")).await {
          return;
        }
      }

      Request::Auth { username, password } => {
        let conf = config::get();
        if username == conf.username && password == conf.password {
          authenticated = true;
          logger::debug!("AUTH succeeded");
          if conn.send(Response::success("OK")).await {
            return;
          }
        } else {
          logger::debug!("AUTH failed with invalid credentials");
          if conn.send(Response::error("Invalid credentials")).await {
            return;
          }
        }
      }

      Request::Get { key } if authenticated => {
        if let Some(val) = MAP.get().unwrap().read().await.get(&key) {
          if let Some(payload) = handle_map_value(key, val.clone()).await {
            if conn.send(Response::success_payload("OK", payload)).await {
              return;
            }
          } else if conn.send(Response::error("Key expired")).await {
            return;
          } else {
            continue;
          }
        } else if conn.send(Response::error("No such key")).await {
          return;
        }
      }

      Request::Delete { key } if authenticated => {
        wrap_overwrite!(conn, key, write => write.remove(&key));
      }

      Request::Set {
        key,
        value,
        expires_in,
      } if authenticated => {
        let expiry = expires_in.map(|exp| SystemTime::now() + Duration::from_secs(exp));
        wrap_overwrite!(conn, key, write => write.insert(key.clone(), MapValue { value, expiry }));
      }

      _ => {
        // (redundant) authentication & malformed requests will be caught before this point
        if conn.send(Response::error("Authentication required")).await {
          return;
        }
      }
    }
  }

  CURRENT_CONNS.fetch_sub(1, SeqCst);
  logger::info!("Connection closed {}", fmt_conns());
}
