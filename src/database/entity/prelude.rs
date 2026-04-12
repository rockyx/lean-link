pub use super::t_users::Entity as TUsers;
pub use super::t_settings::Entity as TSettings;
pub use super::t_logs::Entity as TLogs;

#[cfg(feature = "inspection")]
pub use super::t_defect_details::Entity as TDefectDetails;
#[cfg(feature = "inspection")]
pub use super::t_geometry_measurements::Entity as TGeometryMeasurements;
#[cfg(feature = "inspection")]
pub use super::t_inspection_details::Entity as TInspectionDetails;
#[cfg(feature = "inspection")]
pub use super::t_inspection_records::Entity as TInspectionRecords;
#[cfg(feature = "inspection")]
pub use super::t_inspection_statistics::Entity as TInspectionStatistics;

#[cfg(feature = "modbus")]
pub use super::t_modbus_configs::Entity as TModbusConfigs;
#[cfg(feature = "serialport")]
pub use super::t_serialport_configs::Entity as TSerialportConfigs;
#[cfg(feature = "industry-camera")]
pub use super::t_camera_configs::Entity as TCameraConfigs;