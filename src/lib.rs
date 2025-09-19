use crate::{
    config::ServerConfig,
    service::websocket::{WebSocketMessage, WebSocketServer},
};
use sea_orm::{Database, DatabaseConnection};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::mpsc::Receiver;

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
    #[cfg(feature = "web")]
    pub ws_server: WebSocketServer,
}

impl<UserState, UserConfig> AppState<UserState, UserConfig>
where
    UserConfig: DeserializeOwned + Serialize + Clone,
{
    pub async fn new(server_name: &str, user_state: UserState) -> std::io::Result<Self> {
        let server_config = config::load_config::<UserConfig>(server_name)?;
        let db_conn = Database::connect(server_config.database.url.clone())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        #[cfg(feature = "web")]
        let web_socket_server = WebSocketServer::new();

        Ok(Self {
            db_conn,
            server_config,
            server_name: server_name.to_string(),
            user_state: user_state,
            #[cfg(feature = "web")]
            ws_server: web_socket_server,
        })
    }

    #[cfg(feature = "web")]
    pub async fn start_web_socket(&self) -> std::io::Result<Receiver<WebSocketMessage>> {
        self.ws_server
            .start(
                &self.server_config.web_socket.host,
                &self.server_config.web_socket.port,
            )
            .await
    }
}

pub type Result<T> = std::result::Result<T, errors::Error>;
