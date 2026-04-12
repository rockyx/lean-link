#[cfg(any(feature = "industry-camera", feature = "inspection"))]
pub mod camera;
#[cfg(feature = "inspection")]
pub mod inspection;
#[cfg(feature = "modbus")]
pub mod modbus;
#[cfg(feature = "mqtt")]
pub mod mqtt;
#[cfg(feature = "serialport")]
pub mod serialport;
#[cfg(feature = "socket")]
pub mod socket;
#[cfg(feature = "web")]
pub mod web;
#[cfg(any(feature = "web", feature = "websocket"))]
pub mod websocket;
