use std::process::Command;

use crate::{
    config::ServerConfig,
    service::websocket::{WebSocketMessage, WebSocketServer},
};
use sea_orm::{Database, DatabaseConnection};
use tokio::sync::mpsc::Receiver;

pub mod config;
pub mod crc;
pub mod cron;
pub mod database;
pub mod errors;
pub mod service;
pub mod storage;
pub mod utils;

// pub use lean_link_macros::*;

pub struct AppState {
    pub db_conn: DatabaseConnection,
    pub server_config: ServerConfig,
    pub server_name: String,
    #[cfg(feature = "web")]
    pub ws_server: WebSocketServer,
}

impl AppState {
    pub async fn new(server_name: &str) -> std::io::Result<Self> {
        let server_config = config::load_config(server_name)?;
        let db_conn = Database::connect(server_config.database.url.clone())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        #[cfg(feature = "web")]
        let web_socket_server =
            WebSocketServer::new(server_config.web_socket.clone(), server_config.sys.clone());

        #[cfg(target_os = "linux")]
        {
            let sync_time_from_client = server_config.sys.sync_time_from_client;
            let rtc_i2c_dev = server_config.sys.rtc_i2c_dev.clone();
            if sync_time_from_client {
                // sync system time from RTC
                // sudo hwclock -s -f /dev/i2c-1
                let output = Command::new("sudo")
                    .arg("hwclock")
                    .arg("-s")
                    .arg("-f")
                    .arg(rtc_i2c_dev)
                    .output();
                tracing::info!("syncSysTime command output: {:?}", output);
            }
        }

        Ok(Self {
            db_conn,
            server_config,
            server_name: server_name.to_string(),
            #[cfg(feature = "web")]
            ws_server: web_socket_server,
        })
    }

    #[cfg(feature = "web")]
    pub async fn start_web_socket(&self) -> std::io::Result<Receiver<WebSocketMessage>> {
        self.ws_server.start().await
    }
}

pub type Result<T> = std::result::Result<T, errors::Error>;
