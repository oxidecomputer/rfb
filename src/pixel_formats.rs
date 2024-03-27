// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

//! Pixel Formats
//!
//! The pixel format data structure is specified in section 7.4 of RFC 6143. The data structure
//! describes how large a pixel is in bits, how many bits of the pixel are used for describing
//! color, and how colors are encoded in the pixel: either as a color specification or a color map,
//! with a color specification being the most common.
//!
//! The color specification format describes which bits in the pixel represent each color (red,
//! green, and blue), the max value of each color, and the endianness of the pixel. The location of
//! each color is described by a shift, in which the shift represents how many shifts are needed to
//! get the color value to the least significant bit (that is, how many right shifts are needed).
//!
//! For example, consider the 32-bit pixel value 0x01020304 with a depth of 24. Let's say the pixel
//! format has a red shift of 0, green shift of 8, blue shift of 16, all colors have a max value of
//! 255, and the host is little-endian. This is the pixel format little-endian xBGR.
//!
//! So to get the value of each color, we would do:
//! - red = (0x01020304 >> 0) & 255 = 0x04
//! - blue = (0x01020304 >> 8) & 255 = 0x03
//! - green = (0x01020304 >> 16) & 255 = 0x02
//!
//! This is relatively straightforward when considering a single pixel format. But an RFB server
//! must be able to translate between pixel formats, including translating between hosts of
//! different endianness. Further, it is convenient to represent pixels using a vector of bytes
//! instead of a vector of n-bit values, and transformations on pixels are done for groups of bytes
//! representing a pixel, rather than a single pixel value. But thinking about pixels in this
//! representation can be tricky, as the pixel format describes shifts, which operate the same on a
//! value regardless of endianness, but code operating on a byte vector must be endian-aware.
//!
//! If we think about the same pixel before as a byte vector, we would have the following
//! representation: [ 0x04, 0x03, 0x02, 0x01 ]. Note that the bytes are in reverse order from the
//! value above because the host is little-endian, so the least-significant byte (0x04) is first.
//!
//! So to get the value of each color, we would index into the pixel based on the shift. A shift of
//! 0 indicates the color is at the least significant byte (the first byte, byte 0 for
//!   little-endian pixels), a shift of 8 is the second least significant byte (1), and so on:
//! - red = pixel\[0\] & 255 = 0x04
//! - green = pixel\[1\] & 255 = 0x03
//! - blue = pixel\[2\] & 255 = 0x02
//!
//! Since the RFB server is considering pixels that might be from little-endian or big-endian hosts
//! though, consider if the same byte vector came from an RGBx big endian pixel. In that case, the
//! least significant byte is byte 3 and the most significant byte is byte 0. So the color values
//! for this vector would be:
//! - red = pixel\[3\] & 255 = 0x01
//! - green = pixel\[2\] & 255 = 0x02
//! - blue = pixel\[1\] & 255 = 0x03
//!

use crate::rfb::{ColorFormat, ColorSpecification, PixelFormat};

#[derive(Debug, thiserror::Error)]
pub enum PixelFormatError {
    #[error("unsupported or unknown fourcc: 0x{0:x}")]
    UnsupportedFourCc(u32),
}

///  Utility functions and constants related to fourcc codes.
///
/// Fourcc is a 4-byte ASCII code representing a pixel format. For example, the value
/// 0x34325258 is '42RX' in ASCII (34='4', 32='2', 52='R', and 58='X'). This code maps to the pixel
/// format 32-bit little-endian xRGB.
///
/// A good reference for mapping common fourcc codes to their corresponding pixel formats is the
/// drm_fourcc.h header file in the linux source code.
pub mod fourcc {
    use super::{ColorConstants, PixelFormatError};
    use crate::pixel_formats::{Rgb332Formats, Rgb565Formats, Rgb888Formats};
    use crate::rfb::PixelFormat;

    #[repr(u32)]
    pub enum FourCC {
        /// little-endian xRGB, 8:8:8:8
        XR24 = u32::from_ne_bytes(*b"XR24"),
        /// little-endian RGBx, 8:8:8:8
        RX24 = u32::from_ne_bytes(*b"RX24"),
        /// little-endian xBGR, 8:8:8:8
        XB24 = u32::from_ne_bytes(*b"XB24"),
        /// little-endian BGRx, 8:8:8:8
        BX24 = u32::from_ne_bytes(*b"BX24"),
        /// little-endian RGB, 5:6:5
        RG16 = u32::from_ne_bytes(*b"RG16"),
        /// little-endian BGR, 5:6:5
        BG16 = u32::from_ne_bytes(*b"BG16"),
        /// RGB, 3:3:2
        RGB8 = u32::from_ne_bytes(*b"RGB8"),
        /// BGR, 2:3:3
        BGR8 = u32::from_ne_bytes(*b"BGR8"),
    }

