use crate::{logger, wrap_fatal};
use serde::{Deserialize, Serialize};
use std::{
  fs::{metadata, read_to_string, File},
  io::{Result, Write},
  sync::OnceLock,
};
use toml::{from_str, to_string_pretty};

#[derive(Deserialize, Serialize)]
pub struct ConnectionConfig {
  pub enabled: bool,
  pub address: String,
  pub port: u16,
  pub tls: bool,
}

#[derive(Deserialize, Serialize, Default)]
pub struct TlsConfig {
  pub cert: String,
  pub key: String,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
  pub tcp: ConnectionConfig,
  pub ws: ConnectionConfig,
  pub tls: Option<TlsConfig>,
  pub username: String,
  pub password: String,
  pub max_conns: usize,
  pub max_body_size: usize,
  pub log_level: Option<log::Level>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      tcp: ConnectionConfig {
        enabled: true,
        address: "0.0.0.0".to_owned(),
        port: 6380,
        tls: false,
      },
      ws: ConnectionConfig {
        enabled: false,
        address: "0.0.0.0".to_owned(),
        port: 6381,
        tls: false,
      },
      tls: Some(TlsConfig::default()),
      username: "admin".to_owned(),
      password: "password".to_owned(),
      max_conns: 10000,
      max_body_size: 8 * 1024 * 1024,
      log_level: None,
    }
  }
}

fn create() -> Result<()> {
  let mut file = File::create("config.toml")?;
  file.write_all(to_string_pretty(&Config::default()).unwrap().as_bytes())?;
  Ok(())
}

pub fn get() -> &'static Config {
  static CONFIG: OnceLock<Config> = OnceLock::new();
  CONFIG.get_or_init(|| {
    let mut config_found = false;
    if let Ok(m) = metadata("config.toml") {
      config_found = m.is_file();
    }

    if !config_found {
      logger::info!("No config found! Creating one...");
      wrap_fatal!(create(), "Failed to create config: {}");
      return Config::default();
    }

    let conf_string = wrap_fatal!(read_to_string("config.toml"), "Failed to load config: {}");
    wrap_fatal!(from_str(&conf_string), "Failed to parse config: {}")
  })
}
