use super::*;

use rmp_serde::{from_slice, to_vec};
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
    let bytes = check!(srv: to_vec(&res))?;
    self.0.write_all(&bytes).await.map_err(Error::IoError)
  }

  #[inline]
  async fn recv(&mut self) -> Result<Request, Error> {
    match self.0.read(&mut self.1).await {
      Ok(0) => Err(Error::Closed),
      Ok(amt) => check!(req: from_slice(&self.1[0..amt])),
      Err(e) => Err(Error::IoError(e)),
    }
  }

  #[inline]
  async fn close(&mut self) -> Result<(), Error> {
    self.0.shutdown().await.map_err(Error::IoError)
  }
}
