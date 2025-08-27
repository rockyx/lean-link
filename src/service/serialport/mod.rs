mod port;
mod group;

use std::time::Duration;

pub use port::*;
pub use group::*;
use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, StopBits};

/// 串口配置
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SerialPortConfig {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    #[serde(with = "duration_millis")]
    pub timeout: Duration,  
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
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