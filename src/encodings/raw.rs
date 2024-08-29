use std::sync::Arc;

use async_trait::async_trait;
use futures::{
    stream::{self, BoxStream},
    StreamExt,
};

use crate::{
    encodings::{Encoding, EncodingType},
    pixel_formats::transform,
    rfb::{ConnectionContext, PixelFormat},
};

/// Section 7.7.1
pub struct RawEncoding {
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: u16,
    height: u16,
    pixfmt: PixelFormat,
}

impl RawEncoding {
    pub fn new(pixels: Vec<u8>, width: u16, height: u16, pixfmt: &PixelFormat) -> Self {
        Self {
            pixels,
            width,
            height,
            pixfmt: pixfmt.clone(),
        }
    }

    pub(crate) fn raw_buffer(&self) -> &[u8] {
        &self.pixels
    }
}

impl<'a> From<&RawEncodingRef<'a>> for RawEncoding {
    fn from(raw_ref: &RawEncodingRef<'a>) -> Self {
        RawEncoding {
            pixels: raw_ref.pixels.to_vec(),
            width: raw_ref.width,
            height: raw_ref.height,
            pixfmt: raw_ref.pixfmt.to_owned(),
        }
    }
}

impl<'a> From<&'a RawEncoding> for RawEncodingRef<'a> {
    fn from(raw_owned: &'a RawEncoding) -> Self {
        Self {
            pixels: &raw_owned.pixels,
            width: raw_owned.width,
            height: raw_owned.height,
            pixfmt: raw_owned.pixfmt.to_owned(),
        }
    }
}

#[async_trait]
impl Encoding for RawEncoding {
    fn get_type(&self) -> EncodingType {
        EncodingType::Raw
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn pixel_format(&self) -> &PixelFormat {
        &self.pixfmt
    }

    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter(self.pixels.iter().copied()).boxed()
    }

    fn transform(&self, output: &PixelFormat) -> Box<dyn Encoding> {
        Box::new(RawEncoding {
            pixels: transform(&self.pixels, &self.pixfmt, &output),
            width: self.width,
            height: self.height,
            pixfmt: output.to_owned(),
        })
    }
}

pub struct RawEncodingRef<'a> {
    pixels: &'a [u8],
    width: u16,
    height: u16,
    pixfmt: PixelFormat,
}

impl<'a> RawEncodingRef<'a> {
    pub fn new(pixels: &'a [u8], width: u16, height: u16, pixfmt: &PixelFormat) -> Self {
        Self {
            pixels,
            width,
            height,
            pixfmt: pixfmt.clone(),
        }
    }

    // useful for transforming into other encodings
    pub(crate) fn raw_buffer(&self) -> &[u8] {
        &self.pixels
    }
}

#[async_trait]
impl<'a> Encoding for RawEncodingRef<'a> {
    fn get_type(&self) -> EncodingType {
        EncodingType::Raw
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn pixel_format(&self) -> &PixelFormat {
        &self.pixfmt
    }

    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter(self.pixels.iter().copied()).boxed()
    }

    fn transform(&self, output: &PixelFormat) -> Box<dyn Encoding> {
        Box::new(RawEncoding {
            pixels: transform(&self.pixels, &self.pixfmt, &output),
            width: self.width,
            height: self.height,
            pixfmt: output.to_owned(),
        })
    }
}
