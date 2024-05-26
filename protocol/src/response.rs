use crate::primitive_value::PrimitiveValue;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct Response {
  error: bool,
  message: Option<String>,
  payload: Option<ResponsePayload>,
}

#[derive(Serialize, Deserialize)]
pub struct ResponsePayload {
  key: String,
  value: PrimitiveValue,
  expires_in: Option<u64>,
}

impl Response {
  pub fn success(message: &str) -> Self {
    Self {
      error: false,
      message: Some(message.to_string()),
      payload: None,
    }
  }

  pub fn success_payload(message: &str, payload: ResponsePayload) -> Self {
    Self {
      error: false,
      message: Some(message.to_string()),
      payload: Some(payload),
    }
  }

  pub fn error(message: &str) -> Self {
    Self {
      error: true,
      message: Some(message.to_string()),
      payload: None,
    }
  }

  pub fn error_payload(message: &str, payload: ResponsePayload) -> Self {
    Self {
      error: true,
      message: Some(message.to_string()),
      payload: Some(payload),
    }
  }
}

impl fmt::Display for Response {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match serde_json::to_string(self) {
      Ok(s) => {
        f.write_str(&s)?;
        Ok(())
      }
      Err(_) => Err(fmt::Error),
    }
  }
}

impl std::str::FromStr for Response {
  type Err = serde_json::Error;
  fn from_str(s: &str) -> Result<Response, Self::Err> {
    serde_json::from_str::<Response>(s)
  }
}
