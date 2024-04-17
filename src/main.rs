pub mod arg_parser;
pub mod config;
pub mod conn_handler;
pub mod datetime;
pub mod logger;
pub mod request;
pub mod response;
pub mod server;

use config::Config;
use server::Server;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  let conf = Config::from_file("config.toml")?;
  Server::new(&conf.address, &conf.port, conf.max_conns)?.listen();
  Ok(())
}
