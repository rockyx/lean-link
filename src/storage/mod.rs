use std::path::PathBuf;

pub use tsink::*;

fn get_td_path() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    Some(exe_dir.join("td"))
}
pub fn persistent_storage() -> tsink::StorageBuilder {
    let td_path = get_td_path();
    if td_path.is_none() {
        return tsink::StorageBuilder::new();
    }
    let td_path = td_path.unwrap();
    tsink::StorageBuilder::new().with_data_path(td_path)
}
