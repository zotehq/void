use crate::datetime::DateTime;
use std::process::exit;

pub fn trace(msg: &str, caller: &str) {
  let datetime = DateTime::new();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;30mTRACE\x1b[0m from \x1b[1;37m{}\x1b[0m]: {}",
    datetime.date, datetime.time, caller, msg
  );
}

pub fn debug(msg: &str) {
  let datetime = DateTime::new();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;30mDEBUG\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn info(msg: &str) {
  let datetime = DateTime::new();

  println!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[34mINFO\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn warn(msg: &str) {
  let datetime = DateTime::new();

  eprintln!(
    "[\x1b[1;37m{} {}\x1b[0m] [\x1b[1;33mWARN\x1b[0m]: {}",
    datetime.date, datetime.time, msg
  );
}

pub fn error(msg: &str) {
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
