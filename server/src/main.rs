pub mod config;
pub mod connection;
pub mod logger;
pub mod server;

use protocol::*;
use rmp_serde::{from_read, to_vec};
use scc::HashIndex;
use std::fs::{metadata, File};
use std::io::Write;
use std::sync::OnceLock;
use tokio::time::{sleep, Duration};

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

// THE DB ITSELF

type Database = HashIndex<String, Table, Hasher>;

pub static DB: OnceLock<Database> = OnceLock::new();

async fn save_db() {
  let db = DB.get().unwrap();
  let bytes = wrap_fatal!(to_vec(db), "Failed to serialize database: {}");
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
    let db = wrap_fatal!(from_read(file), "Failed to parse db.void: {}");
    DB.set(db).unwrap();
  } else {
    logger::info!("db.void not found, creating...");
    DB.set(Database::default()).unwrap();
    save_db().await;
  }

  // spawn listeners & autosaver

  tokio::spawn(server::listen());
  tokio::spawn(async {
    let duration = Duration::from_secs(config::get().autosave_interval);
    loop {
      sleep(duration).await;
      logger::info!("Autosaving...");
      save_db().await;
      logger::info!("Save complete.");
    }
  });

  // handle signals

  match tokio::signal::ctrl_c().await {
    Ok(()) => {
      logger::info!("SIGINT detected, saving...");
      save_db().await;
    }
    Err(e) => logger::error!("Failed to listen for shutdown signal: {}", e),
  }
}
