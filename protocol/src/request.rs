use crate::primitive_value::PrimitiveValue;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "UPPERCASE")]
pub enum Request {
  Auth {
    username: String,
    password: String,
  },
  Get {
    key: String,
  },
  Set {
    key: String,
    value: PrimitiveValue,
    expires_in: Option<u64>,
  },
  Delete {
    key: String,
  },
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