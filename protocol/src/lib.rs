pub mod primitive_value;
pub mod request;
pub mod response;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::Error as _, ser::Error as _, Deserialize, Deserializer, Serializer};

pub(crate) fn from_b64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
  D: Deserializer<'de>,
{
  let s: &str = Deserialize::deserialize(deserializer)?;
  if s.len() > 18 {
    // exit early so we don't waste time decoding
    return Err(D::Error::custom("Ping payload over 125 bytes"));
  }

  let bytes = STANDARD.decode(s).map_err(D::Error::custom)?;
  if bytes.len() > 125 {
    return Err(D::Error::custom("Ping payload over 125 bytes"));
  }
  Ok(bytes)
}

pub(crate) fn to_b64<S>(x: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  if x.len() > 125 {
    return Err(S::Error::custom("Ping payload over 125 bytes"));
  }
  serializer.serialize_str(&STANDARD.encode(x))
}
