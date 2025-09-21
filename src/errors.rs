#[cfg(feature = "web")]
use crate::service::web::service::{ErrorCode, WebResponse};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// IO Error
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "modbus")]
    #[error("Modbus Error: {0}")]
    Modbus(#[from] tokio_modbus::Error),
    #[error("Modbus Exception Code: {0}")]
    #[cfg(feature = "modbus")]
    ModbusExceptionCode(#[from] tokio_modbus::ExceptionCode),
    #[cfg(feature = "web")]
    #[error("JWT Error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[cfg(feature = "web")]
    #[error("Missing Token")]
    MissingToken,
    #[error("Database Error: {0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error("Json Error: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "web")]
    #[error("Authorization Fail")]
    AuthorizationFail(ErrorCode),
    #[cfg(feature = "web")]
    #[error("Internal Error")]
    InternalError(ErrorCode),
    #[error("TSink Error: {0}")]
    Tsink(#[from] tsink::TsinkError),
    #[error("Configure Error")]
    Configure,
}

#[cfg(feature = "web")]
impl actix_web::error::ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            Error::Jwt(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::MissingToken => actix_web::http::StatusCode::UNAUTHORIZED,
            #[cfg(feature = "web")]
            Error::AuthorizationFail(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            #[cfg(feature = "web")]
            Error::InternalError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            Error::Jwt(err) => actix_web::HttpResponse::Unauthorized().body(format!("{}", err)),
            Error::MissingToken => {
                actix_web::HttpResponse::new(actix_web::http::StatusCode::UNAUTHORIZED)
            }
            #[cfg(feature = "web")]
            Error::AuthorizationFail(code) => {
                actix_web::HttpResponse::build(actix_web::http::StatusCode::UNAUTHORIZED)
                    .json(WebResponse::<()>::with_error_code(code))
            }
            #[cfg(feature = "web")]
            Error::InternalError(code) => {
                actix_web::HttpResponse::build(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .json(WebResponse::<()>::with_error_code(code))
            }
            _ => actix_web::HttpResponse::new(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
