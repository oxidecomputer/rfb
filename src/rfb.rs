// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use std::sync::Arc;

use async_trait::async_trait;
use bitflags::bitflags;
use cancel_safe_futures::sync::RobustMutex;
use futures::{
    future::BoxFuture,
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::encodings::{Encoding, EncodingType};
use crate::keysym::KeySym;

pub struct ConnectionContext {
    pub zlib: RobustMutex<flate2::Compress>,
}
impl Default for ConnectionContext {
    fn default() -> Self {
        Self {
            zlib: RobustMutex::new(flate2::Compress::new(flate2::Compression::fast(), false)),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid protocol version message")]
    InvalidProtocolVersion,

    #[error("invalid security type message ({0})")]
    InvalidSecurityType(u8),

    #[error("invalid text encoding")]
    InvalidTextEncoding,

    #[error("unknown client message type ({0})")]
    UnknownClientMessageType(u8),

    #[error(transparent)]
    KeySymError(#[from] crate::keysym::KeySymError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub trait ReadMessage {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>>
    where
        Self: Sized;
}

#[async_trait]
pub trait WriteMessage {
    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8>;
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>>;
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum ProtoVersion {
    Rfb33,
    Rfb37,
    Rfb38,
}

impl ReadMessage for ProtoVersion {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async move {
            let mut buf = [0u8; 12];
            stream.read_exact(&mut buf).await?;
            Self::try_from(buf)
        }
        .boxed()
    }
}

impl TryFrom<[u8; 12]> for ProtoVersion {
    type Error = ProtocolError;

    fn try_from(buf: [u8; 12]) -> Result<Self, Self::Error> {
        match &buf {
            b"RFB 003.003\n" => Ok(ProtoVersion::Rfb33),
            b"RFB 003.007\n" => Ok(ProtoVersion::Rfb37),
            b"RFB 003.008\n" => Ok(ProtoVersion::Rfb38),
            _ => Err(ProtocolError::InvalidProtocolVersion),
        }
    }
}

impl Into<&'static [u8; 12]> for &ProtoVersion {
    fn into(self) -> &'static [u8; 12] {
        match self {
            ProtoVersion::Rfb33 => b"RFB 003.003\n",
            ProtoVersion::Rfb37 => b"RFB 003.007\n",
            ProtoVersion::Rfb38 => b"RFB 003.008\n",
        }
    }
}

#[async_trait]
impl WriteMessage for ProtoVersion {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            let s: &[u8; 12] = (&self).into();
            Ok(stream.write_all(s).await?)
        }
        .boxed()
    }
    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        let s: &[u8; 12] = self.into();
        stream::iter(s.iter().copied()).boxed()
    }
}

// Section 7.1.2
#[derive(Debug, Clone)]
pub struct SecurityTypes(pub Vec<SecurityType>);

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum SecurityType {
    None = 0,
    VncAuthentication = 1,
}

#[async_trait]
impl WriteMessage for SecurityTypes {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            // TODO: fix cast
            stream.write_u8(self.0.len() as u8).await?;
            for t in self.0.into_iter() {
                t.write_to(stream).await?;
            }

            Ok(())
        }
        .boxed()
    }
    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter([self.0.len() as u8].into_iter()) // TODO: fix cast
            .chain(
                stream::iter(self.0.iter())
                    .then(move |t| t.encode(ctx.to_owned()))
                    .flatten(),
            )
            .boxed()
    }
}

impl ReadMessage for SecurityType {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async move {
            let t = stream.read_u8().await?;
            match t {
                1 => Ok(SecurityType::None),
                2 => Ok(SecurityType::VncAuthentication),
                v => Err(ProtocolError::InvalidSecurityType(v)),
            }
        }
        .boxed()
    }
}

#[async_trait]
impl WriteMessage for SecurityType {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            stream.write_u8(self as u8).await?;
            Ok(())
        }
        .boxed()
    }

    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter([*self as u8].into_iter()).boxed()
    }
}

// Section 7.1.3
pub enum SecurityResult {
    Success,
    Failure(String),
}

#[async_trait]
impl WriteMessage for SecurityResult {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            match self {
                SecurityResult::Success => {
                    stream.write_u32(0).await?;
                }
                SecurityResult::Failure(s) => {
                    stream.write_u32(1).await?;
                    stream.write_all(s.as_bytes()).await?;
                }
            };

            Ok(())
        }
        .boxed()
    }
    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        match self {
            SecurityResult::Success => stream::iter(0u32.to_be_bytes().into_iter()).boxed(),
            SecurityResult::Failure(s) => {
                stream::iter(1u32.to_be_bytes().into_iter().chain(s.bytes())).boxed()
            }
        }
    }
}

