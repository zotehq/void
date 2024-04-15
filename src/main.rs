pub mod config;

use config::Config;

fn main() {
  let conf = Config::from("config.toml").unwrap();
  println!(
    "address = {}\nport = {}\nusername = {}\npassword = {}",
    conf.address, conf.port, conf.username, conf.password
  );
}
