use super::*;

use simd_json::{Buffers, to_vec, serde::from_slice_with_buffers as from_slice};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct TcpConnection<S: RawStream>(S, Vec<u8>, Buffers);

impl<S: RawStream> From<S> for TcpConnection<S> {
  #[inline]
  fn from(value: S) -> Self {
    Self(value, vec![0; CONFIG.max_body_size], Buffers::new(256))
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  #[inline]
  async fn send(&mut self, res: Response) -> Result<(), Error> {
    match self.0.write_all(&to_vec(&res).unwrap()).await {
      Err(e) => Err(Error::IoError(e)),
      Ok(_) => Ok(()),
    }
  }

  #[inline]
  async fn recv(&mut self) -> Result<Request, Error> {
    let req = match self.0.read(&mut self.1).await {
      Ok(0) => Err(Error::Closed),
      Ok(amt) => Ok(&mut self.1[0..amt]),
      Err(e) => Err(Error::IoError(e)),
    }?;
    Ok(check_req!(from_slice(req, &mut self.2)))
  }
}
