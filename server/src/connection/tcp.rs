use super::*;

use std::sync::atomic::Ordering::Relaxed;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// store max_body_size here too since its stored in the WebSocketConfig
pub struct TcpConnection<S: RawStream>(S, usize);

impl<S: RawStream> From<S> for TcpConnection<S> {
  fn from(value: S) -> Self {
    Self(value, MAX_BODY_SIZE.load(Relaxed))
  }
}

#[async_trait::async_trait]
impl<S: RawStream> Connection for TcpConnection<S> {
  async fn send(&mut self, res: Response) -> bool {
    if let Err(e) = self.0.write_all(res.to_string().as_bytes()).await {
      warn!("Connection error while writing response to client");
      trace!(target: "conn_handler::write_response", "{}", e);
      true
    } else {
      false
    }
  }

  async fn recv(&mut self) -> Result<Request, ReceiveError> {
    let mut request: Vec<u8> = vec![0; self.1];

    match self.0.read(&mut request).await {
      Ok(0) => return Err(ReceiveError::ConnectionClosed),
      Ok(amt) => request.shrink_to(amt),
      Err(e) => {
        warn!("Connection error while reading request from client");
        trace!(target: "conn_handler::handle_conn", "{}", e);
        return Err(ReceiveError::ConnectionError(e));
      }
    };

    let request = wrap_malformed_req!(self, String::from_utf8(request));
    let request = request.trim_end_matches('\0').trim();
    Ok(wrap_malformed_req!(self, Request::from_str(request)))
  }
}
