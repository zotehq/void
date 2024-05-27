mod tcp;
mod websocket;
pub use tcp::*;
pub use websocket::*;

use crate::{config, logger::*, MapValue, MAP};
use protocol::{request::*, response::*};

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, SystemTime};
use std::{fmt, io::Error as IoError, str::FromStr};

use tokio::io::{AsyncRead, AsyncWrite};

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

// CONNECTION TRAIT IMPLEMENTATION HELPERS

// should be faster than running config::get() all the time
pub static MAX_BODY_SIZE: AtomicUsize = AtomicUsize::new(0);

pub trait RawStream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {}
impl<S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static> RawStream for S {}

#[macro_export]
macro_rules! wrap_malformed_req {
  ($self:ident, $in:expr) => {
    match $in {
      Ok(o) => o,
      Err(e) => {
        let responded = $self.send(Response::status(BadRequest)).await;
        let err = ReceiveError::MalformedRequest(!responded);

        warn!("{}", err);
        trace!(target: "conn_handler::handle_conn", "{}", e);

        return Err(err);
      }
    }
  };
}

pub use wrap_malformed_req;

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

async fn build_payload(key: String, val: MapValue) -> Option<ResponsePayload> {
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
      if let Some(payload) = build_payload($key, val.clone()).await {
        // value existed and hadn't expired, send as payload
        if $conn.send(Response::ok(payload)).await {
          return;
        } else {
          continue;
        }
      }
      // value expired
    }
    // value already didn't exist

    if $conn.send(Response::OK).await {
      return;
    }
  };
}

// CONNECTION HANDLER

pub async fn handle_conn(conn: &mut dyn Connection) {
  CURRENT_CONNS.fetch_add(1, SeqCst);
  info!("Connection established {}", fmt_conns());

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
        debug!("Redundant AUTH attempted");
        if conn.send(Response::status(RedundantAuth)).await {
          return;
        }
      }

      Request::Auth { username, password } => {
        let conf = config::get();
        if username == conf.username && password == conf.password {
          authenticated = true;
          debug!("AUTH succeeded");
          if conn.send(Response::OK).await {
            return;
          }
        } else {
          debug!("AUTH failed with invalid credentials");
          if conn.send(Response::status(BadCredentials)).await {
            return;
          }
        }
      }

      Request::Get { key } if authenticated => {
        if let Some(val) = MAP.get().unwrap().read().await.get(&key) {
          if let Some(payload) = build_payload(key, val.clone()).await {
            if conn.send(Response::ok(payload)).await {
              return;
            }
          } else if conn.send(Response::status(KeyExpired)).await {
            return;
          } else {
            continue;
          }
        } else if conn.send(Response::status(NoSuchKey)).await {
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
        if conn.send(Response::status(AuthRequired)).await {
          return;
        }
      }
    }
  }

  CURRENT_CONNS.fetch_sub(1, SeqCst);
  info!("Connection closed {}", fmt_conns());
}
