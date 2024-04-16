use crate::{conn_handler::handle_connection, logger, response::Response};
use std::{error::Error, io::Write, net::TcpListener, thread};

pub struct Server {
  listener: TcpListener,
  max_conns: usize,
  current_conns: usize,
}

impl Server {
  pub fn new(host: &str, port: &u16, max_conns: usize) -> Result<Server, Box<dyn Error>> {
    logger::info(&format!("Binding to {}:{}", host, port));

    Ok(Server {
      listener: TcpListener::bind(format!("{}:{}", host, port))?,
      max_conns,
      current_conns: 0,
    })
  }

  pub fn listen(mut self) {
    logger::info("Listening for connections");

    for stream in self.listener.incoming() {
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
      thread::spawn(move || {
        handle_connection(stream);
        self.current_conns -= 1;
      });

      logger::info("Connection accepted");
    }
  }
}
