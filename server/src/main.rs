pub mod config;
pub mod connection;
pub mod logger;
pub mod server;

mod util;
pub use util::*;

use config::*;
use logger::*;
use protocol::*;

use rmp_serde::{from_read, to_vec};
use toml::{from_str, to_string_pretty};

use getargs::{Opt, Options};
use scc::HashMap;
use tokio::{
  fs::{metadata, File},
  io::AsyncWriteExt,
  time::{sleep, Duration},
};

use std::fs::{create_dir_all, read_to_string, File as SyncFile};
use std::path::PathBuf;

// THE DATABASE ITSELF

pub type Database = HashMap<String, Table, Hasher>;

pub static DB_PATH: Global<PathBuf> = Global::new();
pub static DATABASE: Global<Database> = Global::new();

async fn save_db() {
  let mut file = wrap_fatal!(File::create(&*DB_PATH).await, "Failed to open database: {}");
  let bytes = wrap_fatal!(to_vec(&*DATABASE), "Failed to serialize database: {}");
  wrap_fatal!(file.write_all(&bytes).await, "Failed to write database: {}");
}

#[tokio::main]
async fn main() {
  // initialize logger (info level by default)

  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "info");
  }

  env_logger::init();

  // load command-line args

  let mut conf_path = dirs::config_dir()
    .expect("Failed to get default user config directory")
    .join("void/config.toml");
  let mut db_path = dirs::data_dir()
    .expect("Failed to get default user data directory")
    .join("void/db.void");

  let args = std::env::args().skip(1).collect::<Vec<_>>();
  let mut opts = Options::new(args.iter().map(String::as_str));

  while let Some(opt) = wrap_fatal!(opts.next_opt(), "Failed to parse arguments: {}") {
    match opt {
      Opt::Short('h') | Opt::Long("help") => {
        eprintln!(
          r"Usage: void [OPTIONS]...
In memory key-value fault tolerant cache built to handle millions of requests.

  -h, --help       display this help and exit
  -c, --config     specify config.toml path (default: {})
  -d, --database   specify db.void path (default: {})",
          conf_path.to_string_lossy(),
          db_path.to_string_lossy()
        );

        return;
      }

      Opt::Short('c') | Opt::Long("config") => {
        conf_path = wrap_fatal!(opts.value(), "Failed to config path: {}").into();
      }

      Opt::Short('d') | Opt::Long("database") => {
        db_path = wrap_fatal!(opts.value(), "Failed to database path: {}").into();
      }

      _ => eprintln!("option: {:?}", opt),
    }
  }

  DB_PATH.set(db_path); // set globally so autosaver can access

  // load config

  if let Ok(m) = metadata(&conf_path).await {
    if m.is_file() {
      let conf_string = wrap_fatal!(read_to_string(conf_path), "Failed to open config: {}");
      let conf = wrap_fatal!(from_str(&conf_string), "Failed to parse config: {}");
      CONFIG.set(conf);
    } else {
      fatal!("Provided config path is not a file!");
    }
  } else {
    info!("Config not found, creating...");
    if let Some(p) = conf_path.parent() {
      wrap_fatal!(create_dir_all(p), "Failed to create config directory: {}");
    }
    CONFIG.set(Config::default());
    let mut file = wrap_fatal!(File::create(conf_path).await, "Failed to open config: {}");
    let bytes = to_string_pretty(&*CONFIG).unwrap().as_bytes().to_vec();
    wrap_fatal!(file.write_all(&bytes).await, "Failed to write config: {}");
  }

  // load database

  if let Ok(m) = metadata(&*DB_PATH).await {
    if m.is_file() {
      let file = wrap_fatal!(SyncFile::open("db.void"), "Failed to open database: {}");
      let db = wrap_fatal!(from_read(file), "Failed to parse database: {}");
      DATABASE.set(db);
    } else {
      fatal!("Provided database path is not a file!");
    }
  } else {
    info!("Database not found, creating...");
    if let Some(p) = DB_PATH.parent() {
      wrap_fatal!(create_dir_all(p), "Failed to create database directory: {}");
    }
    DATABASE.set(Database::default());
    save_db().await;
  }

  // spawn listeners & autosaver

  tokio::spawn(server::listen());
  tokio::spawn(async {
    let duration = Duration::from_secs(CONFIG.autosave_interval);
    loop {
      sleep(duration).await;
      info!("Autosaving...");
      save_db().await;
      info!("Save complete.");
    }
  });

  // handle signals

  match tokio::signal::ctrl_c().await {
    Ok(()) => {
      info!("SIGINT detected, saving...");
      save_db().await;
    }
    Err(e) => error!("Failed to listen for shutdown signal: {}", e),
  }
}
