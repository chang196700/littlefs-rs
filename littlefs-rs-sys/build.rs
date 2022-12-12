use std::env;
use std::path::PathBuf;
use bindgen::EnumVariation;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    #[cfg(feature = "online")]
    let littlefs_path = {
        use reqwest::header::HeaderMap;
        use reqwest::header::USER_AGENT;

        let lfs_path = out_path.clone().join("littlefs");
        let version = env::var("LITTLEFS_VERSION").unwrap_or_else(|_| "master".to_string());

        let mut headers = HeaderMap::new();
        headers.append(USER_AGENT, "rust-bindgen/0.59.1".parse().unwrap());

        let client = reqwest::blocking::Client::new();

        let archive = lfs_path.join(format!("littlefs-{}.zip", version));

        client.get(format!("https://github.com/littlefs-project/littlefs/archive/{}.zip", version))
            .headers(headers.clone())
            .send()?
            .copy_to(&mut std::fs::File::create(&archive)?)?;

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&archive)?)?;
        let name = archive.file_names().next().unwrap()
            .split_once('/').unwrap().0.to_string();

        archive.extract(&lfs_path)?;

        lfs_path.join(name)
    };

    #[cfg(not(feature = "online"))]
    let littlefs_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("littlefs");

    let littlefs_path = littlefs_path.to_str().unwrap();

    let target = env::var("CLANG_TARGET")
        .or_else(|_| env::var("TARGET"))?;

    let definitions = vec![
        #[cfg(not(feature = "malloc"))]
            "LFS_NO_MALLOC",
        #[cfg(not(feature = "debug"))]
            "LFS_NO_DEBUG",
        #[cfg(not(feature = "warn"))]
            "LFS_NO_WARN",
        #[cfg(not(feature = "error"))]
            "LFS_NO_ERROR",
        #[cfg(not(feature = "assert"))]
            "LFS_NO_ASSERT",
        #[cfg(feature = "trace")]
            "LFS_YES_TRACE",
    ];

    if cfg!(not(feature = "no_lib")) {
        let mut builder = cc::Build::new();
        let builder = builder
            .flag("-std=c11")
            .file(format!("{littlefs_path}/lfs.c"))
            .file(format!("{littlefs_path}/lfs_util.c"));

        for def in &definitions {
            builder.flag(&format!("-D{}", def));
        }

        let flags = env::var("CFLAGS").unwrap_or_default();
        for flag in flags.lines() {
            builder.flag(flag);
        }

        builder.compile("lfs-sys");
    }

    let mut bindings = bindgen::Builder::default();

    for def in &definitions {
        bindings = bindings.clang_arg(&format!("-D{}", def));
    }

    let bindings = bindings
        .header(format!("{littlefs_path}/lfs.h"))
        .allowlist_file(".*/lfs.h")
        .allowlist_file(".*/lfs_util.h")
        .clang_arg(format!("--target={}", target))
        .use_core()
        .ctypes_prefix("cty")
        .rustified_enum("lfs_error")
        .rustfmt_bindings(true)
        .fit_macro_constants(true)
        .default_enum_style(EnumVariation::NewType { is_bitfield: true, is_global: false })
        .derive_default(true)
        .generate()?;

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Ok(())
}
