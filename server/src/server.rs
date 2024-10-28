use crate::{config::CONFIG, connection::*, logger::*, util::Global, wrap_fatal};
use protocol::{Response, Status::ConnLimit};
use std::sync::atomic::Ordering::SeqCst;
use tokio::net::TcpListener;
use tokio_native_tls::{native_tls, TlsAcceptor as AsyncTlsAcceptor};

// IMPLEMENTATION HELPERS

// do some small stuff to hand control off to the connection handler
async fn conn_handoff<S: RawStream>(conn: &mut Connection<S>, accept: bool) {
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
  ($addr:expr, $tls:expr, $max_conns:expr) => {
    async move {
      let listener = match TcpListener::bind($addr).await {
        Ok(l) => l,
        Err(e) => {
          fatal!("Failed to bind: {}", e);
        }
      };

      info!("Listening for connections at {}", $addr);

      loop {
        let stream = match listener.accept().await {
          Err(e) => {
            error!("Connection failed: {}", e);
            continue;
          }
          Ok((s, _)) => s,
        };

        tokio::spawn(async {
          // unfortunately TcpStream and TlsStream are different types
          // we can't overwrite `stream` for TLS and convert it generically
          // convert the two stream types separately and store in conn
          if $tls {
            let stream = match TLS_ACCEPTOR.accept(stream).await {
              Err(e) => {
                error!("Failed to accept TLS handshake: {}", e);
                return;
                //continue;
              }
              Ok(s) => s,
            };
            conn_handoff(&mut Connection::from(stream), CURRENT_CONNS.load(SeqCst) < $max_conns).await;
          } else {
            conn_handoff(&mut Connection::from(stream), CURRENT_CONNS.load(SeqCst) < $max_conns).await;
          }
        });
      }
    }
  };
}

pub static TLS_ACCEPTOR: Global<AsyncTlsAcceptor> = Global::new();

// SET UP LISTENERS

pub async fn listen() {
  let conf = &*CONFIG;

  if !conf.conn.enabled {
    fatal!("No protocols enabled!");
  }

  // SET UP TLS ACCEPTOR

  if conf.conn.tls {
    if conf.tls.is_none() {
      fatal!("TLS is enabled but no TLS config was provided!");
    }

    let tls = conf.tls.as_ref().unwrap();
    let cert = wrap_fatal!(std::fs::read(&tls.cert), "Failed to parse TLS cert: {}");
    let key = wrap_fatal!(std::fs::read(&tls.key), "Failed to parse TLS key: {}");
    let identity = wrap_fatal!(
      native_tls::Identity::from_pkcs8(&cert, &key),
      "Failed to build TLS identity: {}"
    );
    let acceptor = wrap_fatal!(
      native_tls::TlsAcceptor::new(identity),
      "Failed to build TLS acceptor: {}"
    );
    TLS_ACCEPTOR.set(acceptor.into());
  }

  // START LISTENING FOR CONNECTIONS

    tokio::spawn(listener!(
      &format!("{}:{}", conf.conn.address, conf.conn.port),
      conf.conn.tls,
      conf.max_conns
    ));
}
