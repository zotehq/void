use crate::logger;
use std::{
  io::{BufReader, BufWriter, Read, Write},
  net::TcpStream,
};

// to-do: fix reads, it reads 0 bytes
pub fn handle_connection(stream: TcpStream) {
  let mut reader = BufReader::new(&stream);
  let mut writer = BufWriter::new(&stream);

  loop {
    let mut buf: Vec<u8> = vec![0; 1024];

    let bytes_read = match reader.read(&mut buf) {
      Ok(0) => {
        logger::info("Connection closed");
        return;
      }
      Ok(n) => n,
      Err(e) => {
        logger::warn("Connection error");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    };

    match writer.write_all(&buf[..bytes_read]) {
      Ok(_) => {
        continue;
      }
      Err(e) => {
        logger::warn("Connection error");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    }
  }
}
