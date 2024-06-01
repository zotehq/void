use super::*;

use rmp_serde::{from_slice, to_vec};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

pub struct TcpConnection<S: RawStream>(BufReader<S>, Vec<u8>);

impl<S: RawStream> From<S> for TcpConnection<S> {
  #[inline]
  fn from(stream: S) -> Self {
    Self(BufReader::new(stream), vec![0; CONFIG.max_body_size])
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  #[inline]
  async fn send(&mut self, res: Response) -> Result<(), Error> {
    let msg = check!(srv: to_vec(&res))?;
    let mut bytes = (msg.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(&msg);
    check!(io: self.0.write_all(&bytes).await)
  }

  #[inline]
  async fn recv(&mut self) -> Result<Request, Error> {
    let mut len_buf = [0; 4];
    check!(io: self.0.read_exact(&mut len_buf).await)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > CONFIG.max_body_size {
      return Err(Error::BadRequest("Message length surpassed maximum".into()));
    }
    check!(io: self.0.read_exact(&mut self.1[0..len]).await)?;
    check!(req: from_slice(&self.1[0..len]))
  }

  #[inline]
  async fn close(&mut self) -> Result<(), Error> {
    check!(io: self.0.shutdown().await)
  }
}
