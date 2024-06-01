use crate::{Table, TableValue};
use serde::{Deserialize, Serialize};

// RESPONSE STATUS

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
  // COMMON
  #[serde(rename = "OK")]
  Success,
  #[serde(rename = "Too many connections")]
  ConnLimit,
  #[serde(rename = "Malformed request")]
  BadRequest,
  #[serde(rename = "Server error")]
  ServerError,

  // AUTH
  Unauthorized,
  Forbidden,
  #[serde(rename = "Invalid credentials")]
  BadCredentials,

  // TABLES/KEYS
  #[serde(rename = "Already exists")]
  AlreadyExists,
  #[serde(rename = "No such table")]
  NoSuchTable,
  #[serde(rename = "No such key")]
  NoSuchKey,
  #[serde(rename = "Key expired")]
  KeyExpired,
}

pub use Status::*;

impl Status {
  #[cfg(feature = "http")]
  pub fn to_http(&self) -> http::StatusCode {
    match *self {
      Success => http::StatusCode::OK,
      ConnLimit => http::StatusCode::SERVICE_UNAVAILABLE,
      BadRequest => http::StatusCode::BAD_REQUEST,
      ServerError => http::StatusCode::INTERNAL_SERVER_ERROR,

      Unauthorized => http::StatusCode::UNAUTHORIZED,
      Forbidden => http::StatusCode::FORBIDDEN,
      BadCredentials => http::StatusCode::NOT_ACCEPTABLE,

      AlreadyExists => http::StatusCode::CONFLICT,
      NoSuchTable | NoSuchKey => http::StatusCode::NOT_FOUND,
      KeyExpired => http::StatusCode::GONE,
    }
  }
}

// RESPONSE PAYLOAD

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)] // we flatten this enum and have unique fields
pub enum Payload {
  Pong,
  Tables {
    tables: Vec<String>,
  },
  Keys {
    keys: Vec<String>,
  },
  Table {
    table: Table,
  },
  TableValue {
    table: String,
    key: String,
    value: TableValue,
  },
}

// RESPONSE

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Response {
  pub status: Status,
  #[serde(flatten)]
  pub payload: Option<Payload>,
}

impl Response {
  #[cfg(feature = "http")]
  pub fn to_http(&self) -> http::Result<http::Response<String>> {
    http::Response::builder()
      .status(self.status.to_http())
      .body("".to_owned())
  }

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
