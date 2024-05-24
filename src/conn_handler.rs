use crate::logger;
use crate::server::{log_conns_minus_one, SERVER};
use may::net::TcpStream;
use std::io::{Read, Write};
use std::sync::atomic::Ordering::Relaxed;

pub fn handle_connection(mut stream: TcpStream) {
  loop {
    let mut buffer: Vec<u8> = vec![0; SERVER.max_body_size.load(Relaxed)];

    let bytes_read = match stream.read(&mut buffer) {
      Ok(0) => {
        logger::info(&log_conns_minus_one("Connection closed"));
        return;
      }
      Ok(n) => n,
      Err(e) => {
        logger::warn("Connection error (specifically in reading request from client)");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    };

    match stream.write_all(&buffer[..bytes_read]) {
      Ok(_) => continue,
      Err(e) => {
        logger::warn("Connection error (specifically in writing response to client)");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    }
  }
}
