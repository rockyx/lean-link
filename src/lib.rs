use crate::{
    config::ServerConfig,
    service::websocket::{WebSocketMessage, WebSocketServer},
};
use sea_orm::{Database, DatabaseConnection};
use tokio::sync::mpsc::Receiver;

// Base re-export
pub use anyhow;
pub use bcrypt;
pub use bytes;
pub use chrono;
pub use crc;
pub use dashmap;
pub use directories;
pub use normpath;
pub use rand;
pub use regex;
pub use rust_decimal;
pub use rust_decimal_macros;
pub use serde_json;
pub use serde_yaml;
pub use smallvec;
pub use thiserror;
pub use tsink;

// Tracing re-export
pub use tracing;
pub use tracing_subscriber;

// Tokio re-export
pub use async_trait;
pub use futures;
pub use futures_util;
pub use tokio;
pub use tokio_cron_scheduler;
pub use tokio_retry2;
pub use tokio_stream;

// Database re-export
pub use sea_orm;
pub use sea_orm_migration;

// UUID re-export
pub use uuid;

// Atix re-export
#[cfg(feature = "web")]
pub use actix_cors;
#[cfg(feature = "web")]
pub use actix_utils;
#[cfg(feature = "web")]
pub use jsonwebtoken;
#[cfg(any(feature = "web", feature = "socket"))]
pub use tokio_tungstenite;
#[cfg(feature = "web")]
pub use tracing_actix_web;

// Mqtt re-export
#[cfg(feature = "mqtt")]
pub use rumqttc;

// SerialPort re-export
#[cfg(feature = "serialport")]
pub use serialport;
#[cfg(feature = "serialport")]
pub use tokio_serial;

// Modbus re-export
#[cfg(feature = "modbus")]
pub use tokio_modbus;

// Platform re-export
#[cfg(target_os = "windows")]
pub use winapi;

pub mod config;
pub mod database;
pub mod errors;
pub mod ffi;
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
            use crate::utils::datetime::set_local_time_from_ds1307;
            use crate::utils::i2c::path_to_i2c_bus;

            let sync_time_from_rtc = server_config.sys.sync_time_from_rtc;
            let rtc_i2c_dev = server_config.sys.rtc_i2c_dev.clone();
            let rtc_i2c_addr = server_config.sys.rtc_i2c_addr;
            if sync_time_from_rtc {
                // sync system time from RTC
                let bus_result = path_to_i2c_bus(&rtc_i2c_dev);
                if bus_result.is_err() {
                    tracing::info!("syncSysTime command output: {:?}", bus_result);
                } else {
                    let bus = bus_result.unwrap();
                    let output = set_local_time_from_ds1307(bus, rtc_i2c_addr);
                    tracing::info!("syncSysTime command output: {:?}", output);
                }
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
