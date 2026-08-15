fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=keysor.rc");
        println!("cargo:rerun-if-changed=keysor.manifest");
        println!("cargo:rerun-if-changed=keysor.ico");
        embed_resource::compile("keysor.rc", embed_resource::NONE);
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
