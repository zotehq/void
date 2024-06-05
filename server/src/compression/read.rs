use super::*;
use crate::AnyResult as Result;

// SYNC READERS
use brotli::Decompressor as BrotliReaderSync;
use flate2::bufread::DeflateDecoder as DeflateReaderSync;
use flate2::bufread::GzDecoder as GzipReaderSync;
use flate2::bufread::ZlibDecoder as ZlibReaderSync;
use lz4_flex::frame::FrameDecoder as Lz4Reader;
use snap::read::FrameDecoder as SnappyReaderSync;
use weezl::{decode::Decoder as LzwReader, BitOrder::Msb};
use zstd::bulk::Decompressor as ZstdReaderSync;
// ASYNC READERS
use async_compression::tokio::bufread::BrotliDecoder as BrotliReaderAsync;
use async_compression::tokio::bufread::DeflateDecoder as DeflateReaderAsync;
use async_compression::tokio::bufread::GzipDecoder as GzipReaderAsync;
use async_compression::tokio::bufread::ZlibDecoder as ZlibReaderAsync;
use async_compression::tokio::bufread::ZstdDecoder as ZstdReaderAsync;
use tokio_snappy::SnappyIO as SnappyAsync;

use bytes::{Bytes, BytesMut};
use std::io::{Cursor, Read};
use tokio::io::{AsyncBufRead, AsyncReadExt};
use tokio::task::spawn_blocking;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

// BYTES -> BYTES

#[inline]
pub async fn bytes_to_bytes(src: Bytes, len: usize, mode: Mode) -> Result<Bytes> {
  // Since we already have the full byte array, avoid extraneous async conversion operations.
  spawn_blocking(move || bytes_to_bytes_sync(src, len, mode)).await?
}

fn bytes_to_bytes_sync(src: Bytes, len: usize, mode: Mode) -> Result<Bytes> {
  let mut out = BytesMut::zeroed(len);

  match mode {
    Lz4 => Lz4Reader::new(&*src).read_exact(&mut out)?,
    Zstd => _ = ZstdReaderSync::new()?.decompress_to_buffer(&src, &mut *out)?,
    Snappy => SnappyReaderSync::new(&*src).read_exact(&mut out)?,
    Brotli => BrotliReaderSync::new(&*src, len).read_exact(&mut out)?,
    Deflate => DeflateReaderSync::new(&*src).read_exact(&mut out)?,
    Zlib => ZlibReaderSync::new(&*src).read_exact(&mut out)?,
    Gzip => GzipReaderSync::new(&*src).read_exact(&mut out)?,
    Lzw => _ = LzwReader::new(Msb, 9).decode_bytes(&src, &mut out).status?,
  }

  Ok(out.freeze())
}

// READER -> BYTES

pub async fn read_to_bytes<R>(src: &mut R, len: usize, full: usize, mode: Mode) -> Result<Bytes>
where
  R: AsyncBufRead + Unpin,
{
  let mut out = BytesMut::zeroed(full);

  match mode {
    Lz4 => {
      let mut src_out = vec![0; len];
      src.read_exact(&mut src_out).await?;
      // Return here because out gets moved.
      return spawn_blocking(move || -> Result<Bytes> {
        Lz4Reader::new(&*src_out).read_exact(&mut out)?;
        Ok(out.freeze())
      })
      .await?;
    }
    Zstd => _ = ZstdReaderAsync::new(src).read_exact(&mut out).await?,
    Snappy => _ = SnappyAsync::new(src).read_exact(&mut out).await?,
    Brotli => _ = BrotliReaderAsync::new(src).read_exact(&mut out).await?,
    Deflate => _ = DeflateReaderAsync::new(src).read_exact(&mut out).await?,
    Zlib => _ = ZlibReaderAsync::new(src).read_exact(&mut out).await?,
    Gzip => _ = GzipReaderAsync::new(src).read_exact(&mut out).await?,
    Lzw => {
      // Cost of this should be less than converting to and from a Vec<u8>.
      let out = Cursor::new(out.as_mut()).compat_write();
      let mut lzw = LzwReader::new(Msb, 9);
      lzw.into_async(out).decode(src.compat()).await.status?;
    }
  }

  Ok(out.freeze())
}
