pub fn create_paths(path: &str) -> std::io::Result<()> {
    if path.starts_with(".") {
        #[cfg(target_family = "unix")]
        {
            std::fs::create_dir_all(std::path::Path::new(path))
        }

        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::FileAttributesExt;
            std::fs::create_dir_all(std::path::Path::new(db_path))?;
            let metadata = std::fs::metadata(db_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() | 0x2); // 设置隐藏属性
            std::fs::set_permissions(db_path, permissions)
        }
    } else {
        std::fs::create_dir_all(std::path::Path::new(path))
    }
}
