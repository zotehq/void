pub mod config;
pub mod connection;
pub mod logger;
pub mod server;

#[cfg(feature = "gxhash")]
use gxhash::GxBuildHasher;
use protocol::primitive_value::PrimitiveValue;
use rmp_serde::{from_read, to_vec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{metadata, File};
use std::io::Write;
use std::sync::OnceLock;
use std::time::SystemTime;
use tokio::{
  signal,
  sync::RwLock,
  time::{sleep, Duration},
};

// HELPERS

#[macro_export]
macro_rules! wrap_fatal {
  ($in:expr, $fmt:expr) => {
    match $in {
      Ok(o) => o,
      Err(e) => {
        $crate::logger::fatal!($fmt, e);
      }
    }
  };
}

// THE MAP ITSELF

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MapValue {
  value: PrimitiveValue,
  // calculated from "expires_in" if specified when SET
  // if GET is attempted and current timestamp is past this, remove key and return error
  // if not, calculate difference and set "expires_in" in response
  expiry: Option<SystemTime>,
}

#[cfg(feature = "gxhash")]
type Map = HashMap<String, MapValue, GxBuildHasher>;
#[cfg(not(feature = "gxhash"))]
type Map = HashMap<String, MapValue>;

pub static MAP: OnceLock<RwLock<Map>> = OnceLock::new();

async fn save_map() {
  let map = MAP.get().unwrap().read().await;
  let bytes = wrap_fatal!(to_vec(&*map), "Failed to serialize database: {}");
  drop(map); // drop ASAP we don't need this lock anymore
  let mut file = wrap_fatal!(File::create("db.void"), "Failed to create db.void: {}");
  wrap_fatal!(file.write_all(&bytes), "Failed to write to db.void: {}");
}

#[tokio::main]
async fn main() {
  logger::init();

  // load database file

  let mut db_found = false;
  if let Ok(m) = metadata("db.void") {
    db_found = m.is_file();
  }

  if db_found {
    let file = wrap_fatal!(File::open("db.void"), "Failed to load db.void: {}");
    let map = wrap_fatal!(from_read(file), "Failed to parse db.void: {}");
    MAP.set(RwLock::new(map)).unwrap();
  } else {
    logger::info!("db.void not found, creating...");
    MAP.set(RwLock::new(Map::default())).unwrap();
    save_map().await;
  }

  // spawn listeners & autosaver

  tokio::spawn(server::listen());
  tokio::spawn(async {
    let duration = Duration::from_secs(config::get().autosave_interval);
    loop {
      sleep(duration).await;
      logger::info!("Autosaving...");
      save_map().await;
      logger::info!("Save complete.");
    }
  });

  // handle signals

  match signal::ctrl_c().await {
    Ok(()) => {
      logger::info!("SIGINT detected, saving...");
      save_map().await;
    }
    Err(e) => logger::error!("Failed to listen for shutdown signal: {}", e),
  }
}
