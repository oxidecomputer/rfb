use crate::encodings::{Encoding, EncodingType};
use crate::rfb::PixelFormat;

pub struct TRLEncoding {
    tiles: Vec<Vec<TRLETile>>,
}

#[repr(transparent)]
struct PackedIndeces(u8);

// impl From<&[u8; 2]> for PackedIndeces {
//     fn from(&[left, right]: &[u8; 2]) -> Self {
//         Self((left << 4) | (right & 0xF))
//     }
// }
impl PackedIndeces {
    // fn new<const N: usize>(indeces: &[u8; N]) -> Self {
    //     const BPP: usize = 16 / size_of;
    // }
    fn new_4bpp(&[left, right]: &[u8; 2]) -> Self {
        Self((left << 4) | (right & 0xF))
    }
    fn new_2bpp(indeces: &[u8]) -> Self {
        let mut x = 0;
        assert!(indeces.len() <= 4);
        for (pos, ci) in indeces.iter().copied().enumerate() {
            x |= ((ci << 6) & 0xC0) >> (pos * 2);
        }
        Self(x)
    }
    fn new_1bpp(indeces: &[u8]) -> Self {
        let mut x = 0;
        assert!(indeces.len() <= 16);
        for (pos, ci) in indeces.iter().copied().enumerate() {
            x |= ((ci << 7) & 0x80) >> pos;
        }
        Self(x)
    }
}

// may be able to reuse this for ZRLE? (64px instead of 16px)
enum TRLETile {
    /// 0
    Raw { pixels: Vec<CPixel> },
    /// 1
    SolidColor { color: CPixel },
    /// 2-16
    PackedPalette {
        palette: Vec<CPixel>,
        packed_pixels: Vec<PackedIndeces>,
    },
    /// 127
    PackedPaletteReused { packed_pixels: Vec<PackedIndeces> },
    /// 128
    PlainRLE { color: CPixel, length: usize },
    /// 129
    PaletteRLEReused { pixels: Vec<u8> },
    /// 130-255
    PaletteRLE {
        palette: Vec<CPixel>,
        pixels: Vec<u8>,
    },
}

struct CPixel {
    format: PixelFormat,
    bytes: Vec<u8>,
}

impl Encoding for TRLEncoding {
    fn get_type(&self) -> EncodingType {
        EncodingType::TRLE
    }

    fn encode(&self) -> Vec<u8> {
        todo!()
    }

    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding> {
        todo!()
    }
}
