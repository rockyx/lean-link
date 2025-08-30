pub mod service;
pub mod database;
pub mod utils;
pub mod config;

pub use lean_link_macros::*;

pub struct AppState {
    db: std::sync::Arc<database::DbHelper>,
}

impl AppState {
    pub fn get_db(&self) -> std::sync::Arc<database::DbHelper> {
        self.db.clone()
    }
}
