pub fn create_paths(path: &str) -> std::io::Result<()> {
    if path.starts_with(".") {
        #[cfg(target_family = "unix")]
        {
            std::fs::create_dir_all(std::path::Path::new(path))
        }

        #[cfg(target_family = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use std::path::Path;
            use winapi::um::fileapi::SetFileAttributesW;
            use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

            std::fs::create_dir_all(Path::new(path))?;
            // 转换路径为 UTF-16 编码（Windows API 所需）
            let wide_path: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(Some(0).into_iter())
                .collect();

            // 调用 Windows API 设置目录属性为隐藏
            unsafe  {
                let success = SetFileAttributesW(wide_path.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
                if success != 0 {
                    return Ok(());
                } else {
                    return Err(std::io::Error::last_os_error());
                }
            }
        }
    } else {
        std::fs::create_dir_all(std::path::Path::new(path))
    }
}
