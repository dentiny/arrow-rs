// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Contains codec interface and supported codec implementations.
//!
//! See [`Compression`](crate::basic::Compression) enum for all available compression
//! algorithms.
//!
#[cfg_attr(
    feature = "experimental",
    doc = r##"
# Example

```no_run
use parquet::{basic::Compression, compression::{create_codec, CodecOptionsBuilder}};

let codec_options = CodecOptionsBuilder::default()
    .set_backward_compatible_lz4(false)
    .build();
let mut codec = match create_codec(Compression::SNAPPY, &codec_options) {
 Ok(Some(codec)) => codec,
 _ => panic!(),
};

let data = vec![b'p', b'a', b'r', b'q', b'u', b'e', b't'];
let mut compressed = vec![];
codec.compress(&data[..], &mut compressed).unwrap();

let mut output = vec![];
codec.decompress(&compressed[..], &mut output, None).unwrap();

assert_eq!(output, data);
```
"##
)]
use crate::basic::Compression as CodecType;
use crate::errors::{ParquetError, Result};

/// Parquet compression codec interface.
pub trait Codec: Send {
    /// Compresses data stored in slice `input_buf` and appends the compressed result
    /// to `output_buf`.
    ///
    /// Note that you'll need to call `clear()` before reusing the same `output_buf`
    /// across different `compress` calls.
    fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()>;

    /// Decompresses data stored in slice `input_buf` and appends output to `output_buf`.
    ///
    /// If the uncompress_size is provided it will allocate the exact amount of memory.
    /// Otherwise, it will estimate the uncompressed size, allocating an amount of memory
    /// greater or equal to the real uncompress_size.
    ///
    /// Returns the total number of bytes written.
    fn decompress(
        &mut self,
        input_buf: &[u8],
        output_buf: &mut Vec<u8>,
        uncompress_size: Option<usize>,
    ) -> Result<usize>;
}

/// Struct to hold `Codec` creation options.
#[derive(Debug, PartialEq, Eq)]
pub struct CodecOptions {
    /// Whether or not to fallback to other LZ4 older implementations on error in LZ4_HADOOP.
    backward_compatible_lz4: bool,
}

impl Default for CodecOptions {
    fn default() -> Self {
        CodecOptionsBuilder::default().build()
    }
}

pub struct CodecOptionsBuilder {
    /// Whether or not to fallback to other LZ4 older implementations on error in LZ4_HADOOP.
    backward_compatible_lz4: bool,
}

impl Default for CodecOptionsBuilder {
    fn default() -> Self {
        Self {
            backward_compatible_lz4: true,
        }
    }
}

impl CodecOptionsBuilder {
    /// Enable/disable backward compatible LZ4.
    ///
    /// If backward compatible LZ4 is enable, on LZ4_HADOOP error it will fallback
    /// to the older versions LZ4 algorithms. That is LZ4_FRAME, for backward compatibility
    /// with files generated by older versions of this library, and LZ4_RAW, for backward
    /// compatibility with files generated by older versions of parquet-cpp.
    ///
    /// If backward compatible LZ4 is disabled, on LZ4_HADOOP error it will return the error.
    pub fn set_backward_compatible_lz4(mut self, value: bool) -> CodecOptionsBuilder {
        self.backward_compatible_lz4 = value;
        self
    }

    pub fn build(self) -> CodecOptions {
        CodecOptions {
            backward_compatible_lz4: self.backward_compatible_lz4,
        }
    }
}

/// Defines valid compression levels.
pub(crate) trait CompressionLevel<T: std::fmt::Display + std::cmp::PartialOrd> {
    const MINIMUM_LEVEL: T;
    const MAXIMUM_LEVEL: T;

    /// Tests if the provided compression level is valid.
    fn is_valid_level(level: T) -> Result<()> {
        let compression_range = Self::MINIMUM_LEVEL..=Self::MAXIMUM_LEVEL;
        if compression_range.contains(&level) {
            Ok(())
        } else {
            Err(ParquetError::General(format!(
                "valid compression range {}..={} exceeded.",
                compression_range.start(),
                compression_range.end()
            )))
        }
    }
}

