use crate::{conn_handler::handle_connection, logger, response::Response};
use may::{coroutine::scope, go, net::TcpListener};
use std::{
  io::Write,
  sync::{
    atomic::{
      AtomicUsize,
      Ordering::{AcqRel, Acquire, Relaxed},
    },
    OnceLock,
  },
};

#[derive(Default)]
pub struct Server {
  pub addr: OnceLock<String>,
  pub max_conns: AtomicUsize,
  pub current_conns: AtomicUsize,
  pub max_body_size: AtomicUsize,
}

pub static SERVER: Server = Server {
  addr: OnceLock::new(),
  max_conns: AtomicUsize::new(0),
  current_conns: AtomicUsize::new(0),
  max_body_size: AtomicUsize::new(0),
};

pub fn log_conns(msg: &str) -> String {
  let current_conns = SERVER.current_conns.load(Acquire);
  let max_conns = SERVER.max_conns.load(Relaxed);
  format!(
    "{msg} ({current_conns} {} / {max_conns} max)",
    if current_conns == 1 { "conn" } else { "conns" }
  )
}

pub fn log_conns_minus_one(msg: &str) -> String {
  // we need this when a client disconnects because the disconnection
  // wont take effect in the current_conns immediately
  let current_conns = SERVER.current_conns.load(Acquire) - 1;
  let max_conns = SERVER.max_conns.load(Relaxed);
  format!(
    "{msg} ({current_conns} {} / {max_conns} max)",
    if current_conns == 1 { "conn" } else { "conns" }
  )
}

pub fn listen(host: &str, port: u16) {
  let addr = SERVER.addr.get_or_init(|| format!("{host}:{port}"));

  logger::info(&("Binding to ".to_owned() + &addr));

  let threads = num_cpus::get();
  may::config().set_workers(threads);

  scope(|s| {
    for _ in 0..threads {
      go!(s, move || {
        let listener = match TcpListener::bind(addr) {
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

          if SERVER.current_conns.load(Relaxed) >= SERVER.max_conns.load(Acquire) {
            logger::warn(&log_conns("Too many connections"));
            let _ = stream.write_all(
              &Response::error("Too many connections")
                .to_json()
                .unwrap()
                .as_bytes(),
            ); // unwrap is bad, but i am absolutely SURE no error in serialization can happen
            continue;
          }

          SERVER.current_conns.fetch_add(1, AcqRel);
          go!(move || {
            handle_connection(stream);
            SERVER.current_conns.fetch_sub(1, AcqRel);
          });

          logger::info(&log_conns("Connection established"));
        }
      });
    }

    logger::info("Listening for connections");
  });
}
