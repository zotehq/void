pub struct Response<'a> {
  error: bool,
  msg: Option<&'a str>,
  data: Option<&'a [u8]>,
}

impl<'a> Response<'a> {
  pub fn error(msg: &'a str) -> Self {
    Self {
      error: true,
      msg: Some(msg),
      data: None,
    }
  }

  pub fn ok(msg: Option<&'a str>, data: Option<&'a [u8]>) -> Self {
    Self {
      error: false,
      msg,
      data,
    }
  }

  pub fn to_bytes(&self) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![self.error as u8];

    if let Some(msg) = self.msg {
      bytes.extend(msg.as_bytes());
    }

    bytes.push(0);

    if let Some(data) = self.data {
      bytes.extend(data);
    }

    bytes
  }
}
