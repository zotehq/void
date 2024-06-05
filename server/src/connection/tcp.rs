use super::*;

use crate::compression::{read::read_to_bytes, Mode};

use bytes::BytesMut;
use rmp_serde::{from_slice, to_vec};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

pub struct TcpConnection<S: RawStream>(BufReader<S>, BytesMut);

impl<S: RawStream> From<S> for TcpConnection<S> {
  #[inline(always)] // we only call this once, always inline
  fn from(stream: S) -> Self {
    // add an extra 4 bytes for uncompressed length size
    let len = CONFIG.max_message_size + 4;
    Self(BufReader::new(stream), BytesMut::zeroed(len))
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
    let bytes = [
      &(msg.len() as u32).to_le_bytes(),
      [0].as_slice(), // TODO: compression
      msg.as_slice(),
    ]
    .concat();
    check!(etc: self.0.write_all(&bytes).await)
  }

  #[inline]
  async fn recv(&mut self) -> Result<Request, Error> {
    let len = check!(etc: self.0.read_u32_le().await)? as usize;
    if len > CONFIG.max_message_size {
      return Err(RequestTooLarge.into());
    }

    let comp = check!(etc: self.0.read_u8().await)?;
    if comp == 0 {
      check!(etc: self.0.read_exact(&mut self.1[0..len]).await)?;
      check!(req: from_slice(&self.1[0..len]))
    } else {
      let mode = check!(req: Mode::try_from(comp))?;
      let full_len = check!(etc: self.0.read_u32_le().await)? as usize;
      let uncompressed = check!(req: read_to_bytes(&mut self.0, len, full_len, mode).await)?;
      check!(req: from_slice(&uncompressed))
    }
  }

  #[inline]
  async fn close(&mut self) -> Result<(), Error> {
    self.0.shutdown().await.map_err(|_| Closed.into())
  }
}
