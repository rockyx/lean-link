use std::time::Duration;

pub use group::*;
pub use port::*;
use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, StopBits};

use crate::database::entity::t_serialport_configs;

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

impl From<t_serialport_configs::Model> for SerialPortConfig {
    fn from(value: t_serialport_configs::Model) -> Self {
        Self {
            path: value.path.clone(),
            baud_rate: value.baud_rate,
            data_bits: value.data_bits_enum().unwrap_or(DataBits::Eight),
            stop_bits: value.stop_bits_enum().unwrap_or(StopBits::One),
            parity: value.parity_enum().unwrap_or(Parity::None),
            flow_control: value.flow_control_enum().unwrap_or(FlowControl::None),
            timeout: value.timeout(),
        }
    }
}