/// Given the compression type `codec`, returns a codec used to compress and decompress
/// bytes for the compression type.
/// This returns `None` if the codec type is `UNCOMPRESSED`.
pub fn create_codec(codec: CodecType, _options: &CodecOptions) -> Result<Option<Box<dyn Codec>>> {
    #[allow(unreachable_code, unused_variables)]
    match codec {
        CodecType::BROTLI(level) => {
            #[cfg(any(feature = "brotli", test))]
            return Ok(Some(Box::new(BrotliCodec::new(level))));
            Err(ParquetError::General(
                "Disabled feature at compile time: brotli".into(),
            ))
        }
        CodecType::GZIP(level) => {
            #[cfg(any(feature = "flate2", test))]
            return Ok(Some(Box::new(GZipCodec::new(level))));
            Err(ParquetError::General(
                "Disabled feature at compile time: flate2".into(),
            ))
        }
        CodecType::SNAPPY => {
            #[cfg(any(feature = "snap", test))]
            return Ok(Some(Box::new(SnappyCodec::new())));
            Err(ParquetError::General(
                "Disabled feature at compile time: snap".into(),
            ))
        }
        CodecType::LZ4 => {
            #[cfg(any(feature = "lz4", test))]
            return Ok(Some(Box::new(LZ4HadoopCodec::new(
                _options.backward_compatible_lz4,
            ))));
            Err(ParquetError::General(
                "Disabled feature at compile time: lz4".into(),
            ))
        }
        CodecType::ZSTD(level) => {
            #[cfg(any(feature = "zstd", test))]
            return Ok(Some(Box::new(ZSTDCodec::new(level))));
            Err(ParquetError::General(
                "Disabled feature at compile time: zstd".into(),
            ))
        }
        CodecType::LZ4_RAW => {
            #[cfg(any(feature = "lz4", test))]
            return Ok(Some(Box::new(LZ4RawCodec::new())));
            Err(ParquetError::General(
                "Disabled feature at compile time: lz4".into(),
            ))
        }
        CodecType::UNCOMPRESSED => Ok(None),
        _ => Err(nyi_err!("The codec type {} is not supported yet", codec)),
    }
}

#[cfg(any(feature = "snap", test))]
mod snappy_codec {
    use snap::raw::{decompress_len, max_compress_len, Decoder, Encoder};

    use crate::compression::Codec;
    use crate::errors::Result;

    /// Codec for Snappy compression format.
    pub struct SnappyCodec {
        decoder: Decoder,
        encoder: Encoder,
    }

    impl SnappyCodec {
        /// Creates new Snappy compression codec.
        pub(crate) fn new() -> Self {
            Self {
                decoder: Decoder::new(),
                encoder: Encoder::new(),
            }
        }
    }

    impl Codec for SnappyCodec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let len = match uncompress_size {
                Some(size) => size,
                None => decompress_len(input_buf)?,
            };
            let offset = output_buf.len();
            output_buf.resize(offset + len, 0);
            self.decoder
                .decompress(input_buf, &mut output_buf[offset..])
                .map_err(|e| e.into())
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            let output_buf_len = output_buf.len();
            let required_len = max_compress_len(input_buf.len());
            output_buf.resize(output_buf_len + required_len, 0);
            let n = self
                .encoder
                .compress(input_buf, &mut output_buf[output_buf_len..])?;
            output_buf.truncate(output_buf_len + n);
            Ok(())
        }
    }
}
#[cfg(any(feature = "snap", test))]
pub use snappy_codec::*;

#[cfg(any(feature = "flate2", test))]
mod gzip_codec {

    use std::io::{Read, Write};

    use flate2::{read, write, Compression};

    use crate::compression::Codec;
    use crate::errors::Result;

    use super::GzipLevel;

    /// Codec for GZIP compression algorithm.
    pub struct GZipCodec {
        level: GzipLevel,
    }

    impl GZipCodec {
        /// Creates new GZIP compression codec.
        pub(crate) fn new(level: GzipLevel) -> Self {
            Self { level }
        }
    }

