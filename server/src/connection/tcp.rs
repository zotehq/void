use super::*;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct TcpConnection<S: RawStream>(S, Vec<u8>);

impl<S: RawStream> From<S> for TcpConnection<S> {
  #[inline]
  fn from(value: S) -> Self {
    Self(value, vec![0; CONFIG.max_body_size])
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

  #[inline]
  async fn recv(&mut self) -> Result<Request, Error> {
    let req = match self.0.read(&mut self.1).await {
      Ok(0) => return Err(Error::Closed),
      Ok(amt) => &self.1[0..amt],
      Err(e) => return Err(Error::IoError(e)),
    };
    Ok(check_req!(Request::from_bytes(req)))
  }
}
