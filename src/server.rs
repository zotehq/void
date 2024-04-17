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
}

pub static SERVER: Server = Server {
  addr: OnceLock::new(),
  max_conns: AtomicUsize::new(0),
  current_conns: AtomicUsize::new(0),
};

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
              logger::info(&format!("Connection failed: {}", error.to_string()));
              continue;
            }
            Ok(stream) => stream,
          };

          if SERVER.current_conns.load(Relaxed) >= SERVER.max_conns.load(Acquire) {
            logger::warn("Connection dropped due to max connections limit");
            let _ = stream.write_all(&Response::error("Too many connections").to_bytes());
            continue;
          }

          SERVER.current_conns.fetch_add(1, AcqRel);
          go!(move || {
            let _ = handle_connection(stream);
            SERVER.current_conns.fetch_sub(1, AcqRel);
          });

          logger::info("Connection accepted");
        }
      });
    }

    logger::info("Listening for connections");
  });
}
