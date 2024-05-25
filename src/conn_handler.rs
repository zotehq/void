use crate::{
  logger,
  request::Request,
  server::{log_conns_minus_one, SERVER},
};
use may::net::TcpStream;
use std::{io::Read, sync::atomic::Ordering::Relaxed};

pub fn handle_connection(mut stream: TcpStream) {
  loop {
    let mut request: Vec<u8> = vec![0; SERVER.max_body_size.load(Relaxed)];

    match stream.read(&mut request) {
      Ok(0) => {
        logger::info(&log_conns_minus_one("Connection closed"));
        return;
      }
      Ok(_) => (),
      Err(e) => {
        logger::warn("Connection error (specifically in reading request from client)");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    };

    let request = match String::from_utf8(request.iter().copied().take_while(|&c| c != 0).collect())
    {
      Ok(s) => s,
      Err(e) => {
        logger::warn("Malformed request buffer from client");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    };

    let request = match Request::from_str(request.trim()) {
      Ok(r) => r,
      Err(e) => {
        logger::warn("Malformed request string from client");
        logger::trace(&e.to_string(), "conn_handler::handle_connection");
        return;
      }
    };

    println!("request.action: {}", request.action);
  }
}
