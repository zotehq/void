use crate::{
  actions, config, logger, request::Request, response::Response, server::log_conns_minus_one,
};
use may::net::TcpStream;
use std::{
  io::{Read, Write},
  str::FromStr,
};

fn write_response(stream: &mut TcpStream, response: &Response) -> bool {
  match stream.write_all(response.to_json().as_bytes()) {
    Ok(_) => false,
    Err(e) => {
      logger::warn!("Connection error (specifically in writing response to client)");
      logger::trace!(target: "conn_handler::write_response", "{}", &e.to_string());
      true
    }
  }
}

fn bad_request(stream: &mut TcpStream) -> bool {
  write_response(stream, &Response::error("Bad request"))
}

pub fn handle_connection(mut stream: TcpStream) {
  let mut authenticated = false;
  let max_body_size = config::get().max_body_size;

  loop {
    let mut request: Vec<u8> = vec![0; max_body_size];

    match stream.read(&mut request) {
      Ok(0) => {
        logger::info!("{}", &log_conns_minus_one("Connection closed"));
        return;
      }
      Ok(amt) => request.shrink_to(amt),
      Err(e) => {
        logger::warn!("Connection error (specifically in reading request from client)");
        logger::trace!(target: "conn_handler::handle_connection", "{}", &e.to_string());
        return;
      }
    };

    let request = match String::from_utf8(request) {
      Ok(s) => s,
      Err(e) => {
        logger::warn!("Malformed request buffer from client");
        logger::trace!(target: "conn_handler::handle_connection", "{}", &e.to_string());

        if bad_request(&mut stream) {
          return;
        }

        continue;
      }
    };

    let request = match Request::from_str(request.trim()) {
      Ok(r) => r,
      Err(e) => {
        logger::warn!("Malformed request string from client");
        logger::trace!(target: "conn_handler::handle_connection", "{}", &e.to_string());

        if bad_request(&mut stream) {
          return;
        }

        continue;
      }
    };

    match request.action.as_str() {
      "AUTH" => {
        if authenticated {
          if write_response(&mut stream, &Response::error("Already authenticated")) {
            return;
          }

          continue;
        }

        let auth_result = actions::auth(&request.payload);
        authenticated = auth_result.is_authenticated;
        if write_response(&mut stream, &auth_result.response) {
          return;
        }
      }

      _ => {
        if write_response(&mut stream, &Response::error("Unknown action")) {
          return;
        }
      }
    }
  }
}
