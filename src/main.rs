pub mod arg_parser;
pub mod config;
pub mod conn_handler;
pub mod datetime;
pub mod logger;
pub mod request;
pub mod response;
pub mod server;

use config::Config;
use server::SERVER;
use std::{error::Error, sync::atomic::Ordering::Relaxed};

fn main() -> Result<(), Box<dyn Error>> {
  let conf = Config::from_file("config.toml")?;

  SERVER.max_conns.store(conf.max_conns, Relaxed);
  server::listen(&conf.address, conf.port)?;

  Ok(())
}
