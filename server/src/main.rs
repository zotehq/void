pub mod config;
pub mod connection;
pub mod logger;
pub mod server;

mod util;
pub use util::*;

use config::*;
use logger::*;
use protocol::*;

use rmp_serde::{from_slice, to_vec};
use toml::{from_str, to_string_pretty};

use getargs::{Opt, Options};
use scc::HashMap;
use tokio::time::{sleep, Duration};

use std::fs::{canonicalize as resolve, create_dir_all, write, File};
use std::io::{ErrorKind as IoErrorKind, Read};
use std::path::PathBuf;

// USE JEMALLOC

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// THE DATABASE ITSELF

pub type Database = HashMap<String, Table, Hasher>;

pub static DB_PATH: Global<PathBuf> = Global::new();
pub static DATABASE: Global<Database> = Global::new();

fn save() {
  if let Some(db) = DATABASE.get() {
    let bytes = wrap_fatal!(to_vec(db), "Failed to serialize database: {}");
    wrap_fatal!(write(&*DB_PATH, bytes), "Failed to write database: {}");
  }
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
    .expect("Failed to get user config directory")
    .join("void/config.toml");
  let mut db_path = dirs::data_dir()
    .expect("Failed to get user data directory")
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
        conf_path = wrap_fatal!(opts.value(), "Failed to parse config path: {}").into();
        conf_path = wrap_fatal!(resolve(conf_path), "Failed to resolve config path: {}");
      }

      Opt::Short('d') | Opt::Long("database") => {
        db_path = wrap_fatal!(opts.value(), "Failed to parse database path: {}").into();
        db_path = wrap_fatal!(resolve(db_path), "Failed to resolve database path: {}");
      }

      Opt::Short(s) => fatal!("Invalid shorthand argument: {}", s),
      Opt::Long(l) => fatal!("Invalid argument: {}", l),
    }
  }

  DB_PATH.set(db_path); // set globally so autosaver can access

  // load config

  match File::open(&conf_path) {
    Ok(mut file) => {
      if !file.metadata().is_ok_and(|m| m.is_file()) {
        fatal!("{} is not a file!", conf_path.to_string_lossy());
      }
      info!("Loading config from {}...", conf_path.to_string_lossy());
      let str = &mut String::new();
      wrap_fatal!(file.read_to_string(str), "Failed to read config: {}");
      CONFIG.set(wrap_fatal!(from_str(str), "Failed to parse config: {}"));
    }
    Err(e) if e.kind() == IoErrorKind::NotFound => {
      info!("Config not found, creating...");
      if let Some(p) = conf_path.parent() {
        wrap_fatal!(create_dir_all(p), "Failed to create config directory: {}");
      }
      CONFIG.set(Config::default());
      let str = to_string_pretty(&*CONFIG).unwrap();
      wrap_fatal!(write(conf_path, str), "Failed to write config: {}");
    }
    Err(e) => fatal!("Failed to load config: {}", e),
  }

  // load database

  match File::open(&*DB_PATH) {
    Ok(mut file) => {
      if !file.metadata().is_ok_and(|m| m.is_file()) {
        fatal!("{} is not a file!", DB_PATH.to_string_lossy());
      }
      info!("Loading database from {}...", DB_PATH.to_string_lossy());
      let buf = &mut Vec::new();
      wrap_fatal!(file.read_to_end(buf), "Failed to read database: {}");
      DATABASE.set(wrap_fatal!(from_slice(buf), "Failed to parse database: {}"));
    }
    Err(e) if e.kind() == IoErrorKind::NotFound => {
      info!("Database not found, creating...");
      if let Some(p) = DB_PATH.parent() {
        wrap_fatal!(create_dir_all(p), "Failed to create database directory: {}");
      }
      DATABASE.set(Database::default());
      save();
    }
    Err(e) => fatal!("Failed to load database: {}", e),
  }

  // spawn listeners & autosaver

  tokio::spawn(server::listen());
  tokio::spawn(async {
    let duration = Duration::from_secs(CONFIG.autosave_interval);
    loop {
      sleep(duration).await;
      info!("Autosaving...");
      tokio::task::spawn_blocking(save);
      info!("Save complete.");
    }
  });

  // handle signals

  match tokio::signal::ctrl_c().await {
    Ok(()) => {
      info!("SIGINT detected, saving...");
      save();
    }
    Err(e) => error!("Failed to listen for shutdown signal: {}", e),
  }
}
