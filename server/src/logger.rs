pub use log::{debug, error, info, log, trace, warn, Level};

pub fn init() {
  let conf = crate::config::get();
  if std::env::var("RUST_LOG").is_err() {
    if let Some(ref log_level) = conf.log_level {
      std::env::set_var("RUST_LOG", log_level.as_str());
    } else {
      std::env::set_var("RUST_LOG", "info");
    }
  }

  env_logger::init();
}

// based on log crate error! impl
#[macro_export]
macro_rules! fatal {
    // fatal!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // fatal!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (
        log::log!(target: $target, log::Level::Error, $($arg)+);
        std::process::exit(1);
    );

    // fatal!("a {} event", "log")
    ($($arg:tt)+) => (
        log::log!(log::Level::Error, $($arg)+);
        std::process::exit(1);
    )
}

pub use fatal;
