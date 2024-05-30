use crate::{config::CONFIG, connection::*, logger::*, util::Global, wrap_fatal};
use protocol::{Response, Status::ConnLimit};
use std::sync::{atomic::Ordering::SeqCst, Arc};
use tokio::net::TcpListener;
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
      error!("Failed to convert TCP stream to WebSocket: {}", e);
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
    warn!("Too many connections {}", fmt_conns());
    // ignore error since we don't want this connection anyways
    let _ = conn.send(Response::status(ConnLimit)).await;
    return;
  }

  handle_conn(conn).await;
}

#[macro_export]
macro_rules! listener {
  ($protocol:expr, $addr:expr, $tls:expr, $max_conns:expr) => {
    async move {
      let listener = match TcpListener::bind($addr.clone()).await {
        Ok(l) => l,
        Err(e) => {
          fatal!("Failed to bind: {}", e);
        }
      };

      info!("Listening for connections on {} at {}", $protocol, $addr);

      loop {
        let stream = match listener.accept().await {
          Err(e) => {
            error!("Connection failed: {}", e);
            return;
            //continue;
          }
          Ok((s, _)) => s,
        };

        tokio::spawn(async {
          // unfortunately TcpStream and TlsStream are different types
          // we can't overwrite `stream` for TLS and convert it generically
          // convert the two stream types separately and store in conn
          let conn = if $tls {
            let stream = match TLS_ACCEPTOR.accept(stream).await {
              Err(e) => {
                error!("Failed to accept TLS handshake: {}", e);
                return;
                //continue;
              }
              Ok(s) => s,
            };
            convert_stream(stream, $protocol).await
          } else {
            convert_stream(stream, $protocol).await
          };

          // hand control off to connection handler
          // only accept if we're below connection limit
          conn_handoff(conn, CURRENT_CONNS.load(SeqCst) < $max_conns).await;
        });
      }
    }
  };
}

pub static TLS_ACCEPTOR: Global<AsyncTlsAcceptor> = Global::new();

// SET UP LISTENERS

pub async fn listen() {
  let conf = &*CONFIG;

  if !(conf.tcp.enabled || conf.ws.enabled) {
    fatal!("No protocols enabled!");
  }

  // SET UP TLS ACCEPTOR

  if conf.tcp.tls || conf.ws.tls {
    if conf.tls.is_none() {
      fatal!("TLS enabled for one or more protocols, but no TLS config provided!");
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
    TLS_ACCEPTOR.set(acceptor.into());
  }

  // START LISTENING FOR CONNECTIONS

  if conf.tcp.enabled {
    tokio::spawn(listener!(
      "TCP",
      &format!("{}:{}", conf.tcp.address, conf.tcp.port),
      conf.tcp.tls,
      conf.max_conns
    ));
  }

  if conf.ws.enabled {
    tokio::spawn(listener!(
      "WebSocket",
      &format!("{}:{}", conf.ws.address, conf.ws.port),
      conf.ws.tls,
      conf.max_conns
    ));
  }
}
