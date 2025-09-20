use crate::utils::datetime::{local_time, local_time_option};
use actix_web::scope;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct UserLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(with = "local_time", rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(with = "local_time", rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(with = "local_time_option", rename = "deletedAt")]
    pub deleted_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Serialize, Deserialize)]
pub struct UserLoginResponse {
    pub token: String,
    pub user: User,
}

#[scope("/user")]
pub mod api {
    use crate::{
        AppState,
        database::users,
        service::web::{
            middleware::jwt,
            service::{
                ErrorCode, WebResponse,
                user::{User, UserLoginRequest, UserLoginResponse},
            },
        },
    };
    use actix_web::{post, web};

    #[post("/login")]
    async fn login(
        app_state: web::Data<AppState>,
        req: web::Json<UserLoginRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<UserLoginResponse>>> {
        let db_conn = &app_state.db_conn;

        let user = match users::find_user_by_name(db_conn, req.username.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Ok(
                    WebResponse::with_error_code(ErrorCode::InvalidUsernameOrPassword).into(),
                );
            }
            Err(e) => {
                tracing::error!(error = ?e);
                return Ok(WebResponse::with_error_code(ErrorCode::InternalError).into());
            }
        };

        let verify_password = match bcrypt::verify(req.password.clone(), &user.password) {
            Ok(verify) => verify,
            Err(e) => {
                tracing::error!(error = ?e);
                return Ok(WebResponse::with_error_code(ErrorCode::InternalError).into());
            }
        };

        if !verify_password {
            return Ok(WebResponse::with_error_code(ErrorCode::InvalidUsernameOrPassword).into());
        }

        let token = match jwt::generate_token_with_defaults(
            &user.id,
            &app_state.server_config.jwt.secret,
            app_state.server_config.jwt.expires_in.as_secs() as i64,
        ) {
            Ok(token) => token,
            Err(e) => {
                tracing::error!(error = ?e);
                return Ok(WebResponse::with_error_code(ErrorCode::InternalError).into());
            }
        };
        let resp = UserLoginResponse {
            token,
            user: User {
                id: user.id,
                username: user.username,
                created_at: user.created_at,
                updated_at: user.updated_at,
                deleted_at: user.deleted_at,
            },
        };
        Ok(WebResponse::with_result(resp).into())
    }
}