    impl Codec for GZipCodec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            _uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let mut decoder = read::MultiGzDecoder::new(input_buf);
            decoder.read_to_end(output_buf).map_err(|e| e.into())
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            let mut encoder = write::GzEncoder::new(output_buf, Compression::new(self.level.0));
            encoder.write_all(input_buf)?;
            encoder.try_finish().map_err(|e| e.into())
        }
    }
}
#[cfg(any(feature = "flate2", test))]
pub use gzip_codec::*;

/// Represents a valid gzip compression level.
///
/// Defaults to 6.
///
/// * 0: least compression
/// * 9: most compression (that other software can read)
/// * 10: most compression (incompatible with other software, see below)
/// #### WARNING:
/// Level 10 compression can offer smallest file size,
/// but Parquet files created with it will not be readable
/// by other "standard" paquet readers.
///
/// Do **NOT** use level 10 if you need other software to
/// be able to read the files. Read below for details.
///
/// ### IMPORTANT:
/// There's often confusion about the compression levels in `flate2` vs `arrow`
/// as highlighted in issue [#1011](https://github.com/apache/arrow-rs/issues/6282).
///
/// `flate2` supports two compression backends: `miniz_oxide` and `zlib`.
///
/// - `zlib` supports levels from 0 to 9.
/// - `miniz_oxide` supports levels from 0 to 10.
///
/// `arrow` uses `flate` with `rust_backend` feature,
/// which provides `miniz_oxide` as the backend.
/// Therefore 0-10 levels are supported.
///
/// `flate2` documents this behavior properly with
/// [this commit](https://github.com/rust-lang/flate2-rs/pull/430).
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct GzipLevel(u32);

impl Default for GzipLevel {
    fn default() -> Self {
        // The default as of miniz_oxide 0.5.1 is 6 for compression level
        // (miniz_oxide::deflate::CompressionLevel::DefaultLevel)
        Self(6)
    }
}

impl CompressionLevel<u32> for GzipLevel {
    const MINIMUM_LEVEL: u32 = 0;
    const MAXIMUM_LEVEL: u32 = 9;
}

impl GzipLevel {
    /// Attempts to create a gzip compression level.
    ///
    /// Compression levels must be valid (i.e. be acceptable for [`flate2::Compression`]).
    pub fn try_new(level: u32) -> Result<Self> {
        Self::is_valid_level(level).map(|_| Self(level))
    }

    /// Returns the compression level.
    pub fn compression_level(&self) -> u32 {
        self.0
    }
}

#[cfg(any(feature = "brotli", test))]
mod brotli_codec {

    use std::io::{Read, Write};

    use crate::compression::Codec;
    use crate::errors::Result;

    use super::BrotliLevel;

    const BROTLI_DEFAULT_BUFFER_SIZE: usize = 4096;
    const BROTLI_DEFAULT_LG_WINDOW_SIZE: u32 = 22; // recommended between 20-22

    /// Codec for Brotli compression algorithm.
    pub struct BrotliCodec {
        level: BrotliLevel,
    }

    impl BrotliCodec {
        /// Creates new Brotli compression codec.
        pub(crate) fn new(level: BrotliLevel) -> Self {
            Self { level }
        }
    }

    impl Codec for BrotliCodec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let buffer_size = uncompress_size.unwrap_or(BROTLI_DEFAULT_BUFFER_SIZE);
            brotli::Decompressor::new(input_buf, buffer_size)
                .read_to_end(output_buf)
                .map_err(|e| e.into())
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            let mut encoder = brotli::CompressorWriter::new(
                output_buf,
                BROTLI_DEFAULT_BUFFER_SIZE,
                self.level.0,
                BROTLI_DEFAULT_LG_WINDOW_SIZE,
            );
            encoder.write_all(input_buf)?;
            encoder.flush().map_err(|e| e.into())
        }
    }
}
#[cfg(any(feature = "brotli", test))]
pub use brotli_codec::*;

/// Represents a valid brotli compression level.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct BrotliLevel(u32);

impl Default for BrotliLevel {
    fn default() -> Self {
        Self(1)
    }
}

