use crate::{
    encodings::{Encoding, EncodingType, Pixel},
    rfb::PixelFormat,
};

#[allow(dead_code)]
struct HextileEncoding {
    tiles: Vec<Vec<HextileTile>>,
}

impl HextileEncoding {
    pub fn from_raw(raw: Vec<u8>) -> Self {
        Self {
            tiles: todo!("create subrects. need dimensions"),
        }
    }
}

bitflags::bitflags! {
    pub struct HextileSubencMask: u8 {
        const RAW = 1 << 0;
        const BACKGROUND_SPECIFIED = 1 << 1;
        const FOREGROUND_SPECIFIED = 1 << 2;
        const ANY_SUBRECTS = 1 << 3;
        const SUBRECTS_COLORED = 1 << 4;
    }
}

impl Encoding for HextileEncoding {
    fn get_type(&self) -> EncodingType {
        EncodingType::Hextile
    }

    fn encode(&self) -> Vec<u8> {
        let mut v = Vec::new();
        for tile in &self.tiles {
            let subencoding_mask = todo!();
            v.push(subencoding_mask)
        }
        v
    }

    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding> {
        todo!()
    }
}

#[allow(dead_code)]
enum HextileTile {
    Raw(Vec<u8>),
    Encoded(HextileTileEncoded),
}

#[allow(dead_code)]
struct HextileTileEncoded {
    background: Option<Pixel>,
    foreground: Option<Pixel>,
    // TODO: finish this
}

impl HextileTileEncoded {
    fn subenc_mask(&self) -> HextileSubencMask {
        let mut x = HextileSubencMask::empty();
        if self.background.is_some() {
            x |= HextileSubencMask::BACKGROUND_SPECIFIED;
        }
        if self.foreground.is_some() {
            x |= HextileSubencMask::FOREGROUND_SPECIFIED;
        }
        x
    }
}
