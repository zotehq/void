pub mod read;
pub mod write;

use crate::SyncHashSet;

// COMPRESSION MODE

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[rustfmt::skip]
pub enum Mode {
  Lz4     = 0b00000001,
  Zstd    = 0b00000010,
  Snappy  = 0b00000100,
  Brotli  = 0b00001000,
  Deflate = 0b00010000,
  Zlib    = 0b00100000,
  Gzip    = 0b01000000,
  Lzw     = 0b10000000,
}

use Mode::*;

impl Mode {
  #[inline]
  #[rustfmt::skip]
  pub fn tcpset(value: u8) -> SyncHashSet<Self> {
    let mut set = SyncHashSet::default();
    if value & Lz4 as u8 == 1 { set.insert(Lz4); }
    if value & Zstd as u8 == 1 { set.insert(Zstd); }
    if value & Snappy as u8 == 1 { set.insert(Snappy); }
    if value & Brotli as u8 == 1 { set.insert(Brotli); }
    if value & Deflate as u8 == 1 { set.insert(Deflate); }
    if value & Zlib as u8 == 1 { set.insert(Zlib); }
    if value & Gzip as u8 == 1 { set.insert(Gzip); }
    if value & Lzw as u8 == 1 { set.insert(Lzw); }
    set
  }
}

impl TryFrom<u8> for Mode {
  type Error = String;
  #[inline]
  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0b00000001 => Ok(Lz4),
      0b00000010 => Ok(Zstd),
      0b00000100 => Ok(Snappy),
      0b00001000 => Ok(Brotli),
      0b00010000 => Ok(Deflate),
      0b00100000 => Ok(Zlib),
      0b01000000 => Ok(Gzip),
      0b10000000 => Ok(Lzw),
      _ => Err("Too many compression modes selected".to_owned()),
    }
  }
}
