use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// IO Error
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "modbus")]
    #[error("Modbus Error: {0}")]
    Modbus(#[from] tokio_modbus::Error),
    #[cfg(feature = "web")]
    #[error("JWT Error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[cfg(feature = "web")]
    #[error("Missing Token")]
    MissingToken,
}

impl actix_web::error::ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            Error::Io(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Modbus(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Jwt(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::MissingToken => actix_web::http::StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            Error::Io(_) => {
                actix_web::HttpResponse::new(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Modbus(_) => {
                actix_web::HttpResponse::new(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Jwt(err) => {
                actix_web::HttpResponse::Unauthorized().body(format!("{}", err))
            }
            Error::MissingToken => {
                actix_web::HttpResponse::new(actix_web::http::StatusCode::UNAUTHORIZED)
            }
        }
    }
}
