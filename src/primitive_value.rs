use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum PrimitiveValue {
  String(String),
  Integer(i64),
  Float(f64),
  Boolean(bool),
}
