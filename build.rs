fn main() {
    {
        use std::env;

        let sqlite = env::var("CARGO_FEATURE_SQLITE").is_ok();
        let mysql = env::var("CARGO_FEATURE_MYSQL").is_ok();
        let postgres = env::var("CARGO_FEATURE_POSTGRES").is_ok();

        if (sqlite && mysql && postgres) || (sqlite && mysql) || (sqlite && postgres) || (mysql && postgres) {
            panic!("Features 'sqlite', 'mysql' and 'postgres' cannot be enabled together.");
        }
    }
    #[cfg(feature = "imv-camera")]
    {
        use std::env;
        use std::path::PathBuf;

        let imv_dir = env::var("MV_VIEWER_DIR").expect("MV_VIEWER_DIR not set");

        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let lib_subdir = if target_arch == "x86_64" {
            "x64"
        } else {
            "win32"
        };
        let lib_path = PathBuf::from(&imv_dir)
            .join("Development")
            .join("Lib")
            .join(lib_subdir);

        println!("cargo::rustc-link-search={}", lib_path.display());
        println!("cargo::rustc-link-lib=MVSDKmd");

        let genicam_var = if target_arch == "x86_64" {
            "MV_GENICAM_64"
        } else {
            "MV_GENICAM_32"
        };

        if let Ok(genicam_path) = env::var(genicam_var) {
            println!("cargo:warning=GENICAM DLL path: {}", genicam_path);
        }

        let header_path = PathBuf::from(&imv_dir)
            .join("Development")
            .join("Include")
            .join("IMV")
            .join("IMVApi.h");

        if !header_path.exists() {
            panic!("header path not exists: {}", header_path.display());
        }

        let bindings = bindgen::Builder::default()
            .header(header_path.to_string_lossy())
            .clang_arg(format!(
                "-I{}",
                PathBuf::from(&imv_dir)
                    .join("Development")
                    .join("Include")
                    .join("IMV")
                    .display()
            ))
            .allowlist_function("IMV_.*")
            .allowlist_type("IMV_.*")
            .allowlist_var("IMV_.*")
            .blocklist_var("IMV_OK")
            .derive_debug(true)
            .derive_default(true)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .raw_line("#![allow(non_snake_case)]")
            .raw_line("#![allow(non_camel_case_types)]")
            .raw_line("#![allow(non_upper_case_globals)]")
            .raw_line("pub const IMV_OK: i32 = 0;")
            .generate()
            .expect("bindgen failed");

        let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("src")
            .join("ffi")
            .join("imv.rs");

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir failed");
        }

        bindings
            .write_to_file(&out_path)
            .expect("write to file failed");

        println!(
            "cargo:warning=binding file generated: {}",
            out_path.display()
        );
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-env-changed=MV_VIEWER_DIR");
        println!("cargo:rerun-if-env-changed=MV_GENICAM_32");
        println!("cargo:rerun-if-env-changed=MV_GENICAM_64");
    }
}
