pub mod config;
pub mod connection;
pub mod logger;
pub mod server;

mod util;
pub use util::*;

use protocol::*;

use rmp_serde::{from_read, to_vec};
use scc::HashMap;
use std::fs::File as SyncFile;
use tokio::{
  fs::{metadata, File},
  io::AsyncWriteExt,
  time::{sleep, Duration},
};

// THE DATABASE ITSELF

type Database = HashMap<String, Table, Hasher>;

pub static DATABASE: Global<Database> = Global::new();

async fn save_db() {
  let bytes = wrap_fatal!(to_vec(&*DATABASE), "Failed to serialize database: {}");
  let mut file = wrap_fatal!(File::create("db.void").await, "Failed to open database: {}");
  wrap_fatal!(file.write_all(&bytes).await, "Failed to write database: {}");
}

#[tokio::main]
async fn main() {
  logger::init();

  // load database file

  let mut db_found = false;
  if let Ok(m) = metadata("db.void").await {
    db_found = m.is_file();
  }

  if db_found {
    let file = wrap_fatal!(SyncFile::open("db.void"), "Failed to open database: {}");
    let db = wrap_fatal!(from_read(file), "Failed to parse database: {}");
    DATABASE.set(db);
  } else {
    logger::info!("db.void not found, creating...");
    DATABASE.set(Database::default());
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
