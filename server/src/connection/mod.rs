mod error;
pub use error::*;

use crate::{config::CONFIG, logger::*, TableValue, DATABASE};
use protocol::*;

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, SystemTime};

use scc::hash_map::Entry;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::compression::{read::read_to_bytes, Mode};
use rmp_serde::{from_slice, to_vec};
use bytes::BytesMut;

// CONNECTION STRUCT

pub struct Connection<S: RawStream>(BufReader<S>, BytesMut);

impl<S: RawStream> From<S> for Connection<S> {
  #[inline(always)] // we only call this once, always inline
  fn from(stream: S) -> Self {
    // add an extra 4 bytes for uncompressed length size
    let len = CONFIG.max_message_size + 4;
    Self(BufReader::new(stream), BytesMut::zeroed(len))
  }
}

impl<S: RawStream> Connection<S> {
  #[inline]
  pub async fn send(&mut self, res: Response) -> Result<(), Error> {
    let msg = check!(srv: to_vec(&res))?;
    if msg.len() > CONFIG.max_message_size {
      return Err(ResponseTooLarge.into());
    }
    // PERF: for some reason this is the fastest way to do this
    let bytes = [
      &(msg.len() as u32).to_le_bytes(),
      [0].as_slice(), // TODO: compression
      msg.as_slice(),
    ]
    .concat();
    check!(etc: self.0.write_all(&bytes).await)
  }

  #[inline]
  pub async fn recv(&mut self) -> Result<Request, Error> {
    let len = check!(etc: self.0.read_u32_le().await)? as usize;
    if len > CONFIG.max_message_size {
      return Err(RequestTooLarge.into());
    }

    let comp = check!(etc: self.0.read_u8().await)?;
    if comp == 0 {
      check!(etc: self.0.read_exact(&mut self.1[0..len]).await)?;
      check!(req: from_slice(&self.1[0..len]))
    } else {
      let mode = check!(req: Mode::try_from(comp))?;
      let full_len = check!(etc: self.0.read_u32_le().await)? as usize;
      let uncompressed = check!(req: read_to_bytes(&mut self.0, full_len, mode).await)?;
      check!(req: from_slice(&uncompressed))
    }
  }

  #[inline]
  pub async fn close(&mut self) -> Result<(), Error> {
    self.0.shutdown().await.map_err(|_| Closed.into())
  }
}

pub trait RawStream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {}
impl<S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static> RawStream for S {}

#[macro_export]
#[rustfmt::skip]
macro_rules! check {
  (req: $in:expr) => ( $in.map_err(|e| Error::new(BadRequest.into(), e.into())) );
  (srv: $in:expr) => ( $in.map_err(|e| Error::new(ServerError.into(), e.into())) );
  (etc: $in:expr) => ( $in.map_err(Error::from) );
}

pub use check;

// CONNECTION HANDLER

pub static CURRENT_CONNS: AtomicUsize = AtomicUsize::new(0);

#[inline]
pub fn fmt_conns() -> String {
  let current_conns = CURRENT_CONNS.load(SeqCst);
  let max_conns = CONFIG.max_conns;
  format!("({current_conns} / {max_conns})")
}

#[macro_export]
macro_rules! send {
  ($conn:ident, $msg:expr) => {
    if let Err(e) = $conn.send($msg).await {
      warn!("{e}");
      break;
    }
  };
}

#[inline(always)] // we only call this once, always inline
pub async fn handle_conn<S: RawStream>(conn: &mut Connection<S>) {
  CURRENT_CONNS.fetch_add(1, SeqCst);
  info!("Connection established {}", fmt_conns());

  let mut authenticated = false;

  loop {
    let request = match conn.recv().await {
      Ok(r) => r,
      Err(e) => match e.kind {
        Closed => break,
        Ignored => {
          warn!("{e}");
          continue;
        }
        Continue => continue,
        Io(_) => {
          error!("{e}");
          break;
        }
        ErrorKind::Status(s) => {
          warn!("{e}");
          send!(conn, Response::status(s));
          continue;
        }
      },
    };

    match request {
      Request::Ping => {
        trace!("PING requested");
        send!(conn, Response::ok(Payload::Pong));
      }

      Request::Auth { username, password } => {
        let conf = &*CONFIG;
        if username == conf.username && password == conf.password {
          authenticated = true;
          trace!("AUTH succeeded");
          send!(conn, Response::OK);
        } else {
          trace!("AUTH failed with invalid credentials");
          send!(conn, Response::status(Unauthorized));
        }
      }

      Request::ListTables if authenticated => {
        trace!("LIST TABLE requested");
        let mut tables = Vec::<String>::with_capacity(DATABASE.len());
        let mut entry = DATABASE.first_entry_async().await;
        while let Some(e) = &entry {
          tables.push(e.key().to_owned());
          entry = entry.unwrap().next_async().await;
        }
        send!(conn, Response::ok(Payload::Tables { tables }));
      }

      Request::InsertTable { table, contents } if authenticated => {
        trace!("INSERT TABLE requested | table: {}", table);
        if let Entry::Vacant(entry) = DATABASE.entry_async(table).await {
          let tbl = crate::Table::default();
          if let Some(prot_tbl) = contents {
            // build Table from InsertTable
            let mut entry = prot_tbl.first_entry_async().await;
            while let Some(e) = &entry {
              let key = e.key().to_owned();
              let InsertTableValue { value, lifetime } = e.get().clone();
              let expiry = lifetime.map(|exp| SystemTime::now() + Duration::from_secs(exp));
              let _ = tbl.insert_async(key, TableValue { value, expiry }).await;
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
        trace!("GET TABLE requested | table: {}", table);
        if let Some(table) = DATABASE.get_async(&table).await {
          let table = table.clone();
          let payload = Payload::Table { table };
          send!(conn, Response::ok(payload));
        } else {
          send!(conn, Response::status(NoSuchTable));
        }
      }

      Request::DeleteTable { table } if authenticated => {
        trace!("DELETE TABLE requested | table: {}", table);
        let _ = DATABASE.remove_async(&table).await;
        send!(conn, Response::OK);
      }

      Request::List { table } if authenticated => {
        trace!("LIST requested | table: {}", table);
        if let Some(tbl) = DATABASE.get_async(&table).await {
          let mut keys = Vec::<String>::with_capacity(tbl.len());
          let mut entry = tbl.first_entry_async().await;
          while let Some(e) = &entry {
            keys.push(e.key().to_owned());
            entry = entry.unwrap().next_async().await;
          }
          send!(conn, Response::ok(Payload::Keys { keys }));
        } else {
          send!(conn, Response::status(NoSuchTable));
        }
      }

      Request::Get { table, key } if authenticated => {
        trace!("GET requested | table: {}, key: {}", table, key);
        if let Some(tbl) = DATABASE.get_async(&table).await {
          if let Some(value) = tbl.get_async(&key).await {
            if value.expiry.is_some_and(|st| st <= SystemTime::now()) {
              let _ = value.remove();
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
        trace!("DELETE requested | table: {}, key: {}", table, key);
        if let Some(table) = DATABASE.get_async(&table).await {
          let _ = table.remove_async(&key).await;
        }
        send!(conn, Response::OK);
      }

      Request::Insert { table, key, value } if authenticated => {
        trace!("INSERT requested | table: {}, key: {}", table, key);
        if let Some(table) = DATABASE.get_async(&table).await {
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

      // malformed requests will be caught before this point
      _ => send!(conn, Response::status(Unauthorized)),
    }
  }

  let _ = conn.close().await;
  CURRENT_CONNS.fetch_sub(1, SeqCst);
  info!("Connection closed {}", fmt_conns());
}
