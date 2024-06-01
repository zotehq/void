use crate::{InsertTable, InsertTableValue};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action")]
#[serde(rename_all = "UPPERCASE")]
pub enum Request {
  Ping,
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
