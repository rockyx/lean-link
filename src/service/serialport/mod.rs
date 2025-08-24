use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio::sync::mpsc::{Receiver, Sender};

pub use crate::service::serialport::holder::SerialPortConfig;
pub use crate::service::serialport::inner::{
    HeartbeatCommandBuilder, SerialPortInner, SerialPortInnerBuilder,
};

mod holder;
mod inner;

#[derive(Clone, Debug)]
pub enum FromPortEvent {
    Ack {},
    Timeout {},
    Close {},
    HeartbeatAck {},
    HeartbeatTimeout {},
}

#[derive(Clone, Debug)]
pub enum ToPortEvent {
    Write { data: bytes::Bytes, need_ack: bool },
    Ack {},
    Heartbeat {},
    Stop {},
}

#[derive(Clone, Debug)]
pub enum DataParserEvent {
    Data { data: bytes::Bytes },
    Close {},
}

pub type DataParser =
    Box<dyn Fn(Sender<FrameHandlerEvent>, Receiver<DataParserEvent>) + Send + Sync>;

#[derive(Clone, Debug)]
pub enum FrameHandlerEvent {
    Data { data: bytes::Bytes },
    Close {},
}

pub type FrameHandler = Box<dyn Fn(Sender<ToPortEvent>, Receiver<FrameHandlerEvent>) + Send + Sync>;

#[derive(Clone)]
pub struct SerialPortService {
    inner_map: Arc<RwLock<HashMap<String, SerialPortInner>>>,
}

impl SerialPortService {
    pub fn new() -> Self {
        Self {
            inner_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_serialport(&self, name: &str, inner: SerialPortInner) {
        let mut inner_map = self.inner_map.write().await;

        if inner_map.contains_key(name) {
            inner_map.remove(name);
        }

        inner_map.insert(name.to_string(), inner);
    }

    pub async fn add_serialport_config(
        &self,
        name: &str,
        config: &SerialPortConfig,
    ) -> std::io::Result<()> {
        let inner_map = self.inner_map.read().await;

        if let Some(inner) = inner_map.get(name) {
            inner.add_config(config).await;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "SerialPort not found",
            ))
        }
    }

    pub async fn remove_serialport_config(
        &self,
        name: &str,
        config: &SerialPortConfig,
    ) -> std::io::Result<()> {
        let inner_map = self.inner_map.read().await;

        if let Some(inner) = inner_map.get(name) {
            inner.remove_config(config).await;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "SerialPort not found",
            ))
        }
    }

    pub fn available_ports() -> Vec<String> {
        serialport::available_ports()
            .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
            .unwrap_or_default()
    }

    pub async fn write(&self, name: &str, data: &bytes::Bytes) -> std::io::Result<usize> {
        let inner_map = self.inner_map.read().await;

        if let Some(inner) = inner_map.get(name) {
            inner.write(data).await
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "SerialPort not found",
            ))
        }
    }

    pub async fn write_need_ack(&self, name: &str, data: &bytes::Bytes) -> std::io::Result<usize> {
        let inner_map = self.inner_map.read().await;

        if let Some(inner) = inner_map.get(name) {
            inner.write_need_ack(data).await
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "SerialPort not found",
            ))
        }
    }
}
