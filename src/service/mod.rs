#[cfg(feature = "modbus")]
pub mod modbus;
#[cfg(feature = "mqtt")]
pub mod mqtt;
#[cfg(feature = "serialport")]
pub mod serialport;
#[cfg(feature = "web")]
pub mod web;
#[cfg(any(feature = "web", feature = "websocket"))]
pub mod websocket;
#[cfg(any(feature = "imv-camera"))]
pub mod camera;
#[cfg(feature = "socket")]
pub mod socket;
