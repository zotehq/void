use super::*;

use rmp_serde::{from_slice, to_vec};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

pub struct TcpConnection<S: RawStream>(BufReader<S>, Vec<u8>);

impl<S: RawStream> From<S> for TcpConnection<S> {
  #[inline] // we only call this once, just inline
  fn from(stream: S) -> Self {
    Self(BufReader::new(stream), vec![0; CONFIG.max_message_size])
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  #[inline]
  async fn send(&mut self, res: Response) -> Result<(), Error> {
    let msg = check!(srv: to_vec(&res))?;
    if msg.len() > CONFIG.max_message_size {
      return Err(ResponseTooLarge.into());
    }
    // PERF: for some reason this is the fastest way to do this
    let bytes = [&(msg.len() as u32).to_le_bytes(), msg.as_slice()].concat();
    check!(etc: self.0.write_all(&bytes).await)
  }

  #[inline]
  async fn recv(&mut self) -> Result<Request, Error> {
    let len = check!(etc: self.0.read_u32_le().await)? as usize;
    if len > CONFIG.max_message_size {
      return Err(RequestTooLarge.into());
    }
    check!(etc: self.0.read_exact(&mut self.1[0..len]).await)?;
    check!(req: from_slice(&self.1[0..len]))
  }

  #[inline]
  async fn close(&mut self) -> Result<(), Error> {
    self.0.shutdown().await.map_err(|_| Closed.into())
  }
}
