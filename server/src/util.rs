use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::panic::{RefUnwindSafe, UnwindSafe};

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

/// Using OnceLock was annoying me a bit.
/// This struct does everything we need, with less hassle.
/// Safety is entirely dependent on us not being stupid. I think we'll be fine.
pub struct Global<T> {
  value: UnsafeCell<MaybeUninit<T>>,
  _marker: PhantomData<T>,
}

impl<T> Global<T> {
  #[inline]
  pub const fn new() -> Self {
    Self {
      value: UnsafeCell::new(MaybeUninit::uninit()),
      _marker: PhantomData,
    }
  }

  #[inline]
  pub fn set(&self, value: T) {
    // SAFETY: we call this long before any accesses
    unsafe {
      (*self.value.get()).write(value);
    }
  }
}

impl<T> std::ops::Deref for Global<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target {
    // SAFETY: we never deref before initialization
    unsafe { (*self.value.get()).assume_init_ref() }
  }
}

unsafe impl<T: Sync + Send> Sync for Global<T> {}
unsafe impl<T: Send> Send for Global<T> {}
impl<T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for Global<T> {}
impl<T: UnwindSafe> UnwindSafe for Global<T> {}

// HASHER

#[cfg(feature = "gxhash")]
pub type Hasher = gxhash::GxBuildHasher;
#[cfg(not(feature = "gxhash"))]
pub type Hasher = std::collections::hash_map::RandomState;
