use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum PrimitiveValue {
  String(String),
  Int(i64),
  Uint(u64),
  Float(f64),
  Boolean(bool),
}
