// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use anyhow::{bail, Result};
use async_trait::async_trait;
use log::{debug, error, info, trace};
use std::marker::{Send, Sync};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::rfb::{
    ClientInit, ClientMessage, FramebufferUpdate, KeyEvent, PixelFormat, ProtoVersion, ReadMessage,
    SecurityResult, SecurityType, SecurityTypes, ServerInit, WriteMessage,
};

/// Immutable state
pub struct VncServerConfig {
    pub addr: SocketAddr,
    pub version: ProtoVersion,
    pub sec_types: SecurityTypes,
    pub name: String,
}

/// Mutable state
pub struct VncServerData {
    pub width: u16,
    pub height: u16,

    /// The pixel format of the framebuffer data passed in to the server via
    /// get_framebuffer_update.
    pub input_pixel_format: PixelFormat,
}

#[derive(Clone)]
pub struct VncServer<S: Server> {
    config: Arc<VncServerConfig>,
    data: Arc<Mutex<VncServerData>>,
    pub server: Arc<S>,
}

#[async_trait]
pub trait Server: Sync + Send + Clone + 'static {
    async fn get_framebuffer_update(&self) -> FramebufferUpdate;
    async fn keyevent(&self, _ke: KeyEvent) {}
}

impl<S: Server> VncServer<S> {
    pub fn new(server: S, config: VncServerConfig, data: VncServerData) -> Self {
        assert!(
            config.sec_types.0.len() > 0,
            "at least one security type must be defined"
        );
        Self {
            config: Arc::new(config),
            data: Arc::new(Mutex::new(data)),
            server: Arc::new(server),
        }
    }

    pub async fn set_pixel_format(&self, pixel_format: PixelFormat) {
        let mut locked = self.data.lock().await;
        locked.input_pixel_format = pixel_format;
    }

    pub async fn set_resolution(&self, width: u16, height: u16) {
        let mut locked = self.data.lock().await;
        locked.width = width;
        locked.height = height;
    }

    async fn rfb_handshake(&self, s: &mut TcpStream, addr: SocketAddr) -> Result<()> {
        // ProtocolVersion handshake
        info!("Tx [{:?}]: ProtoVersion={:?}", addr, self.config.version);
        self.config.version.write_to(s).await?;
        let client_version = ProtoVersion::read_from(s).await?;
        info!("Rx [{:?}]: ClientVersion={:?}", addr, client_version);

        if client_version < self.config.version {
            let err_str = format!(
                "[{:?}] unsupported client version={:?} (server version: {:?})",
                addr, client_version, self.config.version
            );
            error!("{}", err_str);
            bail!(err_str);
        }

        // Security Handshake
        let supported_types = self.config.sec_types.clone();
        info!("Tx [{:?}]: SecurityTypes={:?}", addr, supported_types);
        supported_types.write_to(s).await?;
        let client_choice = SecurityType::read_from(s).await?;
        info!("Rx [{:?}]: SecurityType Choice={:?}", addr, client_choice);
        if !self.config.sec_types.0.contains(&client_choice) {
            info!("Tx [{:?}]: SecurityResult=Failure", addr);
            let failure = SecurityResult::Failure("unsupported security type".to_string());
            failure.write_to(s).await?;
            let err_str = format!("invalid security choice={:?}", client_choice);
            error!("{}", err_str);
            bail!(err_str);
        }

        let res = SecurityResult::Success;
        info!("Tx: SecurityResult=Success");
        res.write_to(s).await?;

        Ok(())
    }

    async fn rfb_initialization(&self, s: &mut TcpStream, addr: SocketAddr) -> Result<()> {
        let client_init = ClientInit::read_from(s).await?;
        info!("Rx [{:?}]: ClientInit={:?}", addr, client_init);
        // TODO: decide what to do in exclusive case
        match client_init.shared {
            true => {}
            false => {}
        }

        let data = self.data.lock().await;
        let server_init = ServerInit::new(
            data.width,
            data.height,
            self.config.name.clone(),
            data.input_pixel_format.clone(),
        );
        info!("Tx [{:?}]: ServerInit={:#?}", addr, server_init);
        server_init.write_to(s).await?;

        Ok(())
    }

