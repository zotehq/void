pub mod actions;
pub mod config;
pub mod connection;
pub mod logger;
pub mod server;

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

#[tokio::main]
async fn main() {
  logger::init();
  server::listen().await;
}
