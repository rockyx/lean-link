use actix_web::scope;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageLogsRequest {
    #[serde(default)]
    pub page_index: u64,
    #[serde(default)]
    pub page_size: u64,
}

#[scope("/log")]
pub mod api {
    use actix_web::{post, web};

    use crate::{
        AppState,
        database::{entity::t_logs, logs},
        service::web::service::{Pagination, WebResponse, log::PageLogsRequest},
    };

    #[post("/page-logs")]
    pub async fn page_logs(
        app_state: web::Data<AppState>,
        req: web::Json<PageLogsRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<Pagination<t_logs::Model>>>, crate::errors::Error>
    {
        let db_conn = &app_state.db_conn;

        let result = logs::page_logs(db_conn, req.page_index, req.page_size).await?;

        Ok(WebResponse::with_result(result.into()).into())
    }
}
