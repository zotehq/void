use serde::Serialize;
use std::error::Error;

#[derive(Serialize)]
pub struct Response {
  error: bool,
  message: Option<String>,
  payload: Option<String>,
}

impl Response {
  pub fn error(message: &str) -> Self {
    Self {
      error: true,
      message: Some(message.to_string()),
      payload: None,
    }
  }

  pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string(self)?)
  }
}
