#[cfg(feature = "scc")]
use scc::HashMap;
#[cfg(not(feature = "scc"))]
use std::collections::HashMap;

use serde::{ser::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::RandomState;
use std::time::{Duration, SystemTime};

// SERDE HELPERS

pub(crate) fn from_unix<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
where
  D: Deserializer<'de>,
{
  if let Some(secs) = Deserialize::deserialize(deserializer)? {
    Ok(Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)))
  } else {
    Ok(None)
  }
}

pub(crate) fn to_unix<S>(x: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  if let Some(st) = x {
    serializer.serialize_some(&Some(
      st.duration_since(SystemTime::UNIX_EPOCH)
        .map_err(S::Error::custom)?
        .as_secs(),
    ))
  } else {
    serializer.serialize_none()
  }
}

// IMPLEMENTATION

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum PrimitiveValue {
  String(String),
  Int(i64),
  Uint(u64),
  Float(f64),
  Boolean(bool),
  Array(Vec<PrimitiveValue>),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct InsertTableValue {
  pub value: PrimitiveValue,
  pub lifetime: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TableValue {
  pub value: PrimitiveValue,
  // calculated from "lifetime" if specified in an InsertTableValue
  // if GET is attempted and current timestamp is past this, remove key and return error
  #[serde(deserialize_with = "from_unix", serialize_with = "to_unix")]
  pub expiry: Option<SystemTime>,
}

pub type InsertTable<S = RandomState> = HashMap<String, InsertTableValue, S>;
pub type Table<S = RandomState> = HashMap<String, TableValue, S>;
