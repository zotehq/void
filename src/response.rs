pub struct Response {
	error: bool,
	msg: Option<String>,
	data: Option<Vec<u8>>,
}

impl Response {
	pub fn error(msg: &str) -> Self {
		Self {
			error: true,
			msg: Some(msg.to_string()),
			data: None,
		}
	}

	pub fn ok(msg: Option<&str>, data: Option<Vec<u8>>) -> Self {
		Self {
			error: false,
			msg: match msg {
				None => None,
				Some(v) => Some(v.to_string()),
			},
			data,
		}
	}

	pub fn to_bytes(&self) -> Vec<u8> {
		let mut bytes: Vec<u8> = vec![self.error as u8];

		match &self.msg {
			None => (),
			Some(v) => bytes.extend(v.as_bytes()),
		}

		bytes.push(0);

		match &self.data {
			None => (),
			Some(v) => bytes.extend(v),
		}

		bytes
	}
}
