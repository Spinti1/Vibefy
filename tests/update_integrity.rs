#[path = "../src/update_integrity.rs"]
mod integrity;
use fastframe_update::{Request, Response, Transport};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};

struct Fake(BTreeMap<String, (u16, Option<String>, Vec<u8>)>);
impl Transport for Fake {
    fn get(&self, r: &Request<'_>) -> anyhow::Result<Response> {
        let (status, location, bytes) = self.0.get(r.url).expect("unexpected URL");
        Ok(Response {
            status: *status,
            location: location.clone(),
            body: Box::new(std::io::Cursor::new(bytes.clone())),
        })
    }
}
const META: &str = "https://api.github.com/repos/Spinti1/Vibefy/releases/tags/v9.9.9";
const FILE: &str = "https://github.com/Spinti1/Vibefy/releases/download/v9.9.9/vibefy-setup.exe";
fn request(url: &str) -> Request<'_> {
    Request {
        url,
        accept: "application/json",
        user_agent: "Vibefy/test",
    }
}
fn fixture(
    body: &[u8],
    size: u64,
    digest: serde_json::Value,
    redirect: Option<&str>,
) -> integrity::VerifiedTransport<Fake> {
    let metadata=serde_json::json!({"tag_name":"v9.9.9","assets":[{"name":"vibefy-setup.exe","browser_download_url":FILE,"size":size,"digest":digest}]}).to_string().into_bytes();
    let mut map = BTreeMap::from([
        (META.into(), (200, None, metadata)),
        (FILE.into(), (200, None, body.to_vec())),
    ]);
    if let Some(target) = redirect {
        map.insert(FILE.into(), (302, Some(target.into()), vec![]));
        map.insert(target.into(), (200, None, body.to_vec()));
    }
    integrity::VerifiedTransport::new(Fake(map))
}
fn digest(body: &[u8]) -> serde_json::Value {
    format!("sha256:{:x}", Sha256::digest(body)).into()
}
fn read(transport: &impl Transport, url: &str) -> anyhow::Result<Vec<u8>> {
    let mut bytes = vec![];
    transport.get(&request(url))?.body.read_to_end(&mut bytes)?;
    Ok(bytes)
}
#[test]
fn accepts_exact_release_bytes() {
    let t = fixture(b"setup", 5, digest(b"setup"), None);
    read(&t, META).unwrap();
    assert_eq!(read(&t, FILE).unwrap(), b"setup");
}
#[test]
fn rejects_corrupt_download() {
    let t = fixture(b"wrong", 5, digest(b"setup"), None);
    read(&t, META).unwrap();
    assert!(read(&t, FILE).is_err());
}
#[test]
fn rejects_short_download() {
    let t = fixture(b"set", 5, digest(b"setup"), None);
    read(&t, META).unwrap();
    assert!(read(&t, FILE).is_err());
}
#[test]
fn rejects_extra_bytes() {
    let t = fixture(b"setup!", 5, digest(b"setup"), None);
    read(&t, META).unwrap();
    assert!(read(&t, FILE).is_err());
}
#[test]
fn rejects_missing_or_invalid_digest() {
    for d in [
        serde_json::Value::Null,
        "sha256:1234".into(),
        "md5:1234".into(),
    ] {
        assert!(read(&fixture(b"setup", 5, d, None), META).is_err());
    }
}
#[test]
fn rejects_download_without_metadata() {
    assert!(read(&fixture(b"setup", 5, digest(b"setup"), None), FILE).is_err());
}
#[test]
fn binds_digest_across_github_redirect() {
    let target = "https://release-assets.githubusercontent.com/test?signature=temporary";
    let t = fixture(b"wrong", 5, digest(b"setup"), Some(target));
    read(&t, META).unwrap();
    assert_eq!(t.get(&request(FILE)).unwrap().status, 302);
    assert!(read(&t, target).is_err());
}
#[test]
fn rejects_unsafe_redirects() {
    for target in [
        "http://release-assets.githubusercontent.com/file",
        "https://github.com.evil.example/file",
        "https://127.0.0.1/file",
        "https://user@github.com/file",
    ] {
        let t = fixture(b"setup", 5, digest(b"setup"), Some(target));
        read(&t, META).unwrap();
        assert!(t.get(&request(FILE)).is_err());
    }
}