impl CompressionLevel<u32> for BrotliLevel {
    const MINIMUM_LEVEL: u32 = 0;
    const MAXIMUM_LEVEL: u32 = 11;
}

impl BrotliLevel {
    /// Attempts to create a brotli compression level.
    ///
    /// Compression levels must be valid.
    pub fn try_new(level: u32) -> Result<Self> {
        Self::is_valid_level(level).map(|_| Self(level))
    }

    /// Returns the compression level.
    pub fn compression_level(&self) -> u32 {
        self.0
    }
}

#[cfg(any(feature = "lz4", test))]
mod lz4_codec {
    use std::io::{Read, Write};

    use crate::compression::Codec;
    use crate::errors::{ParquetError, Result};

    const LZ4_BUFFER_SIZE: usize = 4096;

    /// Codec for LZ4 compression algorithm.
    pub struct LZ4Codec {}

    impl LZ4Codec {
        /// Creates new LZ4 compression codec.
        pub(crate) fn new() -> Self {
            Self {}
        }
    }

    impl Codec for LZ4Codec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            _uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let mut decoder = lz4_flex::frame::FrameDecoder::new(input_buf);
            let mut buffer: [u8; LZ4_BUFFER_SIZE] = [0; LZ4_BUFFER_SIZE];
            let mut total_len = 0;
            loop {
                let len = decoder.read(&mut buffer)?;
                if len == 0 {
                    break;
                }
                total_len += len;
                output_buf.write_all(&buffer[0..len])?;
            }
            Ok(total_len)
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(output_buf);
            let mut from = 0;
            loop {
                let to = std::cmp::min(from + LZ4_BUFFER_SIZE, input_buf.len());
                encoder.write_all(&input_buf[from..to])?;
                from += LZ4_BUFFER_SIZE;
                if from >= input_buf.len() {
                    break;
                }
            }
            match encoder.finish() {
                Ok(_) => Ok(()),
                Err(e) => Err(ParquetError::External(Box::new(e))),
            }
        }
    }
}

#[cfg(all(feature = "experimental", any(feature = "lz4", test)))]
pub use lz4_codec::*;

#[cfg(any(feature = "zstd", test))]
mod zstd_codec {
    use std::io::{self, Write};

    use crate::compression::{Codec, ZstdLevel};
    use crate::errors::Result;

    /// Codec for Zstandard compression algorithm.
    pub struct ZSTDCodec {
        level: ZstdLevel,
    }

    impl ZSTDCodec {
        /// Creates new Zstandard compression codec.
        pub(crate) fn new(level: ZstdLevel) -> Self {
            Self { level }
        }
    }

    impl Codec for ZSTDCodec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            _uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let mut decoder = zstd::Decoder::new(input_buf)?;
            match io::copy(&mut decoder, output_buf) {
                Ok(n) => Ok(n as usize),
                Err(e) => Err(e.into()),
            }
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            let mut encoder = zstd::Encoder::new(output_buf, self.level.0)?;
            encoder.write_all(input_buf)?;
            match encoder.finish() {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
    }
}
#[cfg(any(feature = "zstd", test))]
pub use zstd_codec::*;

/// Represents a valid zstd compression level.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct ZstdLevel(i32);

impl CompressionLevel<i32> for ZstdLevel {
    // zstd binds to C, and hence zstd::compression_level_range() is not const as this calls the
    // underlying C library.
    const MINIMUM_LEVEL: i32 = 1;
    const MAXIMUM_LEVEL: i32 = 22;
}

impl ZstdLevel {
    /// Attempts to create a zstd compression level from a given compression level.
    ///
    /// Compression levels must be valid (i.e. be acceptable for [`zstd::compression_level_range`]).
    pub fn try_new(level: i32) -> Result<Self> {
        Self::is_valid_level(level).map(|_| Self(level))
    }

    /// Returns the compression level.
    pub fn compression_level(&self) -> i32 {
        self.0
    }
}

impl Default for ZstdLevel {
    fn default() -> Self {
        Self(1)
    }
}

#[cfg(any(feature = "lz4", test))]
mod lz4_raw_codec {
    use crate::compression::Codec;
    use crate::errors::ParquetError;
    use crate::errors::Result;