// Section 7.3.1
#[derive(Debug)]
pub struct ClientInit {
    pub shared: bool,
}

impl ReadMessage for ClientInit {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async {
            let flag = stream.read_u8().await?;
            match flag {
                0 => Ok(ClientInit { shared: false }),
                _ => Ok(ClientInit { shared: true }),
            }
        }
        .boxed()
    }
}

// Section 7.3.2
#[derive(Debug)]
pub struct ServerInit {
    initial_res: Resolution,
    pixel_format: PixelFormat,
    name: String,
}

impl ServerInit {
    pub fn new(width: u16, height: u16, name: String, pixel_format: PixelFormat) -> Self {
        Self {
            initial_res: Resolution { width, height },
            pixel_format,
            name,
        }
    }
}

#[async_trait]
impl WriteMessage for ServerInit {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            self.initial_res.write_to(stream).await?;
            self.pixel_format.write_to(stream).await?;

            // TODO: cast properly
            stream.write_u32(self.name.len() as u32).await?;
            stream.write_all(self.name.as_bytes()).await?;

            Ok(())
        }
        .boxed()
    }

    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        self.initial_res
            .encode(ctx.clone())
            .await
            .chain(self.pixel_format.encode(ctx).await)
            .chain(stream::iter(
                (self.name.len() as u32)
                    .to_be_bytes()
                    .into_iter()
                    .chain(self.name.bytes()),
            ))
            .boxed()
    }
}

pub enum _ServerMessage {
    FramebufferUpdate(FramebufferUpdate),
    SetColorMapEntries(SetColorMapEntries),
    Bell,
    ServerCutText(CutText),
}

pub struct FramebufferUpdate {
    rectangles: Vec<Rectangle>,
}

impl FramebufferUpdate {
    pub fn new(rectangles: Vec<Rectangle>) -> Self {
        FramebufferUpdate { rectangles }
    }

    pub fn transform(&self, input_pf: &PixelFormat, output_pf: &PixelFormat) -> Self {
        let rectangles = self
            .rectangles
            .iter()
            .map(|r| r.transform(input_pf, output_pf))
            .collect();
        FramebufferUpdate { rectangles }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl ReadMessage for Position {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async {
            let x = stream.read_u16().await?;
            let y = stream.read_u16().await?;

            Ok(Position { x, y })
        }
        .boxed()
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Resolution {
    pub width: u16,
    pub height: u16,
}

impl ReadMessage for Resolution {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async {
            let width = stream.read_u16().await?;
            let height = stream.read_u16().await?;

            Ok(Resolution { width, height })
        }
        .boxed()
    }
}

#[async_trait]
impl WriteMessage for Resolution {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            stream.write_u16(self.width).await?;
            stream.write_u16(self.height).await?;
            Ok(())
        }
        .boxed()
    }

    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter(
            self.width
                .to_be_bytes()
                .into_iter()
                .chain(self.height.to_be_bytes().into_iter()),
        )
        .boxed()
    }
}

pub struct Rectangle {
    position: Position,
    dimensions: Resolution,
    data: Box<dyn Encoding>,
}

impl Rectangle {
    pub fn new(x: u16, y: u16, width: u16, height: u16, data: Box<dyn Encoding>) -> Self {
        Rectangle {
            position: Position { x, y },
            dimensions: Resolution { width, height },
            data,
        }
    }

    pub fn transform(&self, input_pf: &PixelFormat, output_pf: &PixelFormat) -> Self {
        // TODO: refactor out of method args here?
        assert_eq!(input_pf, self.data.pixel_format());
        Rectangle {
            position: self.position,
            dimensions: self.dimensions,
            data: self.data.transform(output_pf),
        }
    }
}

#[async_trait]
impl WriteMessage for Rectangle {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            let encoding_type: i32 = self.data.get_type().into();

            stream.write_u16(self.position.x).await?;
            stream.write_u16(self.position.y).await?;
            stream.write_u16(self.dimensions.width).await?;
            stream.write_u16(self.dimensions.height).await?;
            stream.write_i32(encoding_type).await?;

            // TODO: avoid collect alloc?
            let data: Vec<_> = self.data.encode(todo!()).await.collect().await;
            stream.write_all(&data).await?;

