use protocol::*;
use std::error::Error as StdError;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use tokio_tungstenite::tungstenite::Error as WsError;

// KIND

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorKind {
  Closed,
  Ignored,
  Continue,
  Io(IoErrorKind),
  Status(Status),
}

pub use ErrorKind::{Closed, Continue, Ignored, Io};

impl From<Status> for ErrorKind {
  #[inline]
  fn from(s: Status) -> Self {
    Self::Status(s)
  }
}

impl std::fmt::Display for ErrorKind {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Closed => write!(f, "Connection closed"),
      Ignored => write!(f, "Ignoring unimplemented connection protocol spec"),
      Continue => write!(f, "Non-fatal error occurred, continuing..."),
      Io(e) => write!(f, "I/O error: {e}"),
      ErrorKind::Status(ServerError) => write!(f, "Internal error (this is a bug!)"),
      ErrorKind::Status(s) => write!(f, "Request failed | status: {s}"),
    }
  }
}

// TYPE

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug)]
pub struct Error {
  pub kind: ErrorKind,
  pub reason: Option<BoxError>,
}

impl Error {
  #[inline]
  pub fn new(kind: ErrorKind, reason: BoxError) -> Self {
    Self {
      kind,
      reason: Some(reason),
    }
  }
}

impl From<IoError> for Error {
  #[inline]
  fn from(e: IoError) -> Self {
    match e.kind() {
      IoErrorKind::ConnectionRefused
      | IoErrorKind::ConnectionReset
      | IoErrorKind::ConnectionAborted
      | IoErrorKind::BrokenPipe
      | IoErrorKind::UnexpectedEof => Closed.into(),
      IoErrorKind::OutOfMemory | IoErrorKind::WriteZero => Io(e.kind()).into(),
      IoErrorKind::InvalidInput => Error::new(ServerError.into(), e.into()),
      IoErrorKind::Interrupted | IoErrorKind::TimedOut => Continue.into(),
      _ => Closed.into(),
    }
  }
}

impl From<WsError> for Error {
  #[inline]
  fn from(e: WsError) -> Self {
    match e {
      WsError::ConnectionClosed | WsError::AlreadyClosed => Closed.into(),
      WsError::Io(e) => e.into(),
      WsError::Capacity(_) => RequestTooLarge.into(),
      WsError::WriteBufferFull(_) => Error::new(ServerError.into(), e.into()),
      _ => Error::new(BadRequest.into(), e.into()),
    }
  }
}

impl From<ErrorKind> for Error {
  #[inline]
  fn from(e: ErrorKind) -> Self {
    Self {
      kind: e,
      reason: None,
    }
  }
}

impl From<Status> for Error {
  #[inline]
  fn from(s: Status) -> Self {
    s.into()
  }
}

impl std::fmt::Display for Error {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.kind)?;
    if let Some(e) = &self.reason {
      write!(f, " | reason: {e}")
    } else {
      Ok(())
    }
  }
}
