fn build_identity() {
    fn collect(dir: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("source directory").flatten() {
            let path = entry.path();
            if path.is_dir() { collect(&path, paths); }
            else { paths.push(path); }
        }
    }
    let mut paths = Vec::new();
    collect(std::path::Path::new("src"), &mut paths);
    paths.extend(["Cargo.toml", "Cargo.lock", "build.rs", "keysor.manifest", "keysor.rc"].map(Into::into));
    paths.sort();
    let mut hash = 0xcbf29ce484222325u64;
    println!("cargo:rerun-if-changed=src");
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        for byte in path.to_string_lossy().bytes().chain(std::fs::read(&path).expect("build input")) {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rustc-env=KEYSOR_BUILD_ID={hash:016x}");
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let mode = if std::env::var("KEYSOR_UIACCESS_BUILD").as_deref() == Ok("1") { "uiaccess" } else { "standard" };
    println!("cargo:rustc-env=KEYSOR_BUILD_MODE={profile}-{mode}");
}

fn main() {
    build_identity();
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=keysor.rc");
        println!("cargo:rerun-if-changed=keysor.manifest");
        println!("cargo:rerun-if-changed=keysor.ico");
        println!("cargo:rerun-if-changed=keysor-uiaccess.rc");
        println!("cargo:rerun-if-changed=keysor-uiaccess.manifest");
        println!("cargo:rerun-if-env-changed=KEYSOR_UIACCESS_BUILD");
        println!("cargo:rerun-if-env-changed=KEYSOR_TEST_BUILD");
        let test_build = std::env::var("KEYSOR_TEST_BUILD").as_deref() == Ok("1")
            && std::env::var("PROFILE").as_deref() == Ok("debug");
        if !test_build {
            let resource = if std::env::var("KEYSOR_UIACCESS_BUILD").as_deref() == Ok("1") {
                "keysor-uiaccess.rc"
            } else {
                "keysor.rc"
            };
            embed_resource::compile(resource, embed_resource::NONE);
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
        println!("cargo:rustc-link-lib=framework=Carbon");
        println!("cargo:rerun-if-changed=src/platform/macos/overlay.m");
        println!("cargo:rerun-if-changed=keysor.icns");
        cc::Build::new()
            .file("src/platform/macos/overlay.m")
            .flag("-fobjc-arc")
            .compile("keysor_macos_overlay");
    }
}