            Ok(())
        }
        .boxed()
    }

    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter(
            self.position
                .x
                .to_be_bytes()
                .into_iter()
                .chain(self.position.y.to_be_bytes().into_iter())
                .chain(self.dimensions.width.to_be_bytes().into_iter())
                .chain(self.dimensions.height.to_be_bytes().into_iter())
                .chain(i32::from(self.data.get_type()).to_be_bytes().into_iter()),
        )
        .chain(self.data.encode(ctx.clone()).await)
        .boxed()
    }
}

#[async_trait]
impl WriteMessage for FramebufferUpdate {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            // TODO: type function?
            stream.write_u8(0).await?;

            // 1 byte of padding
            stream.write_u8(0).await?;

            // number of rectangles
            let n_rect = self.rectangles.len() as u16;
            stream.write_u16(n_rect).await?;

            // rectangles
            for r in self.rectangles.into_iter() {
                r.write_to(stream).await?;
            }

            Ok(())
        }
        .boxed()
    }

    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        // number of rectangles
        let n_rect = self.rectangles.len() as u16;
        stream::iter(
            std::iter::once(0u8) // TODO: type function?
                .chain(std::iter::once(0u8)) // 1 byte of padding
                .chain(n_rect.to_be_bytes().into_iter()),
        )
        .chain(
            stream::iter(self.rectangles.iter())
                .then(move |r| r.encode(ctx.clone()))
                .flatten(),
        )
        .boxed()
    }
}

#[derive(Debug)]
pub struct SetColorMapEntries {
    _colors: Vec<_ColorMapEntry>,
}

#[derive(Debug)]
pub struct _ColorMapEntry {
    _color: u16,
    _red: u16,
    _blue: u16,
    _green: u16,
}

// TODO: only ISO 8859-1 (Latin-1) text supported
// used for client and server
#[derive(Debug)]
pub struct CutText {
    _text: String,
}

// Section 7.4
#[derive(Debug, Clone, PartialEq)]
pub struct PixelFormat {
    pub bits_per_pixel: u8, // TODO: must be 8, 16, or 32
    pub depth: u8,          // TODO: must be < bits_per_pixel
    pub big_endian: bool,
    pub color_spec: ColorSpecification,
}

impl PixelFormat {
    /// Constructor for a PixelFormat that uses a color format to specify colors.
    pub fn new_colorformat(
        bpp: u8,
        depth: u8,
        big_endian: bool,
        red_shift: u8,
        red_max: u16,
        green_shift: u8,
        green_max: u16,
        blue_shift: u8,
        blue_max: u16,
    ) -> Self {
        PixelFormat {
            bits_per_pixel: bpp,
            depth,
            big_endian,
            color_spec: ColorSpecification::ColorFormat(ColorFormat {
                red_max,
                green_max,
                blue_max,
                red_shift,
                green_shift,
                blue_shift,
            }),
        }
    }

    pub fn value_mask(&self) -> Option<u64> {
        match self.color_spec {
            ColorSpecification::ColorFormat(ColorFormat {
                red_max,
                green_max,
                blue_max,
                red_shift,
                green_shift,
                blue_shift,
            }) => Some(
                ((red_max as u64) << red_shift)
                    | ((green_max as u64) << green_shift)
                    | ((blue_max as u64) << blue_shift),
            ),
            ColorSpecification::ColorMap(_) => None,
        }
    }

    /// Returns true if the pixel format is RGB888 (8-bits per color and 32 bits per pixel).
    #[deprecated]
    pub fn is_rgb_888(&self) -> bool {
        #[allow(deprecated)]
        {
            use crate::pixel_formats::rgb_888;

            if self.bits_per_pixel != rgb_888::BITS_PER_PIXEL || self.depth != rgb_888::DEPTH {
                return false;
            }

            match &self.color_spec {
                ColorSpecification::ColorFormat(cf) => {
                    (cf.red_max == rgb_888::MAX_VALUE)
                        && (cf.green_max == rgb_888::MAX_VALUE)
                        && (cf.blue_max == rgb_888::MAX_VALUE)
                        && (rgb_888::valid_shift(cf.red_shift))
                        && (rgb_888::valid_shift(cf.green_shift))
                        && (rgb_888::valid_shift(cf.blue_shift))
                }
                ColorSpecification::ColorMap(_) => false,
            }
        }
    }

    /// Returns true if the pixel format is supported (currently only certain
    /// variants of RGB888, RGB565, and RGB332).
    pub fn is_supported(&self) -> bool {
        for fcc in crate::pixel_formats::fourcc::SUPPORTED {
            let fmt: PixelFormat = fcc.into();
            if *self == fmt {
                return true;
            }
        }
        return false;
    }
}

