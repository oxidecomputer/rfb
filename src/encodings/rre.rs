use crate::{
    encodings::{Encoding, EncodingType, Pixel},
    pixel_formats::transform,
    rfb::{PixelFormat, Position, Resolution},
};

use super::RawEncodingRef;

struct RREncoding {
    background_pixel: Pixel,
    sub_rectangles: Vec<RRESubrectangle>,
    width: u16,
    height: u16,
    pixfmt: PixelFormat,
}

struct RRESubrectangle {
    pixel: Pixel,
    position: Position,
    dimensions: Resolution,
}

impl Encoding for RREncoding {
    fn get_type(&self) -> EncodingType {
        EncodingType::RRE
    }

    fn encode(&self) -> Box<dyn Iterator<Item = u8> + '_> {
        Box::new(
            (self.sub_rectangles.len() as u32)
                .to_be_bytes()
                .into_iter()
                .chain(self.background_pixel.bytes.iter().copied())
                .chain(self.sub_rectangles.iter().flat_map(|sr| {
                    sr.pixel
                        .bytes
                        .iter()
                        .copied()
                        .chain(sr.position.x.to_be_bytes().into_iter())
                        .chain(sr.position.y.to_be_bytes().into_iter())
                        .chain(sr.dimensions.width.to_be_bytes().into_iter())
                        .chain(sr.dimensions.height.to_be_bytes().into_iter())
                })),
        )
    }

    fn transform(&self, output: &PixelFormat) -> Box<dyn Encoding> {
        let input = &self.pixfmt;
        let background_pixel = Pixel {
            bytes: transform(&self.background_pixel.bytes, input, output),
        };
        let sub_rectangles = self
            .sub_rectangles
            .iter()
            .map(|sr| {
                let RRESubrectangle {
                    pixel,
                    position,
                    dimensions,
                } = sr;
                let pixel = Pixel {
                    bytes: transform(&pixel.bytes, input, output),
                };
                RRESubrectangle {
                    pixel,
                    position: *position,
                    dimensions: *dimensions,
                }
            })
            .collect();
        Box::new(Self {
            background_pixel,
            sub_rectangles,
            width: self.width,
            height: self.height,
            pixfmt: output.to_owned(),
        })
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn pixel_format(&self) -> &PixelFormat {
        &self.pixfmt
    }
}
