use crate::{config, conn_handler::handle_connection, logger, response::Response};
use may::{coroutine::scope, go, net::TcpListener};
use std::{
  io::Write,
  sync::atomic::{
    AtomicUsize,
    Ordering::{Acquire, Relaxed},
  },
};

pub static CURRENT_CONNS: AtomicUsize = AtomicUsize::new(0);

pub fn log_conns(msg: &str) -> String {
  let current_conns = CURRENT_CONNS.load(Relaxed);
  let max_conns = config::read().max_conns;

  format!(
    "{msg} ({current_conns} {} / {max_conns} max)",
    if current_conns == 1 { "conn" } else { "conns" }
  )
}

pub fn log_conns_minus_one(msg: &str) -> String {
  // we need this when a client disconnects because the disconnection
  // wont take effect in the current_conns immediately
  let current_conns = CURRENT_CONNS.load(Relaxed);
  let max_conns = config::read().max_conns;

  format!(
    "{msg} ({current_conns} {} / {max_conns} max)",
    if current_conns == 1 { "conn" } else { "conns" }
  )
}

pub fn listen() {
  let conf = config::read();
  let addr = &format!("{}:{}", conf.address, conf.port);

  logger::info(&("Binding to ".to_string() + &addr));

  let threads = num_cpus::get();
  may::config().set_workers(threads);

  scope(|s| {
    for _ in 0..threads {
      go!(s, move || {
        let listener = match TcpListener::bind(addr.clone()) {
          Ok(l) => l,
          Err(e) => {
            logger::fatal(&format!("Failed to bind: {}", e.to_string()));
            return;
          }
        };

        for stream in listener.incoming() {
          let mut stream = match stream {
            Err(error) => {
              logger::error(&format!("Connection failed: {}", error.to_string()));
              continue;
            }
            Ok(stream) => stream,
          };

          if CURRENT_CONNS.load(Relaxed) >= conf.max_conns {
            logger::warn(&log_conns("Too many connections"));
            let _ = stream.write_all(
              &Response::error("Too many connections")
                .to_json()
                .unwrap()
                .as_bytes(),
            ); // unwrap is bad, but i am absolutely SURE no error in serialization can happen
            continue;
          }

          CURRENT_CONNS.fetch_add(1, Acquire);
          go!(move || {
            handle_connection(stream);
            CURRENT_CONNS.fetch_sub(1, Acquire);
          });

          logger::info(&log_conns("Connection established"));
        }
      });
    }

    logger::info("Listening for connections");
  });
}
