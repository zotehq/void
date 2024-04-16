use std::{error::Error, net::TcpListener};

pub struct Server {
  listener: TcpListener,
  max_conns: usize,
  current_conns: usize,
}

impl Server {
  pub fn new(host: &str, port: &u16, max_conns: usize) -> Result<Server, Box<dyn Error>> {
    Ok(Server {
      listener: TcpListener::bind(format!("{}:{}", host, port))?,
      max_conns,
      current_conns: 0,
    })
  }

  pub fn listen(self) {
    for stream in self.listener.incoming() {
      let stream = match stream {
        Err(error) => {
          eprintln!("Connection failed: {}", error.to_string());
          continue;
        }
        Ok(stream) => stream,
      };

      println!("Holy dingus it works");
    }
  }
}
