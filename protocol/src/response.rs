use crate::{Table, TableValue};
use serde::{Deserialize, Serialize};

/// Response status. Also functions as an Error type.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Status {
  Success,
  #[serde(rename = "Too many connections")]
  ConnLimit,
  #[serde(rename = "Malformed request")]
  BadRequest,
  #[serde(rename = "Server error")]
  ServerError,

  // MESSAGE LIMITS
  #[serde(rename = "Request too large")]
  RequestTooLarge,
  #[serde(rename = "Response too large")]
  ResponseTooLarge,

  // AUTH
  Unauthorized,
  #[serde(rename = "Permission denied")]
  PermissionDenied,

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

impl std::fmt::Display for Status {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(serde_variant::to_variant_name(self).unwrap())
  }
}

impl Default for Status {
  #[inline]
  fn default() -> Self {
    Self::Success
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Response {
  pub status: Status,
  #[serde(flatten)]
  pub payload: Option<Payload>,
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
