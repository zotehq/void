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
  pub log_level: Option<String>,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[inline]
pub fn load(config_path: &str) {
  let conf_string = match read_to_string(config_path) {
    Ok(s) => s,
    Err(e) => {
      logger::fatal(&format!("Failed to load config: {}", e.to_string()));
      return;
    }
  };

  let conf = match toml::from_str::<Config>(&conf_string) {
    Ok(c) => c,
    Err(e) => {
      logger::fatal(&format!("Failed to parse config: {}", e.to_string()));
      return;
    }
  };

  let _ = CONFIG.set(conf);
}

pub fn read() -> Config {
  if CONFIG.get().is_none() {
    load("config.toml");
  }

  CONFIG.get().unwrap().clone()
}
