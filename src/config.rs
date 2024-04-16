use serde::Deserialize;
use std::{error::Error, fs::read_to_string};
use toml;

#[derive(Deserialize)]
pub struct Config {
  pub address: String,
  pub port: u16,
  pub username: String,
  pub password: String,
  pub max_conns: usize,
}

impl Config {
  #[inline]
  pub fn from_file(config_path: &str) -> Result<Config, Box<dyn Error>> {
    Ok(toml::from_str::<Config>(&read_to_string(config_path)?)?)
  }
}
