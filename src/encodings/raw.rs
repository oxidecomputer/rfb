use crate::{
    encodings::{Encoding, EncodingType},
    pixel_formats::transform,
    rfb::{ColorSpecification, PixelFormat},
};

/// Section 7.7.1
pub struct RawEncoding {
    pixels: Vec<u8>, // TODO: self-lifetime-bound slice to avoid copy
    width: u16,
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

    // useful for transforming into other encodings
    pub(crate) fn raw_buffer(&self) -> &[u8] {
        &self.pixels
    }
}

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

    fn encode(&self) -> Box<dyn Iterator<Item = u8> + '_> {
        Box::new(self.pixels.iter().copied())
    }

    fn transform(&self, output: &PixelFormat) -> Box<dyn Encoding> {
        let input = &self.pixfmt;

        // XXX: This assumes the pixel formats are both rgb. The server code verifies this
        // before calling.
        assert!(matches!(
            &input.color_spec,
            ColorSpecification::ColorFormat(_)
        ));
        assert!(matches!(
            &output.color_spec,
            ColorSpecification::ColorFormat(_)
        ));

        Box::new(Self {
            pixels: transform(&self.pixels, &input, &output),
            width: self.width,
            height: self.height,
            pixfmt: output.clone(),
        })
    }
}