impl ReadMessage for PixelFormat {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async {
            let bits_per_pixel = stream.read_u8().await?;
            let depth = stream.read_u8().await?;
            let be_flag = stream.read_u8().await?;
            let big_endian = match be_flag {
                0 => false,
                _ => true,
            };
            let color_spec = ColorSpecification::read_from(stream).await?;

            // 3 bytes of padding
            let mut buf = [0u8; 3];
            stream.read_exact(&mut buf).await?;

            if let ColorSpecification::ColorMap(..) = &color_spec {
                todo!("SetColorMapEntries"); //.write_to(stream).await?;
            }

            Ok(Self {
                bits_per_pixel,
                depth,
                big_endian,
                color_spec,
            })
        }
        .boxed()
    }
}

#[async_trait]
impl WriteMessage for PixelFormat {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            stream.write_u8(self.bits_per_pixel).await?;
            stream.write_u8(self.depth).await?;
            stream.write_u8(if self.big_endian { 1 } else { 0 }).await?;
            self.color_spec.write_to(stream).await?;

            // 3 bytes of padding
            let buf = [0u8; 3];
            stream.write_all(&buf).await?;

            Ok(())
        }
        .boxed()
    }
    async fn encode(&self, ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        stream::iter([self.bits_per_pixel, self.depth, self.big_endian as u8].into_iter())
            .chain(self.color_spec.encode(ctx).await)
            .chain(stream::iter([0u8; 3].into_iter())) // 3 bytes of padding
            .boxed()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorSpecification {
    ColorFormat(ColorFormat),
    ColorMap(ColorMap), // TODO: implement
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorFormat {
    // TODO: maxes must be 2^N - 1 for N bits per color
    pub red_max: u16,
    pub green_max: u16,
    pub blue_max: u16,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorMap {
    // we currently just use VESA_VGA_256_COLOR_PALETTE
}

impl ReadMessage for ColorSpecification {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async {
            let tc_flag = stream.read_u8().await?;
            match tc_flag {
                0 => Ok(ColorSpecification::ColorMap(ColorMap {})),
                _ => {
                    // ColorFormat
                    let red_max = stream.read_u16().await?;
                    let green_max = stream.read_u16().await?;
                    let blue_max = stream.read_u16().await?;

                    let red_shift = stream.read_u8().await?;
                    let green_shift = stream.read_u8().await?;
                    let blue_shift = stream.read_u8().await?;

                    Ok(ColorSpecification::ColorFormat(ColorFormat {
                        red_max,
                        green_max,
                        blue_max,
                        red_shift,
                        green_shift,
                        blue_shift,
                    }))
                }
            }
        }
        .boxed()
    }
}

#[async_trait]
impl WriteMessage for ColorSpecification {
    fn write_to<'a>(self, stream: &'a mut TcpStream) -> BoxFuture<'a, Result<(), ProtocolError>> {
        async move {
            match self {
                ColorSpecification::ColorFormat(cf) => {
                    stream.write_u8(1).await?; // true color
                    stream.write_u16(cf.red_max).await?;
                    stream.write_u16(cf.green_max).await?;
                    stream.write_u16(cf.blue_max).await?;

                    stream.write_u8(cf.red_shift).await?;
                    stream.write_u8(cf.green_shift).await?;
                    stream.write_u8(cf.blue_shift).await?;
                }
                ColorSpecification::ColorMap(_cm) => {
                    // first 0 byte is true-color-flag = false;
                    // the remaining 9 are the above max/shift fields,
                    // which aren't relevant in ColorMap mode
                    stream.write_all(&[0u8; 10]).await?;
                }
            };

            Ok(())
        }
        .boxed()
    }

    async fn encode(&self, _ctx: Arc<ConnectionContext>) -> BoxStream<u8> {
        match self {
            ColorSpecification::ColorFormat(cf) => {
                stream::iter(
                    std::iter::once(1u8) // true color
                        .chain(cf.red_max.to_be_bytes().into_iter())
                        .chain(cf.green_max.to_be_bytes().into_iter())
                        .chain(cf.blue_max.to_be_bytes().into_iter())
                        .chain([cf.red_shift, cf.green_shift, cf.blue_shift].into_iter()),
                )
                .boxed()
            }
            ColorSpecification::ColorMap(_cm) => {
                // first 0 byte is true-color-flag = false;
                // the remaining 9 are the above max/shift fields,
                // which aren't relevant in ColorMap mode
                stream::iter(std::iter::repeat(0).take(10)).boxed()
            }
        }
    }
}

