use crate::may::net::TcpListener;
use crate::{conn_handler::handle_connection, logger, response::Response};
use std::io::Write;

pub struct Server {
  host: String,
  port: u16,
  max_conns: usize,
  current_conns: usize,
}

impl Server {
  pub fn new(host: &str, port: &u16, max_conns: usize) -> Self {
    Self {
      host: host.to_string(),
      port: port.clone(),
      max_conns,
      current_conns: 0,
    }
  }

  pub fn listen(mut self) {
    logger::info("Configuring may");
    let threads = num_cpus::get();
    may::config().set_workers(threads);

    logger::info(&format!("Binding to {}:{}", self.host, self.port));
    may::coroutine::scope(|s| {
      for _ in 0..threads {
        let host = self.host.clone(); // needed so that rustc doesnt complain, do not try to move self.host.clone() inside go!

        go!(s, move || {
          let listener = TcpListener::bind((host, self.port));
          let listener = match listener {
            Ok(l) => l,
            Err(e) => {
              logger::fatal(&format!("Failed to bind: {}", e));
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

            if self.current_conns >= self.max_conns {
              logger::warn("Connection dropped due to max connections limit");
              let _ = stream.write_all(&Response::error("Too many connections").to_bytes());
              continue;
            }

            self.current_conns += 1;
            go!(move || {
              handle_connection(stream);
              self.current_conns -= 1;
            });

            logger::info("Connection accepted");
          }
        });
      }

      logger::info("Listening for connections");
    });
  }
}
