use super::*;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

// store max_body_size here too since its stored in the WebSocketConfig
pub struct TcpConnection<S: RawStream>(S, usize);

impl<S: RawStream> From<S> for TcpConnection<S> {
  fn from(value: S) -> Self {
    Self(value, CONFIG.max_body_size)
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  #[inline]
  async fn send(&mut self, res: Response) -> Result<(), Error> {
    match self.0.write_all(res.to_string().as_bytes()).await {
      Err(e) => Err(Error::IoError(e)),
      Ok(_) => Ok(()),
    }
  }

  async fn recv(&mut self) -> Result<Request, Error> {
    let mut req: Vec<u8> = vec![0; self.1];

    match self.0.read(&mut req).await {
      Ok(0) => return Err(Error::Closed),
      Ok(amt) => req.truncate(amt),
      Err(e) => {
        return Err(Error::IoError(e));
      }
    };

    let req = check_req!(std::str::from_utf8(&req));
    Ok(check_req!(Request::from_str(req)))
  }
}
