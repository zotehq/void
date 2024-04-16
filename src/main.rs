pub mod config;
pub mod server;

use config::Config;
use server::Server;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  let conf = Config::from_file("config.toml")?;
  Server::new(&conf.address, &conf.port, conf.max_conns)?.listen();
  Ok(())
}
