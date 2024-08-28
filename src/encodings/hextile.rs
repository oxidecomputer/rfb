use crate::{
    encodings::{Encoding, EncodingType, Pixel},
    rfb::{ConnectionContext, PixelFormat},
};

use super::RawEncodingRef;

#[allow(dead_code)]
struct HextileEncoding {
    tiles: Vec<Vec<HextileTile>>,
    width: u16,
    height: u16,
    pixfmt: PixelFormat,
}

impl From<&RawEncodingRef<'_>> for HextileEncoding {
    fn from(raw: &RawEncodingRef) -> Self {
        let pixfmt = raw.pixel_format().to_owned();
        let (width, height) = raw.dimensions();
        Self {
            tiles: todo!("create subrects. need dimensions"),
            width,
            height,
            pixfmt,
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

    fn encode(&self, _ctx: &mut ConnectionContext) -> Box<dyn Iterator<Item = u8> + '_> {
        Box::new(self.tiles.iter().flat_map(|tile| {
            let subencoding_mask = todo!();
            [todo!()].into_iter()
        }))
    }

    fn transform(&self, output: &PixelFormat) -> Box<dyn Encoding> {
        todo!()
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn pixel_format(&self) -> &PixelFormat {
        &self.pixfmt
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
