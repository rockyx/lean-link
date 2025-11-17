use actix_web::web;
use serde::{Deserialize, Serialize};
use tracing::Instrument;

use crate::database::entity::PageResult;

pub mod user;

#[derive(Serialize, Deserialize)]
#[repr(u32)]
#[derive(Clone, Debug)]
pub enum ErrorCode {
    Success = 0,
    InvalidUsernameOrPassword = 10001,
    Unauthorized = 10002,
    InternalError = 50001,
}

fn error_code_to_u32<S>(code: &ErrorCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u32(code.clone() as u32)
}

#[derive(Serialize, Deserialize)]
pub struct WebResponse<T> {
    #[serde(serialize_with = "error_code_to_u32")]
    pub code: ErrorCode,
    pub success: bool,
    pub timestamp: i64,
    pub result: Option<T>,
    pub message: String,
}

impl<T> WebResponse<T> {
    pub fn with_error_code(code: &ErrorCode) -> Self {
        Self {
            code: code.clone(),
            success: false,
            timestamp: chrono::Local::now().timestamp(),
            result: None,
            message: "".to_string(),
        }
    }

    pub fn with_error_code_and_message(code: &ErrorCode, message: String) -> Self {
        Self {
            code: code.clone(),
            success: false,
            timestamp: chrono::Local::now().timestamp(),
            result: None,
            message,
        }
    }
}

impl<T> WebResponse<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    pub fn with_result(result: T) -> Self {
        Self {
            code: ErrorCode::Success,
            success: true,
            timestamp: chrono::Local::now().timestamp(),
            result: Some(result),
            message: "".to_string(),
        }
    }

    pub fn with_result_and_message(result: T, message: String) -> Self {
        Self {
            code: ErrorCode::Success,
            success: true,
            timestamp: chrono::Local::now().timestamp(),
            result: Some(result),
            message,
        }
    }
}

impl<T> Into<web::Json<WebResponse<T>>> for WebResponse<T> {
    fn into(self) -> web::Json<Self> {
        web::Json(self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Pagination<D> {
    pub records: Vec<D>,
    pub total: u64,
    pub current: u64,
    pub size: u64,
    pub pages: u64,
}

impl<T> From<PageResult<T>> for Pagination<T> {
    fn from(value: PageResult<T>) -> Self {
        Self {
            records: value.records,
            total: value.total_count,
            current: value.page_index,
            size: value.page_size,
            pages: value.pages,
        }
    }
}
