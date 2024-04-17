use crate::logger;
use may::{io::SplitIo, net::TcpStream};
use std::io::{BufReader, BufWriter, Read, Result, Write};

// to-do: fix reads, it reads 0 bytes
pub fn handle_connection(stream: TcpStream) -> Result<()> {
  let split = stream.split()?;
  let mut reader = BufReader::new(split.0);
  let mut writer = BufWriter::new(split.1);

  loop {
    let mut buf: Vec<u8> = vec![0; 1024];

    let bytes_read = match reader.read(&mut buf) {
      Ok(0) => {
        logger::info("Connection closed");
        return Ok(());
      }
      Ok(n) => n,
      Err(e) => {
        logger::warn("Connection error");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return Err(e);
      }
    };

    match writer.write_all(&buf[..bytes_read]) {
      Ok(_) => {
        continue;
      }
      Err(e) => {
        logger::warn("Connection error");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return Err(e);
      }
    }
  }
}
