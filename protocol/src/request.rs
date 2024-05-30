use crate::{from_b64, to_b64, InsertTable, InsertTableValue};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "UPPERCASE")]
pub enum Request {
  Ping {
    #[serde(deserialize_with = "from_b64", serialize_with = "to_b64")]
    payload: Vec<u8>,
  },
  Auth {
    username: String,
    password: String,
  },

  // TABLE OPERATIONS
  #[serde(rename = "LIST TABLE")]
  ListTables,
  #[serde(rename = "INSERT TABLE")]
  InsertTable {
    table: String,
    contents: Option<InsertTable>,
  },
  #[serde(rename = "GET TABLE")]
  GetTable {
    table: String,
  },
  #[serde(rename = "DELETE TABLE")]
  DeleteTable {
    table: String,
  },

  // KEY OPERATIONS
  List {
    table: String,
  },
  Get {
    table: String,
    key: String,
  },
  Delete {
    table: String,
    key: String,
  },
  Insert {
    table: String,
    key: String,
    #[serde(flatten)]
    value: InsertTableValue,
  },
}

impl Request {
  #[inline]
  pub fn to_byte_vec(&self) -> Vec<u8> {
    serde_json::to_vec(self).unwrap()
  }

  #[inline]
  pub fn from_bytes(bytes: &[u8]) -> serde_json::Result<Self> {
    serde_json::from_slice(bytes)
  }
}

impl fmt::Display for Request {
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

impl std::str::FromStr for Request {
  type Err = serde_json::Error;
  fn from_str(s: &str) -> Result<Request, Self::Err> {
    serde_json::from_str::<Request>(s)
  }
}
