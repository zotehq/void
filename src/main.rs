pub mod arg_parser;
pub mod config;
pub mod conn_handler;
pub mod datetime;
pub mod logger;
pub mod primitive_value;
pub mod request;
pub mod response;
pub mod server;

use config::Config;
use server::SERVER;
use std::sync::atomic::Ordering::Relaxed;

fn main() {
  let conf = match Config::from_file("config.toml") {
    Ok(c) => c,
    Err(e) => {
      logger::fatal(&format!("Failed to load config: {}", e.to_string()));
      return;
    }
  };

  SERVER.max_conns.store(conf.max_conns, Relaxed);
  SERVER.max_body_size.store(conf.max_body_size, Relaxed);
  server::listen(&conf.address, conf.port);
}