    /// Codec for LZ4 Raw compression algorithm.
    pub struct LZ4RawCodec {}

    impl LZ4RawCodec {
        /// Creates new LZ4 Raw compression codec.
        pub(crate) fn new() -> Self {
            Self {}
        }
    }

    impl Codec for LZ4RawCodec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let offset = output_buf.len();
            let required_len = match uncompress_size {
                Some(uncompress_size) => uncompress_size,
                None => {
                    return Err(ParquetError::General(
                        "LZ4RawCodec unsupported without uncompress_size".into(),
                    ))
                }
            };
            output_buf.resize(offset + required_len, 0);
            match lz4_flex::block::decompress_into(input_buf, &mut output_buf[offset..]) {
                Ok(n) => {
                    if n != required_len {
                        return Err(ParquetError::General(
                            "LZ4RawCodec uncompress_size is not the expected one".into(),
                        ));
                    }
                    Ok(n)
                }
                Err(e) => Err(ParquetError::External(Box::new(e))),
            }
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            let offset = output_buf.len();
            let required_len = lz4_flex::block::get_maximum_output_size(input_buf.len());
            output_buf.resize(offset + required_len, 0);
            match lz4_flex::block::compress_into(input_buf, &mut output_buf[offset..]) {
                Ok(n) => {
                    output_buf.truncate(offset + n);
                    Ok(())
                }
                Err(e) => Err(ParquetError::External(Box::new(e))),
            }
        }
    }
}
#[cfg(any(feature = "lz4", test))]
pub use lz4_raw_codec::*;

#[cfg(any(feature = "lz4", test))]
mod lz4_hadoop_codec {
    use crate::compression::lz4_codec::LZ4Codec;
    use crate::compression::lz4_raw_codec::LZ4RawCodec;
    use crate::compression::Codec;
    use crate::errors::{ParquetError, Result};
    use std::io;

    /// Size of u32 type.
    const SIZE_U32: usize = std::mem::size_of::<u32>();

    /// Length of the LZ4_HADOOP prefix.
    const PREFIX_LEN: usize = SIZE_U32 * 2;

    /// Codec for LZ4 Hadoop compression algorithm.
    pub struct LZ4HadoopCodec {
        /// Whether or not to fallback to other LZ4 implementations on error.
        /// Fallback is done to be backward compatible with older versions of this
        /// library and older versions parquet-cpp.
        backward_compatible_lz4: bool,
    }

    impl LZ4HadoopCodec {
        /// Creates new LZ4 Hadoop compression codec.
        pub(crate) fn new(backward_compatible_lz4: bool) -> Self {
            Self {
                backward_compatible_lz4,
            }
        }
    }

