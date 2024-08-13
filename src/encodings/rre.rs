use crate::{
    encodings::{Encoding, EncodingType, Pixel},
    pixel_formats::transform,
    rfb::{PixelFormat, Position, Resolution},
};

struct RREncoding {
    background_pixel: Pixel,
    sub_rectangles: Vec<RRESubrectangle>,
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

    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            (self.sub_rectangles.len() + 1) * (self.background_pixel.bytes.len() + 8) - 4,
        );
        buf.extend_from_slice(&(self.sub_rectangles.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.background_pixel.bytes);
        for sr in &self.sub_rectangles {
            buf.extend_from_slice(&sr.pixel.bytes);
            buf.extend_from_slice(&sr.position.x.to_be_bytes());
            buf.extend_from_slice(&sr.position.y.to_be_bytes());
            buf.extend_from_slice(&sr.dimensions.width.to_be_bytes());
            buf.extend_from_slice(&sr.dimensions.height.to_be_bytes());
        }
        buf
    }

    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding> {
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
        })
    }
}
