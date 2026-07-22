fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=keysor.rc");
        println!("cargo:rerun-if-changed=keysor.manifest");
        println!("cargo:rerun-if-changed=keysor.ico");
        embed_resource::compile("keysor.rc", embed_resource::NONE);
    }
}
