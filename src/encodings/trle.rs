use std::iter::{from_fn, once};

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
    PlainRLE { runs: Vec<(CPixel, usize)> },
    /// 129
    PaletteRLEReused { runs: Vec<(u8, usize)> },
    /// 130-255
    PaletteRLE {
        palette: Vec<CPixel>,
        runs: Vec<(u8, usize)>,
    },
}

fn rle(mut length: usize) -> impl Iterator<Item = u8> {
    from_fn(move || {
        if length == 0 {
            None
        } else if length > 0xFF {
            length -= 0xFF;
            Some(0xFF)
        } else {
            let byte = (length - 1) as u8;
            length = 0;
            Some(byte)
        }
    })
}

fn pal_rle((index, length): &(u8, usize)) -> Box<dyn Iterator<Item = u8>> {
    if *length == 1 {
        Box::new(once(*index))
    } else {
        Box::new(once(*index | 0x80).chain(rle(*length)))
    }
}

impl TRLETile {
    /// Subencoding of the tile according to RFB 6143 7.7.5
    fn encode(&self) -> Box<dyn Iterator<Item = u8> + '_> {
        match self {
            TRLETile::Raw { pixels } => {
                Box::new(once(0u8).chain(pixels.iter().flat_map(|c| c.bytes.iter().copied())))
            }
            TRLETile::SolidColor { color } => {
                Box::new(once(1u8).chain(color.bytes.iter().copied()))
            }
            TRLETile::PackedPalette {
                palette,
                packed_pixels,
            } => Box::new(
                once(palette.len() as u8)
                    .chain(palette.iter().flat_map(|c| c.bytes.iter().copied()))
                    .chain(packed_pixels.iter().map(|p| p.0)),
            ),
            TRLETile::PackedPaletteReused { packed_pixels } => {
                Box::new(once(127u8).chain(packed_pixels.iter().map(|p| p.0)))
            }
            TRLETile::PlainRLE { runs } => {
                Box::new(once(128).chain(
                    runs.iter().flat_map(|(color, length)| {
                        color.bytes.iter().copied().chain(rle(*length))
                    }),
                ))
            }
            TRLETile::PaletteRLEReused { runs } => {
                Box::new(once(129).chain(runs.iter().flat_map(pal_rle)))
            }
            TRLETile::PaletteRLE { palette, runs } => Box::new(
                once(
                    (palette.len() + 128)
                        .try_into()
                        .expect("TRLE tile palette too large!"),
                )
                .chain(palette.iter().flat_map(|c| c.bytes.iter().copied()))
                .chain(runs.iter().flat_map(pal_rle)),
            ),
        }
    }
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
        self.tiles
            .iter()
            .flat_map(|row| row.iter().flat_map(|tile| tile.encode()))
            .collect()
    }

    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding> {
        todo!()
    }
}
