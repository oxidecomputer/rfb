use crate::{pixel_formats::transform, rfb::ConnectionContext};

use super::{Encoding, EncodingType, RawEncoding, RawEncodingRef};

pub struct ZlibEncodingRef<'a> {
    raw: RawEncodingRef<'a>,
}

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

    fn encode(&self, ctx: &mut ConnectionContext) -> Box<dyn Iterator<Item = u8> + '_> {
        let in_buf = self.raw.raw_buffer();
        let mut out_buf = Vec::with_capacity(in_buf.len());
        ctx.zlib
            .compress_vec(in_buf, &mut out_buf, flate2::FlushCompress::Sync)
            .expect("zlib error");
        Box::new(out_buf.into_iter())
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

    fn encode(&self, ctx: &mut ConnectionContext) -> Box<dyn Iterator<Item = u8> + '_> {
        let in_buf = self.raw.raw_buffer();
        let mut out_buf = Vec::with_capacity(in_buf.len());
        ctx.zlib
            .compress_vec(in_buf, &mut out_buf, flate2::FlushCompress::Sync)
            .expect("zlib error");
        Box::new(out_buf.into_iter())
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
