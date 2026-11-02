fn main() {
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
