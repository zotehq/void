use crate::primitive_value::PrimitiveValue;
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize)]
pub struct Request {
  pub action: String,
  pub payload: RequestPayload,
}

#[derive(Deserialize)]
pub struct RequestPayload {
  pub key: Option<String>,
  pub value: Option<PrimitiveValue>,
  pub expires_in: Option<u32>,
  pub username: Option<String>,
  pub password: Option<String>,
}

impl Request {
  pub fn from_str(s: &str) -> Result<Request, Box<dyn Error>> {
    Ok(serde_json::from_str::<Request>(s)?)
  }
}
