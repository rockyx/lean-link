use crate::database::entity::t_modbus_configs;
use actix_web::scope;
use sea_orm::ActiveValue;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ModbusConfigCreateRequest {
    #[serde(rename = "type")]
    pub config_type: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub slave_id: u8,
    pub serialport_id: Option<Uuid>,
    pub name: Option<String>,
    pub enabled: bool,
}

impl From<ModbusConfigCreateRequest> for t_modbus_configs::ActiveModel {
    fn from(req: ModbusConfigCreateRequest) -> Self {
        t_modbus_configs::ActiveModel {
            id: ActiveValue::set(Uuid::now_v7()),
            r#type: ActiveValue::set(req.config_type),
            host: ActiveValue::set(req.host),
            port: ActiveValue::set(req.port),
            slave_id: ActiveValue::set(req.slave_id),
            serialport_id: ActiveValue::set(req.serialport_id),
            name: ActiveValue::set(req.name),
            enabled: ActiveValue::set(req.enabled),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModbusConfigUpdateRequest {
    #[serde(rename = "type")]
    pub config_type: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub slave_id: Option<u8>,
    pub serialport_id: Option<Uuid>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModbusConfigResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub config_type: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub slave_id: u8,
    pub serialport_id: Option<Uuid>,
    pub name: Option<String>,
    pub enabled: bool,
}

impl From<t_modbus_configs::Model> for ModbusConfigResponse {
    fn from(model: t_modbus_configs::Model) -> Self {
        Self {
            id: model.id,
            config_type: model.r#type,
            host: model.host,
            port: model.port,
            slave_id: model.slave_id,
            serialport_id: model.serialport_id,
            name: model.name,
            enabled: model.enabled,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ModbusConfigListRequest {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub enabled: Option<bool>,
}

#[scope("/modbus")]
pub mod api {
    use crate::{
        AppState,
        database::{entity::t_modbus_configs, modbus_configs},
        service::web::service::{
            ErrorCode, Pagination, WebResponse,
            modbus::{
                ModbusConfigCreateRequest, ModbusConfigListRequest, ModbusConfigResponse,
                ModbusConfigUpdateRequest,
            },
        },
    };
    use actix_web::{delete, get, post, put, web};
    use sea_orm::ActiveValue;
    use uuid::Uuid;

    #[post("/create")]
    async fn create(
        app_state: web::Data<AppState>,
        req: web::Json<ModbusConfigCreateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<ModbusConfigResponse>>, crate::errors::Error> {
        let db_conn = &app_state.db_conn;

        let active_model: t_modbus_configs::ActiveModel = req.into_inner().into();

        match modbus_configs::insert_modbus_config(db_conn, active_model).await {
            Ok(result) => {
                let config =
                    modbus_configs::find_modbus_config_by_id(db_conn, result.last_insert_id)
                        .await
                        .map_err(|e| {
                            tracing::error!(error = ?e);
                            crate::errors::Error::InternalError(ErrorCode::InternalError)
                        })?;

                match config {
                    Some(c) => Ok(WebResponse::with_result(c.into()).into()),
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
        req: web::Json<ModbusConfigUpdateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<ModbusConfigResponse>>, crate::errors::Error> {
        let db_conn = &app_state.db_conn;
        let id = path.into_inner();

        let existing = match modbus_configs::find_modbus_config_by_id(db_conn, id).await {
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

        let active_model = t_modbus_configs::ActiveModel {
            id: ActiveValue::set(id),
            r#type: req
                .config_type
                .as_ref()
                .map_or(ActiveValue::set(existing.r#type), |v| {
                    ActiveValue::set(v.clone())
                }),
            host: if req.host.is_some() {
                ActiveValue::set(req.host.clone())
            } else {
                ActiveValue::set(existing.host)
            },
            port: if req.port.is_some() {
                ActiveValue::set(req.port)
            } else {
                ActiveValue::set(existing.port)
            },
            slave_id: req
                .slave_id
                .map_or(ActiveValue::set(existing.slave_id), ActiveValue::set),
            serialport_id: if req.serialport_id.is_some() {
                ActiveValue::set(req.serialport_id)
            } else {
                ActiveValue::set(existing.serialport_id)
            },
            name: if req.name.is_some() {
                ActiveValue::set(req.name.clone())
            } else {
                ActiveValue::set(existing.name)
            },
            enabled: req
                .enabled
                .map_or(ActiveValue::set(existing.enabled), ActiveValue::set),
        };

        match modbus_configs::update_modbus_config(db_conn, id, active_model).await {
            Ok(Some(model)) => Ok(WebResponse::with_result(model.into()).into()),
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

        match modbus_configs::delete_modbus_config(db_conn, id).await {
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
    ) -> actix_web::Result<web::Json<WebResponse<ModbusConfigResponse>>, crate::errors::Error> {
        let db_conn = &app_state.db_conn;
        let id = path.into_inner();

        match modbus_configs::find_modbus_config_by_id(db_conn, id).await {
            Ok(Some(model)) => Ok(WebResponse::with_result(model.into()).into()),
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
        query: web::Query<ModbusConfigListRequest>,
    ) -> actix_web::Result<
        web::Json<WebResponse<Pagination<ModbusConfigResponse>>>,
        crate::errors::Error,
    > {
        let db_conn = &app_state.db_conn;
        let page = query.page.unwrap_or(1);
        let size = query.size.unwrap_or(10);

        match modbus_configs::page_modbus_configs(db_conn, page, size).await {
            Ok(page_result) => {
                let pagination: Pagination<ModbusConfigResponse> = Pagination {
                    records: page_result.records.into_iter().map(|m| m.into()).collect(),
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
}