    /// Try to decompress the buffer as if it was compressed with the Hadoop Lz4Codec.
    /// Adapted from pola-rs [compression.rs:try_decompress_hadoop](https://pola-rs.github.io/polars/src/parquet2/compression.rs.html#225)
    /// Translated from the apache arrow c++ function [TryDecompressHadoop](https://github.com/apache/arrow/blob/bf18e6e4b5bb6180706b1ba0d597a65a4ce5ca48/cpp/src/arrow/util/compression_lz4.cc#L474).
    /// Returns error if decompression failed.
    fn try_decompress_hadoop(input_buf: &[u8], output_buf: &mut [u8]) -> io::Result<usize> {
        // Parquet files written with the Hadoop Lz4Codec use their own framing.
        // The input buffer can contain an arbitrary number of "frames", each
        // with the following structure:
        // - bytes 0..3: big-endian uint32_t representing the frame decompressed size
        // - bytes 4..7: big-endian uint32_t representing the frame compressed size
        // - bytes 8...: frame compressed data
        //
        // The Hadoop Lz4Codec source code can be found here:
        // https://github.com/apache/hadoop/blob/trunk/hadoop-mapreduce-project/hadoop-mapreduce-client/hadoop-mapreduce-client-nativetask/src/main/native/src/codec/Lz4Codec.cc
        let mut input_len = input_buf.len();
        let mut input = input_buf;
        let mut read_bytes = 0;
        let mut output_len = output_buf.len();
        let mut output: &mut [u8] = output_buf;
        while input_len >= PREFIX_LEN {
            let mut bytes = [0; SIZE_U32];
            bytes.copy_from_slice(&input[0..4]);
            let expected_decompressed_size = u32::from_be_bytes(bytes);
            let mut bytes = [0; SIZE_U32];
            bytes.copy_from_slice(&input[4..8]);
            let expected_compressed_size = u32::from_be_bytes(bytes);
            input = &input[PREFIX_LEN..];
            input_len -= PREFIX_LEN;

            if input_len < expected_compressed_size as usize {
                return Err(io::Error::other("Not enough bytes for Hadoop frame"));
            }

            if output_len < expected_decompressed_size as usize {
                return Err(io::Error::other(
                    "Not enough bytes to hold advertised output",
                ));
            }
            let decompressed_size =
                lz4_flex::decompress_into(&input[..expected_compressed_size as usize], output)
                    .map_err(|e| ParquetError::External(Box::new(e)))?;
            if decompressed_size != expected_decompressed_size as usize {
                return Err(io::Error::other("Unexpected decompressed size"));
            }
            input_len -= expected_compressed_size as usize;
            output_len -= expected_decompressed_size as usize;
            read_bytes += expected_decompressed_size as usize;
            if input_len > expected_compressed_size as usize {
                input = &input[expected_compressed_size as usize..];
                output = &mut output[expected_decompressed_size as usize..];
            } else {
                break;
            }
        }
        if input_len == 0 {
            Ok(read_bytes)
        } else {
            Err(io::Error::other("Not all input are consumed"))
        }
    }

    impl Codec for LZ4HadoopCodec {
        fn decompress(
            &mut self,
            input_buf: &[u8],
            output_buf: &mut Vec<u8>,
            uncompress_size: Option<usize>,
        ) -> Result<usize> {
            let output_len = output_buf.len();
            let required_len = match uncompress_size {
                Some(n) => n,
                None => {
                    return Err(ParquetError::General(
                        "LZ4HadoopCodec unsupported without uncompress_size".into(),
                    ))
                }
            };
            output_buf.resize(output_len + required_len, 0);
            match try_decompress_hadoop(input_buf, &mut output_buf[output_len..]) {
                Ok(n) => {
                    if n != required_len {
                        return Err(ParquetError::General(
                            "LZ4HadoopCodec uncompress_size is not the expected one".into(),
                        ));
                    }
                    Ok(n)
                }
                Err(e) if !self.backward_compatible_lz4 => Err(e.into()),
                // Fallback done to be backward compatible with older versions of this
                // library and older versions of parquet-cpp.
                Err(_) => {
                    // Truncate any inserted element before tryingg next algorithm.
                    output_buf.truncate(output_len);
                    match LZ4Codec::new().decompress(input_buf, output_buf, uncompress_size) {
                        Ok(n) => Ok(n),
                        Err(_) => {
                            // Truncate any inserted element before tryingg next algorithm.
                            output_buf.truncate(output_len);
                            LZ4RawCodec::new().decompress(input_buf, output_buf, uncompress_size)
                        }
                    }
                }
            }
        }

