use crate::{
    config::ServerConfig,
    service::websocket::{WebSocketMessage, WebSocketServer},
};
use sea_orm::{Database, DatabaseConnection};
use tokio::sync::mpsc::Receiver;

// Base re-export
pub use bcrypt as bcrypt;
pub use bytes as bytes;
pub use chrono as chrono;
pub use serde_json as serde_json;
pub use smallvec as smallvec;
pub use thiserror as thiserror;
pub use directories as directories;
pub use serde_yaml as serde_yaml;
pub use normpath as normpath;
pub use anyhow as anyhow;
pub use rand as rand;
pub use regex as regex;
pub use tsink as tsink;
pub use crc as crc;
pub use rust_decimal as rust_decimal;
pub use rust_decimal_macros as rust_decimal_macros;
pub use dashmap as dashmap;

// Tracing re-export
pub use tracing as tracing;
pub use tracing_subscriber as tracing_subscriber;

// Tokio re-export
pub use tokio as tokio;
pub use async_trait as async_trait;
pub use futures as futures;
pub use futures_util as futures_util;
pub use tokio_retry2 as tokio_retry2;
pub use tokio_stream as tokio_stream;
pub use tokio_cron_scheduler as tokio_cron_scheduler;

// Database re-export
pub use sea_orm as sea_orm;
pub use sea_orm_migration as sea_orm_migration;

// UUID re-export
pub use uuid as uuid;

// Atix re-export
#[cfg(feature = "web")]
pub use tracing_actix_web as tracing_actix_web;
#[cfg(feature = "web")]
pub use actix_cors as actix_cors;
#[cfg(feature = "web")]
pub use actix_utils as actix_utils;
#[cfg(feature = "web")]
pub use jsonwebtoken as jsonwebtoken;
#[cfg(any(feature = "web", feature = "socket"))]
pub use tokio_tungstenite as tokio_tungstenite;



// Mqtt re-export
#[cfg(feature = "mqtt")]
pub use rumqttc as rumqttc;

// SerialPort re-export
#[cfg(feature = "serialport")]
pub use serialport as serialport;
#[cfg(feature = "serialport")]
pub use tokio_serial as tokio_serial;

// Modbus re-export
#[cfg(feature = "modbus")]
pub use tokio_modbus as tokio_modbus;

// Platform re-export
#[cfg(target_os = "windows")]
pub use winapi as winapi;


pub mod config;
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
            use crate::utils::i2c::path_to_i2c_bus;
            use crate::utils::datetime::set_local_time_from_ds1307;

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
