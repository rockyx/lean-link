use crate::database::entity::t_serialport_configs;
use actix_web::scope;
use sea_orm::ActiveValue;
use serde::{Deserialize, Serialize};
use serialport::SerialPortType;
use uuid::Uuid;

/// 枚举到的串口信息 (扁平化结构, 与前端 Dart 模型匹配)
#[derive(Serialize, Deserialize, Debug)]
pub struct SerialPortInfoResponse {
    pub name: String,
    #[serde(rename = "type")]
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub vid: Option<String>,
    pub pid: Option<String>,
}

impl From<serialport::SerialPortInfo> for SerialPortInfoResponse {
    fn from(info: serialport::SerialPortInfo) -> Self {
        let (port_type, manufacturer, product, serial_number, vid, pid) = match info.port_type {
            SerialPortType::UsbPort(usb) => (
                "UsbPort".to_string(),
                usb.manufacturer,
                usb.product,
                usb.serial_number,
                Some(format!("{:04x}", usb.vid)),
                Some(format!("{:04x}", usb.pid)),
            ),
            SerialPortType::PciPort => ("PciPort".to_string(), None, None, None, None, None),
            SerialPortType::BluetoothPort => {
                ("BluetoothPort".to_string(), None, None, None, None, None)
            }
            SerialPortType::Unknown => ("Unknown".to_string(), None, None, None, None, None),
        };
        Self {
            name: info.port_name,
            port_type,
            manufacturer,
            product,
            serial_number,
            vid,
            pid,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SerialportConfigCreateRequest {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: String,
    pub stop_bits: String,
    pub parity: String,
    pub flow_control: String,
    #[cfg(not(feature = "sqlite"))]
    pub timeout_ms: u64,
    #[cfg(feature = "sqlite")]
    pub timeout_ms: i64,
}

impl From<SerialportConfigCreateRequest> for t_serialport_configs::ActiveModel {
    fn from(req: SerialportConfigCreateRequest) -> Self {
        t_serialport_configs::ActiveModel {
            id: ActiveValue::set(Uuid::now_v7()),
            path: ActiveValue::set(req.path),
            baud_rate: ActiveValue::set(req.baud_rate),
            data_bits: ActiveValue::set(req.data_bits),
            stop_bits: ActiveValue::set(req.stop_bits),
            parity: ActiveValue::set(req.parity),
            flow_control: ActiveValue::set(req.flow_control),
            timeout_ms: ActiveValue::set(req.timeout_ms),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerialportConfigUpdateRequest {
    pub path: Option<String>,
    pub baud_rate: Option<u32>,
    pub data_bits: Option<String>,
    pub stop_bits: Option<String>,
    pub parity: Option<String>,
    pub flow_control: Option<String>,
    #[cfg(not(feature = "sqlite"))]
    pub timeout_ms: Option<u64>,
    #[cfg(feature = "sqlite")]
    pub timeout_ms: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct SerialportConfigListRequest {
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[scope("/serialport")]
pub mod api {
    use crate::{
        AppState,
        database::{entity::t_serialport_configs, serialport_configs},
        service::web::service::{
            ErrorCode, Pagination, WebResponse,
            serialport::{
                SerialportConfigCreateRequest, SerialportConfigListRequest,
                SerialportConfigUpdateRequest,
            },
        },
    };
    use actix_web::{delete, get, post, put, web};
    use sea_orm::ActiveValue;
    use uuid::Uuid;

    #[post("/create")]
    async fn create(
        app_state: web::Data<AppState>,
        req: web::Json<SerialportConfigCreateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<t_serialport_configs::Model>>, crate::errors::Error>
    {
        let db_conn = &app_state.db_conn;

        let active_model: t_serialport_configs::ActiveModel = req.into_inner().into();

        match serialport_configs::insert_serialport_config(db_conn, active_model).await {
            Ok(result) => {
                let config = serialport_configs::find_serialport_config_by_id(
                    db_conn,
                    result.last_insert_id,
                )
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e);
                    crate::errors::Error::InternalError(ErrorCode::InternalError)
                })?;

                match config {
                    Some(c) => Ok(WebResponse::with_result(c).into()),
                    None => Err(crate::errors::Error::InternalError(
                        ErrorCode::InternalError,
                    )),
                }
            }
            Err(e) => {
                tracing::error!(error = ?e);
                Err(crate::errors::Error::DbErr(e))
            }
        }
    }

    #[put("/update/{id}")]
    async fn update(
        app_state: web::Data<AppState>,
        path: web::Path<Uuid>,
        req: web::Json<SerialportConfigUpdateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<t_serialport_configs::Model>>, crate::errors::Error>
    {
        let db_conn = &app_state.db_conn;
        let id = path.into_inner();

        let existing = match serialport_configs::find_serialport_config_by_id(db_conn, id).await {
            Ok(Some(model)) => model,
            Ok(None) => {
                return Err(crate::errors::Error::BadRequest(
                    ErrorCode::NotFound,
                    "配置没找到".into(),
                ));
            }
            Err(e) => {
                tracing::error!(error = ?e);
                return Err(crate::errors::Error::DbErr(e));
            }
        };

        let active_model = t_serialport_configs::ActiveModel {
            id: ActiveValue::set(id),
            path: req
                .path
                .as_ref()
                .map_or(ActiveValue::set(existing.path), |v| {
                    ActiveValue::set(v.clone())
                }),
            baud_rate: req
                .baud_rate
                .map_or(ActiveValue::set(existing.baud_rate), ActiveValue::set),
            data_bits: req
                .data_bits
                .as_ref()
                .map_or(ActiveValue::set(existing.data_bits), |v| {
                    ActiveValue::set(v.clone())
                }),
            stop_bits: req
                .stop_bits
                .as_ref()
                .map_or(ActiveValue::set(existing.stop_bits), |v| {
                    ActiveValue::set(v.clone())
                }),
            parity: req
                .parity
                .as_ref()
                .map_or(ActiveValue::set(existing.parity), |v| {
                    ActiveValue::set(v.clone())
                }),
            flow_control: req
                .flow_control
                .as_ref()
                .map_or(ActiveValue::set(existing.flow_control), |v| {
                    ActiveValue::set(v.clone())
                }),
            timeout_ms: req
                .timeout_ms
                .map_or(ActiveValue::set(existing.timeout_ms), ActiveValue::set),
        };

        match serialport_configs::update_serialport_config(db_conn, id, active_model).await {
            Ok(Some(model)) => Ok(WebResponse::with_result(model).into()),
            Ok(None) => Err(crate::errors::Error::InternalError(
                ErrorCode::OperationNotAllow,
            )),
            Err(e) => {
                tracing::error!(error = ?e);
                Err(crate::errors::Error::DbErr(e))
            }
        }
    }

    #[delete("/delete/{id}")]
    async fn delete(
        app_state: web::Data<AppState>,
        path: web::Path<Uuid>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, crate::errors::Error> {
        let db_conn = &app_state.db_conn;
        let id = path.into_inner();

        match serialport_configs::delete_serialport_config(db_conn, id).await {
            Ok(true) => Ok(WebResponse::with_result(()).into()),
            Ok(false) => Err(crate::errors::Error::InternalError(
                ErrorCode::OperationNotAllow,
            )),
            Err(e) => {
                tracing::error!(error = ?e);
                Err(crate::errors::Error::DbErr(e))
            }
        }
    }

    #[get("/get/{id}")]
    async fn get(
        app_state: web::Data<AppState>,
        path: web::Path<Uuid>,
    ) -> actix_web::Result<web::Json<WebResponse<t_serialport_configs::Model>>, crate::errors::Error>
    {
        let db_conn = &app_state.db_conn;
        let id = path.into_inner();

        match serialport_configs::find_serialport_config_by_id(db_conn, id).await {
            Ok(Some(model)) => Ok(WebResponse::with_result(model).into()),
            Ok(None) => Err(crate::errors::Error::InternalError(
                ErrorCode::OperationNotAllow,
            )),
            Err(e) => {
                tracing::error!(error = ?e);
                Err(crate::errors::Error::DbErr(e))
            }
        }
    }

    #[get("/list")]
    async fn list(
        app_state: web::Data<AppState>,
        query: web::Query<SerialportConfigListRequest>,
    ) -> actix_web::Result<
        web::Json<WebResponse<Pagination<t_serialport_configs::Model>>>,
        crate::errors::Error,
    > {
        let db_conn = &app_state.db_conn;
        let page = query.page.unwrap_or(1);
        let size = query.size.unwrap_or(10);

        match serialport_configs::page_serialport_configs(db_conn, page, size).await {
            Ok(page_result) => {
                let pagination: Pagination<t_serialport_configs::Model> = Pagination {
                    records: page_result.records,
                    total: page_result.total_count,
                    current: page_result.page_index,
                    size: page_result.page_size,
                    pages: page_result.pages,
                };
                Ok(WebResponse::with_result(pagination).into())
            }
            Err(e) => {
                tracing::error!(error = ?e);
                Err(crate::errors::Error::DbErr(e))
            }
        }
    }

    #[get("/enumerate")]
    pub async fn enumerate_serial_ports() -> actix_web::Result<
        web::Json<WebResponse<Vec<super::SerialPortInfoResponse>>>,
        crate::errors::Error,
    > {
        let ports =
            serialport::available_ports().map_err(|e| crate::errors::Error::Io(e.into()))?;
        let result: Vec<super::SerialPortInfoResponse> =
            ports.into_iter().map(|p| p.into()).collect();
        Ok(WebResponse::with_result(result).into())
    }
}
