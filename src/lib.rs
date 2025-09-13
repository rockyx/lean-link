use sea_orm::DatabaseConnection;
use serde::{Serialize, de::DeserializeOwned};

use crate::config::ServerConfig;

pub mod config;
pub mod database;
pub mod errors;
pub mod service;
pub mod utils;

// pub use lean_link_macros::*;

pub struct AppState<UserState = (), UserConig = ()> {
    pub db_conn: DatabaseConnection,
    pub user_state: UserState,
    pub server_config: ServerConfig<UserConig>,
    pub server_name: String,
}

impl<UserState, UserConfig> AppState<UserState, UserConfig>
where
    UserConfig: DeserializeOwned + Serialize + Clone,
{
    pub fn new(
        server_name: &str,
        db_conn: &DatabaseConnection,
        user_state: UserState,
    ) -> std::io::Result<Self> {
        let server_config = config::load_config::<UserConfig>(server_name)?;
        Ok(Self {
            db_conn: db_conn.clone(),
            server_config,
            server_name: server_name.to_string(),
            user_state,
        })
    }
}

pub type Result<T> = std::result::Result<T, errors::Error>;
