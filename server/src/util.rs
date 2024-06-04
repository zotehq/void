use crate::logger::*;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::OnceLock;

pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;
pub type AnyResult<T> = Result<T, BoxError>;

// MACROS

#[macro_export]
macro_rules! wrap_fatal {
  ($in:expr, $fmt:expr) => {
    match $in {
      Ok(o) => o,
      Err(e) => {
        $crate::logger::fatal!($fmt, e);
      }
    }
  };
}

// GLOBAL STATE CONTAINER

/// Small wrapper around OnceLock to impl Deref
pub struct Global<T: Debug>(OnceLock<T>);

impl<T: Debug> Global<T> {
  #[inline]
  pub const fn new() -> Self {
    Self(OnceLock::new())
  }

  #[inline]
  pub fn get(&self) -> Option<&T> {
    self.0.get()
  }

  #[inline]
  pub fn set(&self, value: T) {
    self.0.set(value).unwrap();
  }
}

impl<T: Debug> Deref for Global<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target {
    if let Some(ptr) = self.0.get() {
      ptr
    } else {
      fatal!("Global deref failed");
    }
  }
}

// HASHER

#[cfg(feature = "gxhash")]
pub type Hasher = gxhash::GxBuildHasher;
#[cfg(not(feature = "gxhash"))]
pub type Hasher = std::collections::hash_map::RandomState;
