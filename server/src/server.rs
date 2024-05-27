use crate::{config, connection::*, logger, wrap_fatal};
use protocol::response::{Response, Status::ConnLimit};
use std::sync::{atomic::Ordering::*, Arc, OnceLock};
use tokio::{join, net::TcpListener};
use tokio_native_tls::{native_tls, TlsAcceptor as AsyncTlsAcceptor};

// IMPLEMENTATION HELPERS

// convert raw stream into either a TCP or WebSocket connection
#[inline(always)]
async fn convert_stream<S: RawStream>(stream: S, protocol: &str) -> Option<Arc<dyn Connection>> {
  if protocol == "TCP" {
    return Some(Arc::new(TcpConnection::from(stream)));
  }

  match WebSocketConnection::convert_stream(stream).await {
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
    let _ = conn.send(Response::status(ConnLimit)).await;
    return;
  }

  handle_conn(conn).await;
}

#[macro_export]
macro_rules! listener {
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
          let stream = match TLS_ACCEPTOR.get().unwrap().accept(stream).await {
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
        tokio::spawn(conn_handoff(conn, CURRENT_CONNS.load(SeqCst) < $max_conns));
      }
    }
  };
}

pub static TLS_ACCEPTOR: OnceLock<AsyncTlsAcceptor> = OnceLock::new();

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
    let cert = wrap_fatal!(std::fs::read(&tls.cert), "Failed to parse TLS cert: {}");
    let key = wrap_fatal!(std::fs::read(&tls.key), "Failed to parse TLS key: {}");
    let identity = wrap_fatal!(
      native_tls::Identity::from_pkcs8(&cert, &key),
      "Failed to create TLS identity: {}"
    );
    let acceptor = wrap_fatal!(
      native_tls::TlsAcceptor::new(identity),
      "Failed to create TLS acceptor: {}"
    );
    let _ = TLS_ACCEPTOR.set(acceptor.into());
  }

  // START LISTENING FOR CONNECTIONS

  MAX_BODY_SIZE.store(conf.max_body_size, Relaxed);

  let mut futures = (None, None);

  if conf.tcp.enabled {
    futures.0 = Some(listener!(
      "TCP",
      &format!("{}:{}", conf.tcp.address, conf.tcp.port),
      conf.tcp.tls,
      conf.max_conns
    ));
  }

  if conf.ws.enabled {
    futures.1 = Some(listener!(
      "WebSocket",
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
