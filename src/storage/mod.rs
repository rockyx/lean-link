use core::fmt;
use std::path::{Path, PathBuf};

fn get_td_path(app_name: &str) -> Option<PathBuf> {
    // Differentiate operating systems
    if cfg!(target_os = "linux") {
        Some(Path::new(&format!("/var/lib/{}/td", app_name)).into())
    } else {
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        Some(exe_dir.join("td"))
    }
}
pub fn persistent_storage(app_name: &str) -> tsink::StorageBuilder {
    let td_path = get_td_path(app_name);
    if td_path.is_none() {
        return tsink::StorageBuilder::new();
    }
    let td_path = td_path.unwrap();
    tsink::StorageBuilder::new().with_data_path(td_path)
}
