pub use builder::Jwt;
use chrono::{Duration, Utc};
pub use jsonwebtoken::*;
pub use middleware::JwtMiddleware;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod builder;
pub(crate) mod inner;
pub mod middleware;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    pub iat: Option<usize>,
    pub iss: Option<String>,
    pub nbf: Option<usize>,
    pub aud: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl Claims {
    // Check if the token has expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp() as usize;
        self.exp < now
    }
}

/// generate JWT token
pub fn generate_token(claims: &Claims, secret_key: &str) -> Result<String, crate::errors::Error> {
    let token = encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )?;

    Ok(token)
}

/// generate deault token
pub fn generate_token_with_defaults(
    sub: &Uuid,
    secret_key: &str,
    expiration_seconds: i64,
) -> Result<String, crate::errors::Error> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::seconds(expiration_seconds)).timestamp() as usize;

    let claims = Claims {
        sub: *sub,
        exp,
        iat: Some(iat),
        iss: Some("lean-link".to_string()),
        nbf: None,
        aud: None,
        data: None,
    };

    generate_token(&claims, secret_key)
}

#[cfg(test)]
mod tests {
    use actix_web::{App, post, test, web};

    use crate::service::web::middleware::jwt::{builder::Jwt, generate_token_with_defaults};
    use uuid::Uuid;

    #[post("hello")]
    async fn hello() -> actix_web::Result<String> {
        Ok("world".into())
    }
    fn configure_secured_routes(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api")
                .wrap(Jwt::default().set_secret_key("secret_key".to_string()))
                .service(hello),
        );
    }
    #[actix_web::test]
    async fn test_invalid_token() {
        let app = test::init_service(App::new().configure(configure_secured_routes)).await;
        let req = test::TestRequest::post()
            .uri("/api/hello")
            .insert_header(("Authorization", "Bearer your_token_here"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_null_token() {
        let app = test::init_service(App::new().configure(configure_secured_routes)).await;
        let req = test::TestRequest::post().uri("/api/hello").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_valid_token() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        let app = test::init_service(App::new().configure(configure_secured_routes)).await;
        let token = generate_token_with_defaults(&Uuid::now_v7(), "secret_key", 3600).unwrap();
        let req = test::TestRequest::post()
            .uri("/api/hello")
            .insert_header(("Authorization", format!("Bearer {}", token).as_str()))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
    }
}
