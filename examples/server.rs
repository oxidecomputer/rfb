// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use async_trait::async_trait;
use env_logger;
use image::io::Reader as ImageReader;
use image::GenericImageView;
use rfb::encodings::RawEncoding;
use rfb::rfb::{FramebufferUpdate, ProtoVersion, Rectangle, SecurityType, SecurityTypes};
use rfb::server::{Server, VncServer, VncServerConfig, VncServerData};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = VncServerConfig {
        addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9000),
        version: ProtoVersion::Rfb38,
        sec_types: SecurityTypes(vec![SecurityType::None, SecurityType::VncAuthentication]),
        name: "rfb-example-server".to_string(),
    };
    let data = VncServerData {
        width: 1024,
        height: 768,
    };
    let server = ExampleServer {};
    let s = VncServer::new(server, config, data);
    s.start().await;
}

#[derive(Clone)]
struct ExampleServer {}

#[async_trait]
impl Server for ExampleServer {
    async fn get_framebuffer_update(&self) -> FramebufferUpdate {
        const LEN: usize = 1024 * 768 * 4;
        let mut pixels = vec![0u8; LEN];
        for i in 0..LEN {
            pixels[i] = 0xff;
        }

        let img = ImageReader::open("oxide.jpg").unwrap().decode().unwrap();
        for (x, y, pixel) in img.pixels() {
            let ux = x as usize;
            let uy = y as usize;
            pixels[uy * (1024 * 4) + ux * 4] = pixel[0];
            pixels[uy * (1024 * 4) + ux * 4 + 1] = pixel[1];
            pixels[uy * (1024 * 4) + ux * 4 + 2] = pixel[2];
            pixels[uy * (1024 * 4) + ux * 4 + 3] = pixel[3];
        }

        let r = Rectangle::new(0, 0, 1024, 768, Box::new(RawEncoding::new(pixels)));

        FramebufferUpdate::new(vec![r])
    }
}
