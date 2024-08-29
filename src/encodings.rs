// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use std::sync::Arc;

use crate::rfb::ConnectionContext;
use crate::rfb::PixelFormat;

use async_trait::async_trait;
use futures::stream::BoxStream;
use EncodingType::*;

// mod hextile;
mod raw;
// mod rre;
mod trle;
mod zlib;

pub use raw::RawEncoding;
pub use raw::RawEncodingRef;

pub use zlib::ZlibEncoding;
pub use zlib::ZlibEncodingRef;

pub use trle::TRLEncoding;
pub use trle::ZRLEncoding;

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

struct Pixel {
    bytes: Vec<u8>,
}

#[async_trait]
pub trait Encoding: Send + Sync {
    fn get_type(&self) -> EncodingType;

    /// Return the width and height in pixels of the encoded screen region.
    fn dimensions(&self) -> (u16, u16);

    /// Return the pixel format of this encoding's data.
    fn pixel_format(&self) -> &PixelFormat;

    /// Transform this encoding from its representation into a byte sequence that can be passed to the client.
    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8>;

    /// Translates this encoding type from its current pixel format to the given format.
    fn transform(&self, output: &PixelFormat) -> Box<dyn Encoding>;
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
