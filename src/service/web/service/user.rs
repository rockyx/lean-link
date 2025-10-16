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
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(with = "local_time")]
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(with = "local_time")]
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(with = "local_time_option")]
    pub deleted_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

impl From<crate::database::entity::t_users::Model> for User {
    fn from(model: crate::database::entity::t_users::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            created_at: model.created_at,
            updated_at: model.updated_at,
            deleted_at: model.deleted_at,
        }
    }
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
    ) -> actix_web::Result<web::Json<WebResponse<UserLoginResponse>>, crate::errors::Error> {
        let db_conn = &app_state.db_conn;

        let user = match users::find_user_by_name(db_conn, req.username.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Err(crate::errors::Error::AuthorizationFail(
                    ErrorCode::InvalidUsernameOrPassword,
                ));
            }
            Err(e) => {
                tracing::error!(error = ?e);
                return Err(crate::errors::Error::DbErr(e));
            }
        };

        let verify_password = match bcrypt::verify(req.password.clone(), &user.password) {
            Ok(verify) => verify,
            Err(e) => {
                tracing::error!(error = ?e);
                return Err(crate::errors::Error::InternalError(
                    ErrorCode::InternalError,
                ));
            }
        };

        if !verify_password {
            return Err(crate::errors::Error::AuthorizationFail(
                ErrorCode::InvalidUsernameOrPassword,
            ));
        }

        let token = match jwt::generate_token_with_defaults(
            &user.id,
            &app_state.server_config.jwt.secret,
            app_state.server_config.jwt.expires_in.as_secs() as i64,
        ) {
            Ok(token) => token,
            Err(e) => {
                tracing::error!(error = ?e);
                return Err(e);
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

    #[post("/user-info")]
    async fn user_info(
        claims: Option<web::ReqData<jwt::Claims>>,
        app_state: web::Data<AppState>,
    ) -> actix_web::Result<web::Json<WebResponse<User>>, crate::errors::Error> {
        let db_conn = &app_state.db_conn;
        if claims.is_none() {
            return Err(crate::errors::Error::AuthorizationFail(
                crate::service::web::service::ErrorCode::Unauthorized,
            ));
        }

        let claims = claims.unwrap();
        let user_id = claims.sub;
        let user = match users::find_user_by_id(db_conn, user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Err(crate::errors::Error::AuthorizationFail(ErrorCode::InvalidUsernameOrPassword));
            }
            Err(e) => {
                tracing::error!(error = ?e);
                return Err(crate::errors::Error::DbErr(e));
            }
        };

        Ok(WebResponse::with_result(user.into()).into())
    }
}
