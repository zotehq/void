use crate::AnyResult;

// SYNC READERS
use brotli::Decompressor as BrotliReaderSync;
use flate2::bufread::DeflateDecoder as DeflateReaderSync;
use flate2::bufread::GzDecoder as GzipReaderSync;
use flate2::bufread::ZlibDecoder as ZlibReaderSync;
use lz4_flex::decompress_into as lz4_read_to;
use snap::read::FrameDecoder as SnappyReaderSync;
use weezl::{decode::Decoder as LzwReader, BitOrder::Msb};
use zstd::bulk::decompress_to_buffer as zstd_read_to;
// ASYNC READERS
use async_compression::tokio::bufread::BrotliDecoder as BrotliReaderAsync;
use async_compression::tokio::bufread::DeflateDecoder as DeflateReaderAsync;
use async_compression::tokio::bufread::GzipDecoder as GzipReaderAsync;
use async_compression::tokio::bufread::ZlibDecoder as ZlibReaderAsync;
use async_compression::tokio::bufread::ZstdDecoder as ZstdReaderAsync;
use tokio_snappy::SnappyIO as SnappyAsync;

use bytes::{Bytes, BytesMut};
use std::io::Cursor;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt};
use tokio::task::spawn_blocking;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

// COMPRESSION MODE

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

impl Mode {
  #[inline]
  pub fn from_u8(b: u8) -> Result<Mode, String> {
    match b {
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

use Mode::*;

// COMPRESSION

//pub async fn write_bytes(src: Bytes, mode: Mode) -> AnyResult<Bytes> {}

// DECOMPRESSION

#[inline]
pub async fn read_bytes(src: Bytes, len: usize, mode: Mode) -> AnyResult<Bytes> {
  // Since we already have the full buffer, avoid extraneous async conversion operations.
  spawn_blocking(move || read_bytes_sync(src, len, mode)).await?
}

fn read_bytes_sync(src: Bytes, len: usize, mode: Mode) -> AnyResult<Bytes> {
  use std::io::Read;

  let mut buf = BytesMut::zeroed(len);

  match mode {
    Lz4 => _ = lz4_read_to(&src, &mut buf)?,
    Zstd => _ = zstd_read_to(&src, &mut buf)?,
    Snappy => SnappyReaderSync::new(&*src).read_exact(&mut buf)?,
    Brotli => BrotliReaderSync::new(&*src, len).read_exact(&mut buf)?,
    Deflate => DeflateReaderSync::new(&*src).read_exact(&mut buf)?,
    Zlib => ZlibReaderSync::new(&*src).read_exact(&mut buf)?,
    Gzip => GzipReaderSync::new(&*src).read_exact(&mut buf)?,
    Lzw => _ = LzwReader::new(Msb, 9).decode_bytes(&src, &mut buf).status?,
  }

  Ok(buf.freeze())
}

pub async fn read<S>(src: &mut S, len: usize, full: usize, mode: Mode) -> AnyResult<Bytes>
where
  S: AsyncBufRead + Unpin,
{
  let mut buf = BytesMut::zeroed(full);

  match mode {
    Lz4 => {
      let mut src_buf = vec![0; len];
      src.read_exact(&mut src_buf).await?;
      // Return here because buf gets moved.
      return spawn_blocking(move || -> AnyResult<Bytes> {
        lz4_read_to(&src_buf, &mut buf)?;
        Ok(buf.freeze())
      })
      .await?;
    }
    Zstd => read_helper(&mut buf, ZstdReaderAsync::new(src)).await?,
    Snappy => read_helper(&mut buf, SnappyAsync::new(src)).await?,
    Brotli => read_helper(&mut buf, BrotliReaderAsync::new(src)).await?,
    Deflate => read_helper(&mut buf, DeflateReaderAsync::new(src)).await?,
    Zlib => read_helper(&mut buf, ZlibReaderAsync::new(src)).await?,
    Gzip => read_helper(&mut buf, GzipReaderAsync::new(src)).await?,
    Lzw => {
      // Cost of this should be less than converting to & from a Vec<u8>.
      let buf = Cursor::new(buf.as_mut()).compat_write();
      let mut reader = LzwReader::new(Msb, 9);
      reader.into_async(buf).decode(src.compat()).await.status?;
    }
  }

  Ok(buf.freeze())
}

#[inline(always)]
async fn read_helper<R: AsyncRead + Unpin>(buf: &mut [u8], mut reader: R) -> AnyResult<()> {
  reader.read_exact(buf).await?;
  Ok(())
}