    async fn handle_conn(&self, s: &mut TcpStream, addr: SocketAddr) {
        info!("[{:?}] new connection", addr);

        if let Err(e) = self.rfb_handshake(s, addr).await {
            error!("[{:?}] could not complete handshake: {:?}", addr, e);
            return;
        }

        if let Err(e) = self.rfb_initialization(s, addr).await {
            error!("[{:?}] could not complete handshake: {:?}", addr, e);
            return;
        }

        let data = self.data.lock().await;
        let mut output_pixel_format = data.input_pixel_format.clone();
        drop(data);

        loop {
            let req = ClientMessage::read_from(s).await;

            match req {
                Ok(client_msg) => match client_msg {
                    ClientMessage::SetPixelFormat(pf) => {
                        debug!("Rx [{:?}]: SetPixelFormat={:#?}", addr, pf);

                        // TODO: invalid pixel formats?
                        output_pixel_format = pf;
                    }
                    ClientMessage::SetEncodings(e) => {
                        debug!("Rx [{:?}]: SetEncodings={:?}", addr, e);
                    }
                    ClientMessage::FramebufferUpdateRequest(f) => {
                        debug!("Rx [{:?}]: FramebufferUpdateRequest={:?}", addr, f);

                        let mut fbu = self.server.get_framebuffer_update().await;

                        let data = self.data.lock().await;

                        // We only need to change pixel formats if the client requested a different
                        // one than what's specified in the input.
                        //
                        // For now, we only support transformations between 4-byte RGB formats, so
                        // if the requested format isn't one of those, we'll just leave the pixels
                        // as is.
                        if data.input_pixel_format != output_pixel_format
                            && data.input_pixel_format.is_rgb_888()
                            && output_pixel_format.is_rgb_888()
                        {
                            debug!(
                                "transforming: input={:#?}, output={:#?}",
                                data.input_pixel_format, output_pixel_format
                            );
                            fbu = fbu.transform(&data.input_pixel_format, &output_pixel_format);
                        } else if !(data.input_pixel_format.is_rgb_888()
                            && output_pixel_format.is_rgb_888())
                        {
                            debug!("cannot transform between pixel formats (not rgb888): input.is_rgb_888()={}, output.is_rgb_888()={}", data.input_pixel_format.is_rgb_888(), output_pixel_format.is_rgb_888());
                        } else {
                            debug!("no input transformation needed");
                        }

                        if let Err(e) = fbu.write_to(s).await {
                            error!(
                                "[{:?}] could not write FramebufferUpdateRequest: {:?}",
                                addr, e
                            );
                            return;
                        }
                        debug!("Tx [{:?}]: FramebufferUpdate", addr);
                    }
                    ClientMessage::KeyEvent(ke) => {
                        trace!("Rx [{:?}]: KeyEvent={:?}", addr, ke);
                        self.server.keyevent(ke).await;
                    }
                    ClientMessage::PointerEvent(pe) => {
                        trace!("Rx [{:?}: PointerEvent={:?}", addr, pe);
                    }
                    ClientMessage::ClientCutText(t) => {
                        trace!("Rx [{:?}: ClientCutText={:?}", addr, t);
                    }
                },
                Err(e) => {
                    error!("[{:?}] error reading client message: {}", addr, e);
                    return;
                }
            }
        }
    }

    pub async fn start(&self) {
        let listener = TcpListener::bind(self.config.addr).await.unwrap();

        loop {
            let (mut s, a) = listener.accept().await.unwrap();
            let server = self.clone();
            tokio::spawn(async move {
                VncServer::handle_conn(&server, &mut s, a).await;
            });
        }
    }
}
