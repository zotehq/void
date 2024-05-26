pub mod actions;
pub mod config;
pub mod conn_handler;
pub mod logger;
pub mod primitive_value;
pub mod request;
pub mod response;
pub mod server;

fn main() {
  logger::init();
  server::listen();
}
