//! Cast stub for Spotify Connect removal.

pub fn device_id(_name: &str) -> String {
    String::new()
}

pub fn start<F>(_: std::net::IpAddr, _: u16, _: &str, _: F) -> anyhow::Result<()> {
    Ok(())
}

pub async fn device_token(
    _: &crate::session_reads::Session,
    _: &str,
    _: &str,
) -> anyhow::Result<String> {
    Ok(String::new())
}
