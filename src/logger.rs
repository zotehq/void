use crate::{config, datetime::DateTime};
use std::process::exit;

// ugliest loglevel implementation in the world

struct LogLevel {
  trace: bool,
  debug: bool,
  info: bool,
  warn: bool,
  error: bool,
}

impl LogLevel {
  pub fn load() -> Self {
    match config::read()
      .log_level
      .unwrap_or("INFO".to_string())
      .as_str()
    {
      "TRACE" => Self {
        trace: true,
        debug: true,
        info: true,
        warn: true,
        error: true,
      },
      "DEBUG" => Self {
        trace: false,
        debug: true,
        info: true,
        warn: true,
        error: true,
      },
      "INFO" => Self {
        trace: false,
        debug: false,
        info: true,
        warn: true,
        error: true,
      },
      "WARN" => Self {
        trace: false,
        debug: false,
        info: false,
        warn: true,
        error: true,
      },
      "ERROR" => Self {
        trace: false,
        debug: false,
        info: false,
        warn: false,
        error: true,
      },
      "FATAL" => Self {
        trace: false,
        debug: false,
        info: false,
        warn: false,
        error: false,
      },
      _ => Self {
        trace: false,
        debug: false,
        info: true,
        warn: true,
        error: true,
      },
    }
  }
}

pub fn trace(msg: &str, caller: &str) {
  if !LogLevel::load().trace {
    return;
  }

  let datetime = DateTime::new();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;30mTRACE\x1b[0m from \x1b[1;37m{}\x1b[0m]: {}",
    datetime.date, datetime.time, caller, msg
  );
}

pub fn debug(msg: &str) {
  if !LogLevel::load().debug {
    return;
  }

  let datetime = DateTime::new();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;30mDEBUG\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn info(msg: &str) {
  if !LogLevel::load().info {
    return;
  }

  let datetime = DateTime::new();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[34mINFO\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn warn(msg: &str) {
  if !LogLevel::load().warn {
    return;
  }

  let datetime = DateTime::new();

  eprintln!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;33mWARN\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn error(msg: &str) {
  if !LogLevel::load().error {
    return;
  }

  let datetime = DateTime::new();

  eprintln!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[31mERROR\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn fatal(msg: &str) {
  let datetime = DateTime::new();

  eprintln!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;31mFATAL\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );

  exit(1);
}
