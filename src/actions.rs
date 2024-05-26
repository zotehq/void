use crate::{config, request::RequestPayload, response::Response};

pub struct AuthResult {
  pub is_authenticated: bool,
  pub response: Response,
}

pub fn auth(payload: &RequestPayload) -> AuthResult {
  let conf = config::read();

  if payload.username.is_none() || payload.password.is_none() {
    return AuthResult {
      is_authenticated: false,
      response: Response::error("Missing username or password"),
    };
  }

  let username = payload.username.as_ref().unwrap();
  let password = payload.password.as_ref().unwrap();

  if username != &conf.username || password != &conf.password {
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
