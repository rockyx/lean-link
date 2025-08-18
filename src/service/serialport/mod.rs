use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

use crate::service::serialport::inner::SerialPortInner;

mod holder;
mod inner;

/// 串口配置
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortConfig {
    /// 串口路径
    pub path: String,
    /// 波特率
    pub baud_rate: u32,
    /// 数据位
    pub data_bits: DataBits,
    /// 流控制
    pub flow_control: FlowControl,
    /// 校验位
    pub parity: Parity,
    /// 停止位
    pub stop_bits: StopBits,
    /// 超时时间
    #[serde(with = "duration_millis")]
    pub timeout: Duration,
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[derive(Clone, Debug)]
pub enum FromPortEvent {
    Ack {},
    Timeout {},
    Close {},
}

#[derive(Clone, Debug)]
pub enum ToPortEvent {
    Write { data: bytes::Bytes, need_ack: bool },
    Ack {},
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

pub struct SerialPortOption {
    pub name: String,
    pub configs: Vec<SerialPortConfig>,
    pub data_parser: DataParser,
    pub frame_handler: FrameHandler,
}

#[derive(Clone)]
pub struct SerialPortDevice {
    inner_map: Arc<RwLock<HashMap<String, SerialPortInner>>>,
}

impl SerialPortDevice {
    pub fn new() -> Self {
        Self {
            inner_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_serialport(&self, option: SerialPortOption) {
        let mut inner_map = self.inner_map.write().await;

        if inner_map.contains_key(&option.name) {
            inner_map.remove(&option.name);
        }

        let inner = SerialPortInner::new(&option.configs, option.data_parser, option.frame_handler);
        inner_map.insert(option.name, inner);
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
