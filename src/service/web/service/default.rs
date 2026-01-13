use actix_web::{HttpResponse, Result};

pub async fn default_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::NotFound().body("The requested resource was not found"))
}