// Section 7.5
pub enum ClientMessage {
    SetPixelFormat(PixelFormat),
    SetEncodings(Vec<EncodingType>),
    FramebufferUpdateRequest(FramebufferUpdateRequest),
    KeyEvent(KeyEvent),
    PointerEvent(PointerEvent),
    ClientCutText(String),
}

impl ReadMessage for ClientMessage {
    fn read_from<'a>(
        stream: &'a mut TcpStream,
    ) -> BoxFuture<'a, Result<ClientMessage, ProtocolError>> {
        async {
            let t = stream.read_u8().await?;
            let res = match t {
                0 => {
                    // SetPixelFormat
                    let mut padding = [0u8; 3];
                    stream.read_exact(&mut padding).await?;
                    let pixel_format = PixelFormat::read_from(stream).await?;
                    Ok(ClientMessage::SetPixelFormat(pixel_format))
                }

                2 => {
                    // SetEncodings
                    stream.read_u8().await?; // 1 byte of padding
                    let num_encodings = stream.read_u16().await?;

                    // TODO: what to do if num_encodings is 0

                    let mut encodings = Vec::new();
                    for _ in 0..num_encodings {
                        let e: EncodingType = stream.read_i32().await?.into();
                        encodings.push(e);
                    }

                    Ok(ClientMessage::SetEncodings(encodings))
                }
                3 => {
                    // FramebufferUpdateRequest
                    let incremental = match stream.read_u8().await? {
                        0 => false,
                        _ => true,
                    };
                    let position = Position::read_from(stream).await?;
                    let resolution = Resolution::read_from(stream).await?;

                    let fbu_req = FramebufferUpdateRequest {
                        incremental,
                        position,
                        resolution,
                    };

                    Ok(ClientMessage::FramebufferUpdateRequest(fbu_req))
                }
                4 => {
                    // KeyEvent
                    let is_pressed = match stream.read_u8().await? {
                        0 => false,
                        _ => true,
                    };

                    // 2 bytes of padding
                    stream.read_u16().await?;

                    let keysym_raw = stream.read_u32().await?;
                    let keysym = KeySym::try_from(keysym_raw)?;

                    let key_event = KeyEvent {
                        is_pressed,
                        keysym,
                        keysym_raw,
                    };

                    Ok(ClientMessage::KeyEvent(key_event))
                }
                5 => {
                    // PointerEvent
                    let pointer_event = PointerEvent::read_from(stream).await?;
                    Ok(ClientMessage::PointerEvent(pointer_event))
                }
                6 => {
                    // ClientCutText

                    // 3 bytes of padding
                    let mut padding = [0u8; 3];
                    stream.read_exact(&mut padding).await?;

                    let len = stream.read_u32().await?;
                    let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
                    stream.read_exact(&mut buf).await?;

                    // TODO: The encoding RFB uses is ISO 8859-1 (Latin-1), which is a subset of
                    // utf-8. Determine if this is the right approach.
                    let text =
                        String::from_utf8(buf).map_err(|_| ProtocolError::InvalidTextEncoding)?;

                    Ok(ClientMessage::ClientCutText(text))
                }
                unknown => Err(ProtocolError::UnknownClientMessageType(unknown)),
            };

            res
        }
        .boxed()
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FramebufferUpdateRequest {
    incremental: bool,
    position: Position,
    resolution: Resolution,
}

#[derive(Debug, Copy, Clone)]
pub struct KeyEvent {
    is_pressed: bool,
    keysym: KeySym,
    keysym_raw: u32,
}

impl KeyEvent {
    pub fn keysym_raw(&self) -> u32 {
        self.keysym_raw
    }

    pub fn keysym(&self) -> KeySym {
        self.keysym
    }

    pub fn is_pressed(&self) -> bool {
        self.is_pressed
    }
}

bitflags! {
    pub struct MouseButtons: u8 {
        const LEFT = 1 << 0;
        const MIDDLE = 1 << 1;
        const RIGHT = 1 << 2;
        const SCROLL_A = 1 << 3;
        const SCROLL_B = 1 << 4;
        const SCROLL_C = 1 << 5;
        const SCROLL_D = 1 << 6;
    }
}

#[derive(Debug)]
pub struct PointerEvent {
    pub position: Position,
    pub pressed: MouseButtons,
}

impl ReadMessage for PointerEvent {
    fn read_from<'a>(stream: &'a mut TcpStream) -> BoxFuture<'a, Result<Self, ProtocolError>> {
        async {
            let button_mask = stream.read_u8().await?;
            let pressed = MouseButtons::from_bits_truncate(button_mask);
            let position = Position::read_from(stream).await?;

            Ok(PointerEvent { position, pressed })
        }
        .boxed()
    }
}
