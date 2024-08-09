// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use crate::{
    pixel_formats::transform,
    rfb::{PixelFormat, Position, Resolution},
};

use crate::rfb::ColorSpecification;
use EncodingType::*;

#[derive(Debug)]
#[allow(unused)]
pub enum EncodingType {
    Raw,
    CopyRect,
    RRE,
    Hextile,
    TRLE,
    ZRLE,
    CursorPseudo,
    DesktopSizePseudo,
    JRLE,
    ZRLE2,
    JPEG,
    Zlib,
    CursorWithAlpha,
    Other(i32),
}

pub trait Encoding
where
    Self: Send,
{
    fn get_type(&self) -> EncodingType;

    /// Transform this encoding from its representation into a byte vector that can be passed to the client.
    fn encode(&self) -> Vec<u8>;

    /// Translates this encoding type from an input pixel format to an output format.
    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding>;
}

impl From<EncodingType> for i32 {
    fn from(e: EncodingType) -> Self {
        match e {
            Raw => 0,
            CopyRect => 1,
            RRE => 2,
            Hextile => 5,
            TRLE => 15,
            ZRLE => 16,
            CursorPseudo => -239,
            DesktopSizePseudo => -223,
            JRLE => 22,
            ZRLE2 => 24,
            JPEG => 21,
            Zlib => 6,
            CursorWithAlpha => -314,
            Other(n) => n,
        }
    }
}

impl From<i32> for EncodingType {
    fn from(value: i32) -> Self {
        match value {
            0 => Raw,
            1 => CopyRect,
            2 => RRE,
            5 => Hextile,
            15 => TRLE,
            16 => ZRLE,
            -239 => CursorPseudo,
            -223 => DesktopSizePseudo,
            22 => JRLE,
            24 => ZRLE2,
            21 => JPEG,
            6 => Zlib,
            -314 => CursorWithAlpha,
            v => Other(v),
        }
    }
}

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
        Raw
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

#[allow(dead_code)]
struct RREncoding {
    background_pixel: Pixel,
    sub_rectangles: Vec<RRESubrectangle>,
}

#[allow(dead_code)]
struct Pixel {
    bytes: Vec<u8>,
}

#[allow(dead_code)]
struct RRESubrectangle {
    pixel: Pixel,
    position: Position,
    dimensions: Resolution,
}

#[allow(dead_code)]
struct HextileEncoding {
    tiles: Vec<Vec<HextileTile>>,
}

impl HextileEncoding {
    pub fn from_raw(raw: Vec<u8>) -> Self {
        Self {
            tiles: todo!("create subrects"),
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
        for tile in self.tiles {
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
