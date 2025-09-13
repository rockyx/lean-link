#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// IO Error
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "modbus")]
    #[error("Modbus Error: {0}")]
    Modbus(#[from] tokio_modbus::Error),
    #[error("Modbus Exception Code: {0}")]
    ModbusExceptionCode(#[from] tokio_modbus::ExceptionCode),
    #[cfg(feature = "web")]
    #[error("JWT Error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[cfg(feature = "web")]
    #[error("Missing Token")]
    MissingToken,
}

#[cfg(feature = "web")]
impl actix_web::error::ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            Error::Jwt(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::MissingToken => actix_web::http::StatusCode::UNAUTHORIZED,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            Error::Jwt(err) => actix_web::HttpResponse::Unauthorized().body(format!("{}", err)),
            Error::MissingToken => {
                actix_web::HttpResponse::new(actix_web::http::StatusCode::UNAUTHORIZED)
            }
            _ => actix_web::HttpResponse::new(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