    pub const FOURCC_XR24: u32 = FourCC::XR24 as u32;
    pub const FOURCC_RX24: u32 = FourCC::RX24 as u32;
    pub const FOURCC_BX24: u32 = FourCC::BX24 as u32;
    pub const FOURCC_XB24: u32 = FourCC::XB24 as u32;
    pub const FOURCC_RG16: u32 = FourCC::RG16 as u32;
    pub const FOURCC_BG16: u32 = FourCC::BG16 as u32;
    pub const FOURCC_RGB8: u32 = FourCC::RGB8 as u32;
    pub const FOURCC_BGR8: u32 = FourCC::BGR8 as u32;

    impl TryFrom<u32> for FourCC {
        type Error = PixelFormatError;

        fn try_from(value: u32) -> Result<Self, Self::Error> {
            match value {
                FOURCC_XR24 => Ok(FourCC::XR24),
                FOURCC_RX24 => Ok(FourCC::RX24),
                FOURCC_XB24 => Ok(FourCC::XB24),
                FOURCC_BX24 => Ok(FourCC::BX24),
                FOURCC_RG16 => Ok(FourCC::RG16),
                FOURCC_BG16 => Ok(FourCC::BG16),
                FOURCC_RGB8 => Ok(FourCC::RGB8),
                FOURCC_BGR8 => Ok(FourCC::BGR8),
                v => Err(PixelFormatError::UnsupportedFourCc(v)),
            }
        }
    }

    impl From<&FourCC> for PixelFormat {
        fn from(value: &FourCC) -> Self {
            match value {
                FourCC::XR24 => Rgb888Formats::to_pix_fmt(false, 0),
                FourCC::RX24 => Rgb888Formats::to_pix_fmt(false, 8),
                FourCC::XB24 => Rgb888Formats::to_pix_fmt(true, 0),
                FourCC::BX24 => Rgb888Formats::to_pix_fmt(true, 8),
                FourCC::RG16 => Rgb565Formats::to_pix_fmt(false, 0),
                FourCC::BG16 => Rgb565Formats::to_pix_fmt(true, 0),
                FourCC::RGB8 => Rgb332Formats::to_pix_fmt(false, 0),
                FourCC::BGR8 => Rgb332Formats::to_pix_fmt(true, 0),
            }
        }
    }

    pub fn fourcc_to_pixel_format(fourcc: u32) -> Result<PixelFormat, PixelFormatError> {
        FourCC::try_from(fourcc).map(|fmt| PixelFormat::from(&fmt))
    }
}

trait ColorConstants {
    const BYTES_PER_PIXEL: usize = (Self::DEPTH as usize).next_power_of_two() / 8;
    const BITS_PER_PIXEL: u8 = (Self::BYTES_PER_PIXEL * 8) as u8;

    /// Number of bits used for color in a pixel
    const DEPTH: u8 = Self::RED_BITS + Self::GREEN_BITS + Self::BLUE_BITS;

    /// Number of bits used for red channel value
    const RED_BITS: u8;
    /// Number of bits used for green channel value
    const GREEN_BITS: u8;
    /// Number of bits used for blue channel value
    const BLUE_BITS: u8;

    /// Max value for red channel
    const RED_MAX: u16 = (1u16 << Self::RED_BITS) - 1;
    /// Max value for green channel
    const GREEN_MAX: u16 = (1u16 << Self::GREEN_BITS) - 1;
    /// Max value for blue channel
    const BLUE_MAX: u16 = (1u16 << Self::BLUE_BITS) - 1;

    /// Returns true if a shift as specified in a pixel format is valid for described formats.
    fn valid_shift(shift: u8) -> bool;