        fn compress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<()> {
            // Allocate memory to store the LZ4_HADOOP prefix.
            let offset = output_buf.len();
            output_buf.resize(offset + PREFIX_LEN, 0);

            // Append LZ4_RAW compressed bytes after prefix.
            LZ4RawCodec::new().compress(input_buf, output_buf)?;

            // Prepend decompressed size and compressed size in big endian to be compatible
            // with LZ4_HADOOP.
            let output_buf = &mut output_buf[offset..];
            let compressed_size = output_buf.len() - PREFIX_LEN;
            let compressed_size = compressed_size as u32;
            let uncompressed_size = input_buf.len() as u32;
            output_buf[..SIZE_U32].copy_from_slice(&uncompressed_size.to_be_bytes());
            output_buf[SIZE_U32..PREFIX_LEN].copy_from_slice(&compressed_size.to_be_bytes());

            Ok(())
        }
    }
}
#[cfg(any(feature = "lz4", test))]
pub use lz4_hadoop_codec::*;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::util::test_common::rand_gen::random_bytes;

    fn test_roundtrip(c: CodecType, data: &[u8], uncompress_size: Option<usize>) {
        let codec_options = CodecOptionsBuilder::default()
            .set_backward_compatible_lz4(false)
            .build();
        let mut c1 = create_codec(c, &codec_options).unwrap().unwrap();
        let mut c2 = create_codec(c, &codec_options).unwrap().unwrap();

        // Compress with c1
        let mut compressed = Vec::new();
        let mut decompressed = Vec::new();
        c1.compress(data, &mut compressed)
            .expect("Error when compressing");

        // Decompress with c2
        let decompressed_size = c2
            .decompress(compressed.as_slice(), &mut decompressed, uncompress_size)
            .expect("Error when decompressing");
        assert_eq!(data.len(), decompressed_size);
        assert_eq!(data, decompressed.as_slice());

        decompressed.clear();
        compressed.clear();

        // Compress with c2
        c2.compress(data, &mut compressed)
            .expect("Error when compressing");

        // Decompress with c1
        let decompressed_size = c1
            .decompress(compressed.as_slice(), &mut decompressed, uncompress_size)
            .expect("Error when decompressing");
        assert_eq!(data.len(), decompressed_size);
        assert_eq!(data, decompressed.as_slice());

        decompressed.clear();
        compressed.clear();

        // Test does not trample existing data in output buffers
        let prefix = &[0xDE, 0xAD, 0xBE, 0xEF];
        decompressed.extend_from_slice(prefix);
        compressed.extend_from_slice(prefix);

        c2.compress(data, &mut compressed)
            .expect("Error when compressing");

        assert_eq!(&compressed[..4], prefix);

        let decompressed_size = c2
            .decompress(&compressed[4..], &mut decompressed, uncompress_size)
            .expect("Error when decompressing");

        assert_eq!(data.len(), decompressed_size);
        assert_eq!(data, &decompressed[4..]);
        assert_eq!(&decompressed[..4], prefix);
    }

    fn test_codec_with_size(c: CodecType) {
        let sizes = vec![100, 10000, 100000];
        for size in sizes {
            let data = random_bytes(size);
            test_roundtrip(c, &data, Some(data.len()));
        }
    }

    fn test_codec_without_size(c: CodecType) {
        let sizes = vec![100, 10000, 100000];
        for size in sizes {
            let data = random_bytes(size);
            test_roundtrip(c, &data, None);
        }
    }

    #[test]
    fn test_codec_snappy() {
        test_codec_with_size(CodecType::SNAPPY);
        test_codec_without_size(CodecType::SNAPPY);
    }

    #[test]
    fn test_codec_gzip() {
        for level in GzipLevel::MINIMUM_LEVEL..=GzipLevel::MAXIMUM_LEVEL {
            let level = GzipLevel::try_new(level).unwrap();
            test_codec_with_size(CodecType::GZIP(level));
            test_codec_without_size(CodecType::GZIP(level));
        }
    }

    #[test]
    fn test_codec_brotli() {
        for level in BrotliLevel::MINIMUM_LEVEL..=BrotliLevel::MAXIMUM_LEVEL {
            let level = BrotliLevel::try_new(level).unwrap();
            test_codec_with_size(CodecType::BROTLI(level));
            test_codec_without_size(CodecType::BROTLI(level));
        }
    }

    #[test]
    fn test_codec_lz4() {
        test_codec_with_size(CodecType::LZ4);
    }

    #[test]
    fn test_codec_zstd() {
        for level in ZstdLevel::MINIMUM_LEVEL..=ZstdLevel::MAXIMUM_LEVEL {
            let level = ZstdLevel::try_new(level).unwrap();
            test_codec_with_size(CodecType::ZSTD(level));
            test_codec_without_size(CodecType::ZSTD(level));
        }
    }

    #[test]
    fn test_codec_lz4_raw() {
        test_codec_with_size(CodecType::LZ4_RAW);
    }
}
