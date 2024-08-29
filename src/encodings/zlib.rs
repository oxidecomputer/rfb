use std::sync::Arc;

use async_trait::async_trait;
use futures::{
    stream::{self, BoxStream},
    StreamExt,
};

use crate::{pixel_formats::transform, rfb::ConnectionContext};

use super::{Encoding, EncodingType, RawEncoding, RawEncodingRef};

pub struct ZlibEncodingRef<'a> {
    raw: RawEncodingRef<'a>,
}

impl<'a> From<RawEncodingRef<'a>> for ZlibEncodingRef<'a> {
    fn from(raw: RawEncodingRef<'a>) -> Self {
        Self { raw }
    }
}

#[async_trait]
impl<'a> Encoding for ZlibEncodingRef<'a> {
    fn get_type(&self) -> EncodingType {
        EncodingType::Zlib
    }

    fn dimensions(&self) -> (u16, u16) {
        self.raw.dimensions()
    }

    fn pixel_format(&self) -> &crate::rfb::PixelFormat {
        self.raw.pixel_format()
    }

    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        let in_buf = self.raw.raw_buffer();
        let mut out_buf = Vec::with_capacity(in_buf.len());
        ctx.zlib.lock().await.unwrap().perform(|zlib| {
            zlib.compress_vec(in_buf, &mut out_buf, flate2::FlushCompress::Sync)
                .expect("zlib error")
        });
        stream::iter(
            (out_buf.len() as u32)
                .to_be_bytes()
                .into_iter()
                .chain(out_buf.into_iter()),
        )
        .boxed()
    }

    fn transform(&self, output: &crate::rfb::PixelFormat) -> Box<dyn Encoding> {
        let (width, height) = self.raw.dimensions();
        Box::new(ZlibEncoding {
            raw: RawEncoding::new(
                transform(self.raw.raw_buffer(), self.pixel_format(), output),
                width,
                height,
                output,
            ),
        })
    }
}

pub struct ZlibEncoding {
    raw: RawEncoding,
}

impl From<RawEncoding> for ZlibEncoding {
    fn from(raw: RawEncoding) -> Self {
        Self { raw }
    }
}

#[async_trait]
impl Encoding for ZlibEncoding {
    fn get_type(&self) -> EncodingType {
        EncodingType::Zlib
    }

    fn dimensions(&self) -> (u16, u16) {
        self.raw.dimensions()
    }

    fn pixel_format(&self) -> &crate::rfb::PixelFormat {
        self.raw.pixel_format()
    }

    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        let in_buf = self.raw.raw_buffer();
        let mut out_buf = Vec::with_capacity(in_buf.len());
        ctx.zlib.lock().await.unwrap().perform(|zlib| {
            zlib.compress_vec(in_buf, &mut out_buf, flate2::FlushCompress::Sync)
                .expect("zlib error")
        });
        stream::iter(
            (out_buf.len() as u32)
                .to_be_bytes()
                .into_iter()
                .chain(out_buf.into_iter()),
        )
        .boxed()
    }

    fn transform(&self, output: &crate::rfb::PixelFormat) -> Box<dyn Encoding> {
        let (width, height) = self.raw.dimensions();
        Box::new(Self {
            raw: RawEncoding::new(
                transform(self.raw.raw_buffer(), self.pixel_format(), output),
                width,
                height,
                output,
            ),
        })
    }
}
