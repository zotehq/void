use super::*;
use crate::AnyResult as Result;

// SYNC WRITERS
use brotli::{enc::BrotliEncoderParams, CompressorWriter as BrotliWriterSync};
use flate2::write::DeflateEncoder as DeflateWriterSync;
use flate2::write::GzEncoder as GzipWriterSync;
use flate2::write::ZlibEncoder as ZlibWriterSync;
use flate2::Compression;
use lz4_flex::frame::FrameEncoder as Lz4Writer;
use snap::write::FrameEncoder as SnappyWriterSync;
use weezl::{encode::Encoder as LzwWriter, BitOrder::Msb};
use zstd::bulk::Compressor as ZstdWriterSync;
// ASYNC WRITERS
use async_compression::tokio::write::BrotliEncoder as BrotliWriterAsync;
use async_compression::tokio::write::DeflateEncoder as DeflateWriterAsync;
use async_compression::tokio::write::GzipEncoder as GzipWriterAsync;
use async_compression::tokio::write::ZlibEncoder as ZlibWriterAsync;
use async_compression::tokio::write::ZstdEncoder as ZstdWriterAsync;
use tokio_snappy::SnappyIO as SnappyAsync;

use bytes::Bytes;
use std::io::Write;
use std::mem::transmute;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::task::spawn_blocking;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use tokio_util::io::SyncIoBridge;

// BYTES -> BYTES

const FLATE_FAST: Compression = Compression::fast();

#[inline]
pub async fn bytes_to_bytes(src: Bytes, mode: Mode) -> Result<Bytes> {
  // Since we already have the full byte array, avoid extraneous async conversion operations.
  spawn_blocking(move || bytes_to_bytes_sync(&src, mode)).await?
}

fn bytes_to_bytes_sync(src: &[u8], mode: Mode) -> Result<Bytes> {
  // special paths; avoids allocating our own vec
  match mode {
    Zstd => return Ok(ZstdWriterSync::new(0)?.compress(src)?.into()),
    Lzw => return Ok(LzwWriter::new(Msb, 9).encode(src)?.into()),
    _ => {}
  }

  // Assume output length will be at least half of input length (will grow if needed).
  let mut out = Vec::with_capacity(src.len() / 2);

  match mode {
    Lz4 => Lz4Writer::new(&mut out).write_all(src)?,
    Snappy => SnappyWriterSync::new(&mut out).write_all(src)?,
    Brotli => {
      let params = BrotliEncoderParams::default();
      BrotliWriterSync::with_params(&mut out, 0, &params).write_all(src)?
    }
    Deflate => DeflateWriterSync::new(&mut out, FLATE_FAST).write_all(src)?,
    Zlib => ZlibWriterSync::new(&mut out, FLATE_FAST).write_all(src)?,
    Gzip => GzipWriterSync::new(&mut out, FLATE_FAST).write_all(src)?,
    _ => unreachable!(),
  }

  Ok(out.into())
}

// BYTES -> WRITER

pub async fn bytes_to_writer<W>(src: Bytes, dst: &mut W, mode: Mode) -> Result<()>
where
  W: AsyncWrite + Send + Sync + Unpin + 'static,
{
  match mode {
    Lz4 => {
      // SAFETY: W has static lifetime bound
      let dst = unsafe { transmute::<&mut W, &'static mut W>(dst) };
      let dst = SyncIoBridge::new(dst);
      spawn_blocking(move || Lz4Writer::new(dst).write_all(&src)).await??;
    }
    Zstd => ZstdWriterAsync::new(dst).write_all(&src).await?,
    Snappy => SnappyAsync::new(dst).write_all(&src).await?,
    Brotli => BrotliWriterAsync::new(dst).write_all(&src).await?,
    Deflate => DeflateWriterAsync::new(dst).write_all(&src).await?,
    Zlib => ZlibWriterAsync::new(dst).write_all(&src).await?,
    Gzip => GzipWriterAsync::new(dst).write_all(&src).await?,
    Lzw => {
      let dst = dst.compat_write();
      let mut lzw = LzwWriter::new(Msb, 9);
      lzw.into_async(dst).encode(&*src).await.status?;
    }
  }

  Ok(())
}