    /// Construct an appropriate PixelFormat definition for the given channel
    /// ordering and base shift (e.g. BGRx 8:8:8:8 would be (true, 8))
    fn to_pix_fmt(bgr_order: bool, base_shift: u8) -> PixelFormat {
        if bgr_order {
            PixelFormat {
                bits_per_pixel: Self::BITS_PER_PIXEL,
                depth: Self::DEPTH,
                big_endian: false,
                color_spec: ColorSpecification::ColorFormat(ColorFormat {
                    red_max: Self::RED_MAX,
                    green_max: Self::GREEN_MAX,
                    blue_max: Self::BLUE_MAX,
                    red_shift: base_shift,
                    green_shift: base_shift + Self::RED_BITS,
                    blue_shift: base_shift + Self::RED_BITS + Self::GREEN_BITS,
                }),
            }
        } else {
            PixelFormat {
                bits_per_pixel: Self::BITS_PER_PIXEL,
                depth: Self::DEPTH,
                big_endian: false,
                color_spec: ColorSpecification::ColorFormat(ColorFormat {
                    red_max: Self::RED_MAX,
                    green_max: Self::GREEN_MAX,
                    blue_max: Self::BLUE_MAX,
                    red_shift: base_shift + Self::GREEN_BITS + Self::BLUE_BITS,
                    green_shift: base_shift + Self::BLUE_BITS,
                    blue_shift: base_shift,
                }),
            }
        }
    }
}

struct Rgb888Formats;
struct Rgb565Formats;
struct Rgb332Formats;

impl ColorConstants for Rgb888Formats {
    const RED_BITS: u8 = 8;
    const GREEN_BITS: u8 = 8;
    const BLUE_BITS: u8 = 8;

    fn valid_shift(shift: u8) -> bool {
        shift == 0 || shift == 8 || shift == 16 || shift == 24
    }
}

impl ColorConstants for Rgb565Formats {
    const RED_BITS: u8 = 5;
    const GREEN_BITS: u8 = 6;
    const BLUE_BITS: u8 = 5;

    fn valid_shift(shift: u8) -> bool {
        shift == 0 || shift == 5 || shift == 11
    }
}

impl ColorConstants for Rgb332Formats {
    const RED_BITS: u8 = 3;
    const GREEN_BITS: u8 = 3;
    const BLUE_BITS: u8 = 2;

    // not the most thorough
    fn valid_shift(shift: u8) -> bool {
        shift == 0 || shift == 2 || shift == 3 || shift == 5 || shift == 6
    }
}

/// Utility functions for 32-bit RGB pixel formats, with 8-bits used per color.
#[deprecated]
pub mod rgb_888 {
    pub use super::transform;
    use crate::pixel_formats::{ColorConstants, Rgb888Formats};

    pub const BYTES_PER_PIXEL: usize = Rgb888Formats::BYTES_PER_PIXEL;
    pub const BITS_PER_PIXEL: u8 = Rgb888Formats::BITS_PER_PIXEL;

    /// Number of bits used for color in a pixel
    pub const DEPTH: u8 = Rgb888Formats::DEPTH;

    /// Number of bits used for a single color value
    pub const BITS_PER_COLOR: u8 = Rgb888Formats::RED_BITS;

    /// Max value for a given color
    pub const MAX_VALUE: u16 = Rgb888Formats::RED_MAX;

    /// Returns true if a shift as specified in a pixel format is valid for rgb888 formats.
    pub fn valid_shift(shift: u8) -> bool {
        Rgb888Formats::valid_shift(shift)
    }

    /// Returns the byte index into a 4-byte pixel vector for a given color shift, accounting for endianness.
    pub fn color_shift_to_index(shift: u8, big_endian: bool) -> usize {
        assert!(valid_shift(shift));

        if big_endian {
            ((DEPTH - shift) / BITS_PER_COLOR) as usize
        } else {
            (shift / BITS_PER_COLOR) as usize
        }
    }

    /// Returns the index of the unused byte (the only byte not representing R, G, or B).
    pub fn unused_index(r: usize, g: usize, b: usize) -> usize {
        (3 + 2 + 1) - r - g - b
    }
}

