use crate::datetime::DateTime;
use std::process::exit;

#[derive(PartialEq)]
enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
  Fatal,
}

impl LogLevel {
  fn color_code(&self) -> &'static str {
    match self {
      LogLevel::Trace => "\x1b[1;30",
      LogLevel::Debug => "\x1b[1;30",
      LogLevel::Info => "\x1b[34",
      LogLevel::Warn => "\x1b[1;33",
      LogLevel::Error => "\x1b[31",
      LogLevel::Fatal => "\x1b[1;31",
    }
  }

  fn tag(&self) -> &'static str {
    match self {
      LogLevel::Trace => "TRACE",
      LogLevel::Debug => "DEBUG",
      LogLevel::Info => "INFO",
      LogLevel::Warn => "WARN",
      LogLevel::Error => "ERROR",
      LogLevel::Fatal => "FATAL",
    }
  }
}

fn current_datetime() -> DateTime {
  DateTime::new()
}

fn log(level: LogLevel, msg: &str, caller: &str) {
  let datetime = current_datetime();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[{}m{}\x1b[0m\x1b[1;37m{}\x1b[0m]: {}",
    datetime.date,
    datetime.time,
    level.color_code(),
    level.tag(),
    caller,
    msg
  );

  if level == LogLevel::Fatal {
    exit(1);
  }
}

pub fn trace(msg: &str, caller: &str) {
  log(LogLevel::Trace, msg, caller);
}

pub fn debug(msg: &str) {
  log(LogLevel::Debug, msg, "");
}

pub fn info(msg: &str) {
  log(LogLevel::Info, msg, "");
}

pub fn warn(msg: &str) {
  log(LogLevel::Warn, msg, "");
}

pub fn error(msg: &str) {
  log(LogLevel::Error, msg, "");
}

pub fn fatal(msg: &str) {
  log(LogLevel::Fatal, msg, "");
}
