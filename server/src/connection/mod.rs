mod tcp;
mod websocket;
pub use tcp::*;
pub use websocket::*;

use crate::{config, logger::*, TableValue, DB};
use protocol::*;

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, SystemTime};
use std::{fmt, io::Error as IoError, str::FromStr};

use scc::hash_index::Entry;
use tokio::io::{AsyncRead, AsyncWrite};

// CONNECTION TRAIT

pub enum Error {
  Closed,
  IoError(IoError),
  BadRequest,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Closed => write!(f, "Connection closed")?,
      Self::IoError(e) => write!(f, "I/O error: {}", e)?,
      Self::BadRequest => write!(f, "Malformed request from client")?,
    }
    Ok(())
  }
}

#[async_trait::async_trait]
pub trait Connection: Send + Sync + Unpin {
  async fn send(&mut self, res: Response) -> Result<(), Error>;
  async fn recv(&mut self) -> Result<Request, Error>;
}

// CONNECTION TRAIT IMPLEMENTATION HELPERS

// should be faster than running config::get() all the time
pub static MAX_BODY_SIZE: AtomicUsize = AtomicUsize::new(0);

pub trait RawStream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {}
impl<S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static> RawStream for S {}

#[macro_export]
macro_rules! check_req {
  ($in:expr) => {
    match $in {
      Ok(o) => o,
      Err(_) => return Err(Error::BadRequest),
    }
  };
}

pub use check_req;

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

#[macro_export]
macro_rules! send {
  ($conn:ident, $msg:expr) => {
    if let Err(e) = $conn.send($msg).await {
      warn!("{}", e);
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
      Err(e) => match e {
        Error::Closed => break,
        Error::IoError(_) => {
          warn!("{}", e);
          continue;
        }
        Error::BadRequest => {
          warn!("{}", e);
          send!(conn, Response::status(BadRequest));
          continue;
        }
      },
    };

    match request {
      Request::Ping { payload } => {
        trace!("PING received | payload: {:?}", payload);
        send!(conn, Response::payload(Pong, Payload::Pong { payload }))
      }

      Request::Auth { .. } if authenticated => {
        trace!("Redundant AUTH attempted");
        send!(conn, Response::status(RedundantAuth));
      }

      Request::Auth { username, password } => {
        let conf = config::get();
        if username == conf.username && password == conf.password {
          authenticated = true;
          trace!("AUTH succeeded");
          send!(conn, Response::OK);
        } else {
          trace!("AUTH failed with invalid credentials");
          send!(conn, Response::status(BadCredentials));
        }
      }

      Request::ListTables if authenticated => {
        trace!("LIST TABLE received");
        let db = DB.get().unwrap();
        let mut tables = Vec::<String>::with_capacity(db.len());
        let mut entry = db.first_entry_async().await;
        while let Some(e) = &entry {
          tables.push(e.key().clone());
          entry = entry.unwrap().next_async().await;
        }
        send!(conn, Response::ok(Payload::Tables { tables }));
      }

      Request::InsertTable { table, contents } if authenticated => {
        trace!("INSERT TABLE received | table: {}", table);
        if let Entry::Vacant(entry) = DB.get().unwrap().entry_async(table).await {
          let tbl = crate::Table::default();
          if let Some(prot_tbl) = contents {
            // build Table from InsertTable
            let mut entry = prot_tbl.first_entry_async().await;
            while let Some(e) = &entry {
              let key = e.key().clone();
              let InsertTableValue { value, lifetime } = e.get().clone();
              let expiry = lifetime.map(|exp| SystemTime::now() + Duration::from_secs(exp));
              _ = tbl.insert_async(key, TableValue { value, expiry }).await;
              entry = entry.unwrap().next_async().await;
            }
          }
          entry.insert_entry(tbl);
          send!(conn, Response::OK);
        } else {
          send!(conn, Response::status(AlreadyExists));
        }
      }

      Request::GetTable { table } if authenticated => {
        trace!("GET TABLE received | table: {}", table);
        if let Some(table) = DB.get().unwrap().get_async(&table).await {
          let table = table.clone();
          let payload = Payload::Table { table };
          send!(conn, Response::ok(payload));
        } else {
          send!(conn, Response::status(NoSuchTable));
        }
      }

      Request::DeleteTable { table } if authenticated => {
        trace!("DELETE TABLE received | table: {}", table);
        _ = DB.get().unwrap().remove_async(&table).await;
        send!(conn, Response::OK);
      }

      Request::List { table } if authenticated => {
        trace!("LIST received | table: {}", table);
        let db = DB.get().unwrap();
        if let Some(tbl) = db.get_async(&table).await {
          let mut tables = Vec::<String>::with_capacity(tbl.len());
          let mut entry = tbl.first_entry_async().await;
          while let Some(e) = &entry {
            tables.push(e.key().clone());
            entry = entry.unwrap().next_async().await;
          }
          send!(conn, Response::ok(Payload::Tables { tables }));
        } else {
          send!(conn, Response::status(NoSuchTable));
        }
      }

      Request::Get { table, key } if authenticated => {
        trace!("GET received | table: {}, key: {}", table, key);
        if let Some(tbl) = DB.get().unwrap().get_async(&table).await {
          if let Some(value) = tbl.get_async(&key).await {
            if value.expiry.is_some_and(|st| st <= SystemTime::now()) {
              value.remove_entry();
              send!(conn, Response::status(KeyExpired));
            } else {
              let value = value.clone();
              let payload = Payload::TableValue { table, key, value };
              send!(conn, Response::ok(payload));
            }
          } else {
            send!(conn, Response::status(NoSuchKey));
          }
        } else {
          send!(conn, Response::status(NoSuchTable));
        }
      }

      Request::Delete { table, key } if authenticated => {
        trace!("DELETE received | table: {}, key: {}", table, key);
        if let Some(table) = DB.get().unwrap().get_async(&table).await {
          _ = table.remove_async(&key).await;
        }
        send!(conn, Response::OK);
      }

      Request::Insert { table, key, value } if authenticated => {
        trace!("INSERT received | table: {}, key: {}", table, key);
        if let Some(table) = DB.get().unwrap().get_async(&table).await {
          if let Entry::Vacant(entry) = table.entry_async(key).await {
            let InsertTableValue { value, lifetime } = value;
            let expiry = lifetime.map(|exp| SystemTime::now() + Duration::from_secs(exp));
            entry.insert_entry(TableValue { value, expiry });
          } else {
            send!(conn, Response::status(AlreadyExists));
          }
        } else {
          send!(conn, Response::status(NoSuchTable));
        }
      }

      // (redundant) authentication & malformed requests will be caught before this point
      _ => send!(conn, Response::status(AuthRequired)),
    }
  }

  CURRENT_CONNS.fetch_sub(1, SeqCst);
  info!("Connection closed {}", fmt_conns());
}
