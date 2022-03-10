// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, error, info};
use std::marker::{Send, Sync};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::rfb::ClientMessage::{
    ClientCutText, FramebufferUpdateRequest, KeyEvent, PointerEvent, SetEncodings, SetPixelFormat,
};
use crate::rfb::{
    ClientInit, ClientMessage, FramebufferUpdate, ProtoVersion, ReadMessage, SecurityResult,
    SecurityType, SecurityTypes, ServerInit, WriteMessage,
};

#[allow(dead_code)]
pub struct VncServerConfig {
    pub addr: SocketAddr,
    pub version: ProtoVersion,
    pub sec_types: SecurityTypes,
    pub name: String,
}

#[allow(dead_code)]
pub struct VncServerData {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct VncServer<S: Server> {
    config: Arc<VncServerConfig>,
    data: Arc<Mutex<VncServerData>>,
    pub server: Arc<S>,
}

#[async_trait]
pub trait Server: Sync + Send + Clone + 'static {
    async fn get_framebuffer_update(&self) -> FramebufferUpdate;
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

    async fn rfb_handshake(&self, s: &mut TcpStream) -> Result<()> {
        // ProtocolVersion handshake
        info!("Tx: ProtoVersion={:?}", self.config.version);
        self.config.version.write_to(s).await?;
        let client_version = ProtoVersion::read_from(s).await?;
        info!("Rx: ClientVersion={:?}", client_version);

        if client_version < self.config.version {
            let err_str = format!(
                "unsupported client version: {:?} (server version: {:?})",
                client_version, self.config.version
            );
            error!("{}", err_str);
            return Err(anyhow!(err_str));
        }

        // Security Handshake
        let supported_types = self.config.sec_types.clone();
        info!("Tx: SecurityTypes={:?}", supported_types);
        supported_types.write_to(s).await?;
        let client_choice = SecurityType::read_from(s).await?;
        info!("Rx: SecurityType Choice={:?}", client_choice);
        if !self.config.sec_types.0.contains(&client_choice) {
            info!("Tx: SecurityResult=Failure");
            let failure = SecurityResult::Failure("unsupported security type".to_string());
            failure.write_to(s).await?;
            // TODO: close the connection
            let err_str = format!("invalid security choice={:?}", client_choice);
            error!("{}", err_str);
            return Err(anyhow!(err_str));
        }

        let res = SecurityResult::Success;
        info!("Tx: SecurityResult=Success");
        res.write_to(s).await?;

        Ok(())
    }

    async fn rfb_initialization(&self, s: &mut TcpStream) -> Result<()> {
        let client_init = ClientInit::read_from(s).await?;
        info!("Rx: ClientInit={:?}", client_init);
        // TODO: decide what to do in exclusive case
        match client_init.shared {
            true => {}
            false => {}
        }

        let data = self.data.lock().await;
        let server_init = ServerInit::new(data.width, data.height, self.config.name.clone());
        info!("Tx: ServerInit={:?}", server_init);
        server_init.write_to(s).await?;

        Ok(())
    }

    async fn handle_conn(&self, s: &mut TcpStream) {
        if let Err(e) = self.rfb_handshake(s).await {
            // TODO: client debugging information.
            error!("could not complete handshake: {:?}", e);
            return;
        }

        if let Err(e) = self.rfb_initialization(s).await {
            // TODO: client debugging information.
            error!("could not complete initialization: {:?}", e);
            return;
        }

        loop {
            let req = ClientMessage::read_from(s).await;

            match req {
                Ok(client_msg) => match client_msg {
                    SetPixelFormat(pf) => {
                        debug!("Rx: SetPixelFormat={:?}", pf);
                    }
                    SetEncodings(e) => {
                        debug!("Rx: SetEncodings={:?}", e);
                    }
                    FramebufferUpdateRequest(f) => {
                        debug!("Rx: FramebufferUpdateRequest={:?}", f);
                        let fbu = self.server.get_framebuffer_update().await;
                        if let Err(e) = fbu.write_to(s).await {
                            error!("could not write FramebufferUpdateRequest: {:?}", e);
                            return;
                        }
                        debug!("Tx: FramebufferUpdate");
                    }
                    KeyEvent(ke) => {
                        debug!("Rx: KeyEvent={:?}", ke);
                    }
                    PointerEvent(pe) => {
                        debug!("Rx: PointerEvent={:?}", pe);
                    }
                    ClientCutText(t) => {
                        debug!("Rx: ClientCutText={:?}", t);
                    }
                },
                Err(e) => {
                    error!("error reading client message: {}", e);
                    return;
                }
            }
        }
    }

    pub async fn start(&self) {
        let listener = TcpListener::bind(self.config.addr).await.unwrap();

        loop {
            let (mut s, _a) = listener.accept().await.unwrap();
            let server = self.clone();
            tokio::spawn(async move {
                VncServer::handle_conn(&server, &mut s).await;
            });
        }
    }
}
