use crate::config;
use protocol::response::Response;

pub struct AuthResult {
  pub is_authenticated: bool,
  pub response: Response,
}

pub fn auth(username: &str, password: &str) -> AuthResult {
  let conf = config::get();

  if username != conf.username || password != conf.password {
    return AuthResult {
      is_authenticated: false,
      response: Response::error("Incorrect username or password"),
    };
  }

  AuthResult {
    is_authenticated: true,
    response: Response::success("Authenticated"),
  }
}
