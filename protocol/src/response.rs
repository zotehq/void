use crate::{from_b64, primitive_value::PrimitiveValue, to_b64};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize)]
pub enum Status {
  // COMMON
  #[serde(rename = "OK")]
  Success,
  #[serde(rename = "Too many connections")]
  ConnLimit,
  #[serde(rename = "Malformed request")]
  BadRequest,

  // PING/PONG
  #[serde(rename = "Pong!")]
  Pong,

  // AUTH
  #[serde(rename = "Authentication required")]
  AuthRequired,
  #[serde(rename = "Invalid credentials")]
  BadCredentials,
  #[serde(rename = "Already authenticated")]
  RedundantAuth,

  // GET
  #[serde(rename = "Key expired")]
  KeyExpired,
  #[serde(rename = "No such key")]
  NoSuchKey,
}

pub use Status::*;

#[derive(Serialize, Deserialize)]
pub struct Response {
  pub status: Status,
  pub payload: Option<Payload>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)] // we deal with tagging in status
pub enum Payload {
  Pong(#[serde(deserialize_with = "from_b64", serialize_with = "to_b64")] Vec<u8>),
  MapData {
    key: String,
    value: PrimitiveValue,
    expires_in: Option<u64>,
  },
}

impl Response {
  // OK is common

  pub const OK: Self = Self {
    status: Status::Success,
    payload: None,
  };

  #[inline]
  pub fn ok(payload: Payload) -> Self {
    Self {
      status: Status::Success,
      payload: Some(payload),
    }
  }

  // non-OK responses

  #[inline]
  pub fn status(status: Status) -> Self {
    Self {
      status,
      payload: None,
    }
  }

  #[inline]
  pub fn payload(status: Status, payload: Payload) -> Self {
    Self {
      status,
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
