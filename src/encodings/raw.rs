use crate::{
    encodings::{Encoding, EncodingType},
    pixel_formats::transform,
    rfb::{ColorSpecification, PixelFormat},
};

/// Section 7.7.1
pub struct RawEncoding {
    pixels: Vec<u8>,
}

impl RawEncoding {
    pub fn new(pixels: Vec<u8>) -> Self {
        Self { pixels }
    }
}

impl Encoding for RawEncoding {
    fn get_type(&self) -> EncodingType {
        EncodingType::Raw
    }

    fn encode(&self) -> Vec<u8> {
        self.pixels.clone()
    }

    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding> {
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
        })
    }
}
