use crate::Global;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ConnectionConfig {
  pub enabled: bool,
  pub address: String,
  pub port: u16,
  pub tls: bool,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct TlsConfig {
  pub cert: String,
  pub key: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
  pub tcp: ConnectionConfig,
  pub ws: ConnectionConfig,
  pub tls: Option<TlsConfig>,
  pub autosave_interval: u64,
  pub username: String,
  pub password: String,
  pub max_conns: usize,
  pub max_body_size: usize,
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
      autosave_interval: 60,
      username: "admin".to_owned(),
      password: "password".to_owned(),
      max_conns: 10000,
      max_body_size: 8 * 1024 * 1024,
    }
  }
}

pub static CONFIG: Global<Config> = Global::new();
