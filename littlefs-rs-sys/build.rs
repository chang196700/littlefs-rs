use std::env;
use std::path::PathBuf;
use bindgen::EnumVariation;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

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
            .file("littlefs/lfs.c")
            .file("littlefs/lfs_util.c");

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
        .header("littlefs/lfs.h")
        .allowlist_file("littlefs/lfs.h")
        .allowlist_file("littlefs/lfs_util.h")
        .clang_arg(format!("--target={}", target))
        .use_core()
        .ctypes_prefix("cty")
        .rustified_enum("lfs_error")
        .rustfmt_bindings(true)
        .fit_macro_constants(true)
        .default_enum_style(EnumVariation::NewType { is_bitfield: true, is_global: false })
        .derive_default(true)
        .generate()?;

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Ok(())
}
