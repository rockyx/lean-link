use std::time::Duration;

pub use group::*;
pub use port::*;
use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, StopBits};

mod group;
mod port;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortConfig {
    pub path: String,
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
    #[serde(default = "default_data_bits")]
    pub data_bits: DataBits,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: StopBits,
    #[serde(default = "default_parity")]
    pub parity: Parity,
    #[serde(default = "default_flow_control")]
    pub flow_control: FlowControl,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub timeout: Duration,
}

fn default_baud_rate() -> u32 {
    9600
}

fn default_data_bits() -> DataBits {
    DataBits::Eight
}

fn default_flow_control() -> FlowControl {
    FlowControl::None
}

fn default_parity() -> Parity {
    Parity::None
}

fn default_stop_bits() -> StopBits {
    StopBits::One
}

#[cfg(feature = "serialport")]
impl Default for SerialPortConfig {
    fn default() -> Self {
        SerialPortConfig {
            path: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
            timeout: Duration::from_secs(1),
        }
    }
}
