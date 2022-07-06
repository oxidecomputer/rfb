// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use crate::{
    pixel_formats::rgb_888,
    rfb::{PixelFormat, Position, Resolution},
};
use anyhow::Result;

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
    fn encode(&self) -> &Vec<u8>;

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

impl TryFrom<i32> for EncodingType {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Raw),
            1 => Ok(CopyRect),
            2 => Ok(RRE),
            5 => Ok(Hextile),
            15 => Ok(TRLE),
            16 => Ok(ZRLE),
            -239 => Ok(CursorPseudo),
            -223 => Ok(DesktopSizePseudo),
            22 => Ok(JRLE),
            24 => Ok(ZRLE2),
            21 => Ok(JPEG),
            6 => Ok(Zlib),
            -314 => Ok(CursorWithAlpha),
            v => Ok(EncodingType::Other(v)),
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
        EncodingType::Raw
    }

    fn encode(&self) -> &Vec<u8> {
        &self.pixels
    }

    fn transform(&self, input: &PixelFormat, output: &PixelFormat) -> Box<dyn Encoding> {
        // XXX: This assumes the pixel formats are both rgb888. The server code verifies this
        // before calling.
        assert!(input.is_rgb_888());
        assert!(output.is_rgb_888());

        Box::new(Self {
            pixels: rgb_888::transform(&self.pixels, &input, &output),
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
