//! Compile translations and embed the native Windows resources.
fn main() {
    // Retain the legacy OAuth adapter for protected 0.4.28 sessions only.
    // New YouTube connections use the first-party window, with no embedded client.
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("google-oauth.json"),
        b"{}",
    )
    .expect("write legacy configuration placeholder");
    fastframe_i18n::build::compile_catalogs("assets/i18n");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=packaging/windows/spotifast.ico");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("packaging/windows/spotifast.ico")
            .set("ProductName", "Vibefy")
            .set("FileDescription", "Vibefy");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=Windows resources not embedded: {error}");
        }
    }
}
