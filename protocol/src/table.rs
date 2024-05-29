use scc::HashIndex;
use serde::{ser::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::time::{Duration, SystemTime};

// SERDE HELPERS

pub(crate) fn from_unix<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
where
  D: Deserializer<'de>,
{
  let secs: Option<u64> = Deserialize::deserialize(deserializer)?;
  if let Some(s) = secs {
    Ok(Some(SystemTime::UNIX_EPOCH + Duration::from_secs(s)))
  } else {
    Ok(None)
  }
}

pub(crate) fn to_unix<S>(x: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  if let Some(st) = x {
    let secs = st
      .duration_since(SystemTime::UNIX_EPOCH)
      .map_err(S::Error::custom)?
      .as_secs();
    serializer.serialize_some(&Some(secs))
  } else {
    serializer.serialize_none()
  }
}

// IMPLEMENTATION

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum PrimitiveValue {
  String(String),
  Int(i64),
  Uint(u64),
  Float(f64),
  Boolean(bool),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct InsertTableValue {
  pub value: PrimitiveValue,
  pub lifetime: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableValue {
  pub value: PrimitiveValue,
  // calculated from "lifetime" if specified in an InsertTableValue
  // if GET is attempted and current timestamp is past this, remove key and return error
  #[serde(deserialize_with = "from_unix", serialize_with = "to_unix")]
  pub expiry: Option<SystemTime>,
}

#[cfg(feature = "gxhash")]
pub type Hasher = gxhash::GxBuildHasher;
#[cfg(not(feature = "gxhash"))]
pub type Hasher = std::collections::hash_map::RandomState;

pub type InsertTable = HashIndex<String, InsertTableValue, Hasher>;
pub type Table = HashIndex<String, TableValue, Hasher>;
