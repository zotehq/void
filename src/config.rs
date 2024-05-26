use crate::logger;
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, sync::OnceLock};
use toml;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
  pub address: String,
  pub port: u16,
  pub username: String,
  pub password: String,
  pub max_conns: usize,
  pub max_body_size: usize,
  pub log_level: Option<log::Level>,
}

pub fn get() -> &'static Config {
  static CONFIG: OnceLock<Config> = OnceLock::new();
  CONFIG.get_or_init(|| {
    let conf_string = match read_to_string("config.toml") {
      Ok(s) => s,
      Err(e) => {
        logger::fatal!("Failed to load config: {}", e.to_string());
      }
    };

    match toml::from_str::<Config>(&conf_string) {
      Ok(c) => c,
      Err(e) => {
        logger::fatal!("Failed to parse config: {}", e.to_string());
      }
    }
  })
}
