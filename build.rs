//! Compile translations and embed the native Windows resources.
fn main() {
    println!("cargo:rerun-if-env-changed=VIBEFY_GOOGLE_OAUTH_CONFIG");
    let google_path = std::env::var_os("VIBEFY_GOOGLE_OAUTH_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(".cache/google-oauth.json"));
    println!("cargo:rerun-if-changed={}", google_path.display());
    let google = std::fs::read(&google_path).unwrap_or_else(|_| b"{}".to_vec());
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("google-oauth.json"),
        google,
    )
    .expect("write Google application configuration");
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
