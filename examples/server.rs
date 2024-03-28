// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, ValueEnum};
use env_logger;
use image::io::Reader as ImageReader;
use image::GenericImageView;
use log::info;
use rfb::encodings::RawEncoding;
use rfb::pixel_formats::fourcc::FourCC;
use rfb::pixel_formats::transform;
use rfb::rfb::{
    FramebufferUpdate, KeyEvent, PixelFormat, ProtoVersion, Rectangle, SecurityType, SecurityTypes,
};
use rfb::server::{Server, VncServer, VncServerConfig, VncServerData};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

const WIDTH: usize = 1024;
const HEIGHT: usize = 768;

#[derive(Parser, Debug)]
/// A simple VNC server that displays a single image or color, in a given pixel format
///
/// By default, the server will display the Oxide logo image using little-endian RGBx as its pixel format. To specify an alternate image or color, use the `-i` flag:
/// ./example-server -i test-tubes
/// ./example-server -i red
///
/// To specify an alternate pixel format, use the `--big-endian` flag and/or the ordering flags. The
/// server will transform the input image/color to the requested pixel format and use the format
/// for the RFB protocol.
///
/// For example, to use big-endian xRGB:
/// ./example-server --big-endian true -r 1 -g 2 -b 3
///
struct Args {
    /// Image/color to display from the server
    #[clap(value_enum, short, long, default_value_t = Image::Oxide)]
    image: Image,

    /// Pixel format
    #[clap(short, long, default_value = "XR24", action = clap::ArgAction::Set)]
    fourcc: FourCC,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
enum Image {
    Oxide,
    TestTubes,
    Red,
    Green,
    Blue,
    White,
    Black,
}

#[derive(Clone)]
struct ExampleServer {
    display: Image,
    pixfmt: PixelFormat,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let pixfmt = PixelFormat::from(&args.fourcc);
    info!(
        "Starting server: image: {:?}, pixel format; {:#?}",
        args.image, pixfmt
    );

    let config = VncServerConfig {
        addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9000),
        version: ProtoVersion::Rfb38,
        sec_types: SecurityTypes(vec![SecurityType::None, SecurityType::VncAuthentication]),
        name: "rfb-example-server".to_string(),
    };
    let data = VncServerData {
        width: WIDTH as u16,
        height: HEIGHT as u16,
        input_pixel_format: pixfmt.clone(),
    };
    let server = ExampleServer {
        display: args.image,
        pixfmt,
    };
    let s = VncServer::new(server, config, data);
    s.start().await?;

    Ok(())
}

fn generate_color(img: Image, pixfmt: &PixelFormat) -> Vec<u8> {
    let bytes_pp = pixfmt.bits_per_pixel as usize / 8;
    let len = WIDTH * HEIGHT * bytes_pp;
    let mut pixels = Vec::with_capacity(len);

    let color = match img {
        Image::Red => 0xFF000000u32.to_le_bytes(),
        Image::Green => 0x00FF0000u32.to_le_bytes(),
        Image::Blue => 0x0000FF00u32.to_le_bytes(),
        Image::White => 0xFFFFFF00u32.to_le_bytes(),
        Image::Black => 0u32.to_le_bytes(),
        _ => unreachable!(),
    };
    let bytes = transform(&color, &PixelFormat::from(&FourCC::RX24), pixfmt);

    while pixels.len() < len {
        pixels.extend(&bytes);
    }

    pixels
}

fn generate_image(name: &str, pixfmt: &PixelFormat) -> Vec<u8> {
    const RGBX24_BYTES_PP: usize = 4;
    const LEN: usize = WIDTH * HEIGHT * RGBX24_BYTES_PP;

    let mut pixels = vec![0xffu8; LEN];

    let img = ImageReader::open(name).unwrap().decode().unwrap();

    // Convert the input image pixels to the requested pixel format.
    for (x, y, pixel) in img.pixels() {
        let ux = x as usize;
        let uy = y as usize;

        let y_offset = WIDTH * RGBX24_BYTES_PP;
        let x_offset = ux * RGBX24_BYTES_PP;
        let offset = uy * y_offset + x_offset;

        pixels[offset..offset + 4].copy_from_slice(&pixel.0);
    }
    transform(&pixels, &PixelFormat::from(&FourCC::XB24), pixfmt)
}

fn generate_pixels(img: Image, pixfmt: &PixelFormat) -> Vec<u8> {
    match img {
        Image::Oxide => generate_image("example-images/oxide.jpg", pixfmt),
        Image::TestTubes => generate_image("example-images/test-tubes.jpg", pixfmt),
        Image::Red | Image::Green | Image::Blue | Image::White | Image::Black => {
            generate_color(img, pixfmt)
        }
    }
}

#[async_trait]
impl Server for ExampleServer {
    async fn get_framebuffer_update(&self) -> FramebufferUpdate {
        let pixels_width = 1024;
        let pixels_height = 768;
        let pixels = generate_pixels(self.display, &self.pixfmt);
        let r = Rectangle::new(
            0,
            0,
            pixels_width,
            pixels_height,
            Box::new(RawEncoding::new(pixels)),
        );
        FramebufferUpdate::new(vec![r])
    }

    async fn key_event(&self, _ke: KeyEvent) {}
}
