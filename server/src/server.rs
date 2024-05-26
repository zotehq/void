use crate::{config, connection::*, logger, wrap_fatal};
use protocol::response::Response;
use std::{
  fmt,
  fs::read,
  sync::{
    atomic::{
      AtomicUsize,
      Ordering::{Relaxed, SeqCst},
    },
    Arc, OnceLock,
  },
};
use tokio::{join, net::TcpListener};
use tokio_native_tls::{
  native_tls::{Identity, TlsAcceptor},
  TlsAcceptor as AsyncTlsAcceptor,
};

// IMPLEMENTATION HELPERS

#[derive(PartialEq, Eq)]
enum Protocol {
  Tcp,
  WebSocket,
}

impl fmt::Display for Protocol {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Self::Tcp => write!(f, "TCP"),
      Self::WebSocket => write!(f, "WebSocket"),
    }
  }
}

pub static TLS_ACCEPTOR: OnceLock<Arc<AsyncTlsAcceptor>> = OnceLock::new();
pub static CURRENT_CONNS: AtomicUsize = AtomicUsize::new(0);

pub fn fmt_conns() -> String {
  let current_conns = CURRENT_CONNS.load(SeqCst);
  let max_conns = config::get().max_conns;

  format!(
    "({current_conns} {} / {max_conns} max)",
    if current_conns == 1 { "conn" } else { "conns" }
  )
}

#[inline]
pub fn connect() {
  CURRENT_CONNS.fetch_add(1, SeqCst);
  logger::info!("Connection established {}", fmt_conns());
}

#[inline]
pub fn disconnect() {
  CURRENT_CONNS.fetch_sub(1, SeqCst);
  logger::info!("Connection closed {}", fmt_conns());
}

// SET UP LISTENERS

pub async fn listen() {
  let conf = config::get();

  if !(conf.tcp.enabled || conf.ws.enabled) {
    logger::fatal!("No protocols enabled!");
  }

  // SET UP TLS ACCEPTOR

  if conf.tcp.tls || conf.ws.tls {
    if conf.tls.is_none() {
      logger::fatal!("TLS enabled for one or more protocols, but no TLS config provided!");
    }

    let tls = conf.tls.as_ref().unwrap();
    let cert = wrap_fatal!(read(&tls.cert), "Failed to parse TLS cert: {}");
    let key = wrap_fatal!(read(&tls.key), "Failed to parse TLS key: {}");
    let identity = wrap_fatal!(
      Identity::from_pkcs8(&cert, &key),
      "Failed to create TLS identity: {}"
    );
    let acceptor = wrap_fatal!(
      TlsAcceptor::new(identity),
      "Failed to create TLS acceptor: {}"
    );
    let _ = TLS_ACCEPTOR.set(Arc::new(acceptor.into()));
  }

  // START LISTENING FOR CONNECTIONS

  MAX_BODY_SIZE.store(conf.max_body_size, Relaxed);

  let mut futures = (None, None);

  if conf.tcp.enabled {
    futures.0 = Some(listen_macro!(
      Protocol::Tcp,
      &format!("{}:{}", conf.tcp.address, conf.tcp.port),
      conf.tcp.tls,
      conf.max_conns
    ));
  }

  if conf.ws.enabled {
    futures.1 = Some(listen_macro!(
      Protocol::WebSocket,
      &format!("{}:{}", conf.ws.address, conf.ws.port),
      conf.ws.tls,
      conf.max_conns
    ));
  }

  if conf.ws.enabled {
    if conf.tcp.enabled {
      join!(futures.0.unwrap(), futures.1.unwrap());
    } else {
      futures.1.unwrap().await;
    }
  } else {
    futures.0.unwrap().await;
  }
}

#[macro_export]
macro_rules! listen_macro {
  ($protocol:expr, $addr:expr, $tls:expr, $max_conns:expr) => {
    async {
      let listener = match TcpListener::bind($addr.clone()).await {
        Ok(l) => l,
        Err(e) => {
          logger::fatal!("Failed to bind: {}", e);
        }
      };

      logger::info!("Listening for connections on {} at {}", $protocol, $addr);

      loop {
        let stream = match listener.accept().await {
          Err(e) => {
            logger::error!("Connection failed: {}", e);
            continue;
          }
          Ok((s, _)) => s,
        };

        // unfortunately TcpStream and TlsStream are different types
        // we can't overwrite `stream` for TLS and convert it generically
        // convert the two stream types separately and store in conn
        let conn;
        if $tls {
          let acceptor = TLS_ACCEPTOR.get().unwrap().clone();
          let stream = match acceptor.accept(stream).await {
            Err(e) => {
              logger::error!("Failed to accept TLS handshake: {}", e);
              continue;
            }
            Ok(s) => s,
          };
          conn = convert_stream(stream, $protocol).await;
        } else {
          conn = convert_stream(stream, $protocol).await;
        }

        // hand control off to connection handler
        // only accept if we're below connection limit
        conn_handoff(conn, CURRENT_CONNS.load(SeqCst) < $max_conns).await;
      }
    }
  };
}

use listen_macro;

// convert raw stream into either a TCP or WebSocket connection
#[inline(always)]
async fn convert_stream<S: RawStream>(s: S, p: Protocol) -> Option<Arc<dyn Connection>> {
  if p == Protocol::Tcp {
    return Some(Arc::new(TcpConnection::from(s)));
  }

  match WebSocketConnection::convert_stream(s).await {
    Ok(c) => Some(Arc::new(c)),
    Err(e) => {
      logger::error!("Failed to convert TCP stream to WebSocket: {}", e);
      None
    }
  }
}

// do some small stuff to hand control off to the connection handler
async fn conn_handoff(mut conn: Option<Arc<dyn Connection>>, accept: bool) {
  let conn = match conn.as_mut() {
    Some(c) => Arc::get_mut(c).unwrap(),
    None => return,
  };

  if !accept {
    logger::warn!("Too many connections {}", fmt_conns());
    // ignore error since we don't want this connection anyways
    let _ = conn.send(Response::error("Too many connections")).await;
    return;
  }

  handle_conn(conn).await;
}