/// Translate between RGB formats.
pub fn transform(pixels: &[u8], input: &PixelFormat, output: &PixelFormat) -> Vec<u8> {
    if input == output {
        return pixels.to_vec();
    }

    let in_bytes_pp = input.bits_per_pixel.next_power_of_two() as usize / 8;
    let out_bytes_pp = output.bits_per_pixel.next_power_of_two() as usize / 8;

    let in_be_shift = 8 * (4 - in_bytes_pp);
    let out_be_shift = 8 * (4 - out_bytes_pp);

    let mut buf = Vec::with_capacity(pixels.len() * in_bytes_pp / out_bytes_pp);

    let ColorSpecification::ColorFormat(in_cf) = &input.color_spec else {
        unimplemented!("converting from indexed color mode");
    };
    let ColorSpecification::ColorFormat(out_cf) = &input.color_spec else {
        unimplemented!("converting to indexed color mode");
    };

    let mut i = 0;
    while i < pixels.len() {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&pixels[i..i + 4]);
        let word = if input.big_endian {
            u32::from_be_bytes(bytes) >> in_be_shift
        } else {
            u32::from_le_bytes(bytes)
        };

        // shift and mask
        let ir_raw = (word >> in_cf.red_shift) & in_cf.red_max as u32;
        let ig_raw = (word >> in_cf.green_shift) & in_cf.green_max as u32;
        let ib_raw = (word >> in_cf.blue_shift) & in_cf.blue_max as u32;

        // convert to new range
        let ir = ir_raw * out_cf.red_max as u32 / in_cf.red_max as u32;
        let ig = ig_raw * out_cf.green_max as u32 / in_cf.green_max as u32;
        let ib = ib_raw * out_cf.blue_max as u32 / in_cf.blue_max as u32;

        let or = ir << out_cf.red_shift;
        let og = ig << out_cf.green_shift;
        let ob = ib << out_cf.blue_shift;
        let word = or | og | ob;
        let bytes = if output.big_endian {
            (word << out_be_shift).to_be_bytes()
        } else {
            word.to_le_bytes()
        };
        buf.extend(&bytes[..out_bytes_pp]);

        i += in_bytes_pp;
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::{fourcc, transform};

    #[test]
    fn test_rgb_transform() {
        #[rustfmt::skip]
        let pixels = vec![
            0x00, 0x00, 0x00, 0x00,
            0x12, 0x34, 0x56, 0x78,
            0x9A, 0xBC, 0xDE, 0xF0,
            0xFF, 0xFF, 0xFF, 0xFF,
        ];

        // little-endian xRGB
        let xrgb_le = fourcc::fourcc_to_pixel_format(fourcc::FOURCC_XR24).unwrap();

        // little-endian RGBx
        let rgbx_le = fourcc::fourcc_to_pixel_format(fourcc::FOURCC_RX24).unwrap();

        // little-endian BGRx
        let bgrx_le = fourcc::fourcc_to_pixel_format(fourcc::FOURCC_BX24).unwrap();

        // little-endian xBGR
        let xbgr_le = fourcc::fourcc_to_pixel_format(fourcc::FOURCC_XB24).unwrap();

        // same pixel format
        assert_eq!(transform(&pixels, &xrgb_le, &xrgb_le), pixels);
        assert_eq!(transform(&pixels, &rgbx_le, &rgbx_le), pixels);
        assert_eq!(transform(&pixels, &bgrx_le, &bgrx_le), pixels);
        assert_eq!(transform(&pixels, &xbgr_le, &xbgr_le), pixels);

        // in all examples below, the 'x' non-channel value is dropped (zeroed)

        // little-endian xRGB -> little-endian RGBx
        //  B  G  R  x            x  B  G  R
        // [0, 1, 2, 3]       -> [0, 0, 1, 2]
        #[rustfmt::skip]
        let p2 = vec![
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x12, 0x34, 0x56,
            0x00, 0x9A, 0xBC, 0xDE,
            0x00, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(transform(&pixels, &xrgb_le, &rgbx_le), p2);

        // little-endian RGBx -> little-endian xRGB
        //  x  B  G  R            B  G  R  x
        // [0, 1, 2, 3]       -> [1, 2, 3, 0]
        #[rustfmt::skip]
        let p3 = vec![
            0x00, 0x00, 0x00, 0x00,
            0x34, 0x56, 0x78, 0x00,
            0xBC, 0xDE, 0xF0, 0x00,
            0xFF, 0xFF, 0xFF, 0x00,
        ];
        assert_eq!(transform(&pixels, &rgbx_le, &xrgb_le), p3);
        // little-endian BGRx -> little-endian xBGR
        //  x  R  G  B            R  G  B  x
        // [0, 1, 2, 3]       -> [1, 2, 3, 0]
        assert_eq!(transform(&pixels, &bgrx_le, &xbgr_le), p3);

        // little-endian xRGB -> little-endian BGRx
        //  B  G  R  x            x  R  G  B
        // [0, 1, 2, 3]       -> [0, 2, 1, 0]
        #[rustfmt::skip]
        let p4 = vec![
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x56, 0x34, 0x12,
            0x00, 0xF0, 0xDE, 0xBC,
            0x00, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(transform(&pixels, &xrgb_le, &bgrx_le), p4);
        // little-endian BGRx -> little-endian xRGB
        //  x  R  G  B            B  G  R  x
        // [0, 1, 2, 3]       -> [3, 2, 1, 0]
        assert_eq!(transform(&pixels, &bgrx_le, &xrgb_le), p4);
    }
}
