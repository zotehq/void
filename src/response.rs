use serde::Serialize;
use std::error::Error;

#[derive(Serialize)]
pub struct Response {
  error: bool,
  message: Option<String>,
  payload: Option<ResponsePayload>,
}

#[derive(Serialize)]
pub struct ResponsePayload {
  key: String,
  value: String,
  #[serde(rename = "type")]
  ktype: String,
  expires_in: Option<i32>,
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

  pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string(self)?)
  }
}
