use crate::{Table, TableValue};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
  // COMMON
  #[serde(rename = "OK")]
  Success,
  #[serde(rename = "Too many connections")]
  ConnLimit,
  #[serde(rename = "Malformed request")]
  BadRequest,

  // AUTH
  #[serde(rename = "Authentication required")]
  AuthRequired,
  #[serde(rename = "Invalid credentials")]
  BadCredentials,
  #[serde(rename = "Already authenticated")]
  RedundantAuth,

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Response {
  pub status: Status,
  #[serde(flatten)]
  pub payload: Option<Payload>,
}

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
