#[path = "../src/oauth_transport.rs"]
mod transport;

#[tokio::test]
async fn bundled_roots_do_not_accept_an_untrusted_https_certificate() {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::sync::Arc;
    use std::time::Duration;
    // Public test-only localhost key. It grants no application or server access.
    let cert = CertificateDer::from(include_bytes!("fixtures/oauth/localhost-cert.der").to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        include_bytes!("fixtures/oauth/localhost-key.der").to_vec(),
    ));
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![cert], key)
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("https://{}/token", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let config = Arc::new(config);
        let mut connections = 0;
        while connections < 2 && std::time::Instant::now() < deadline {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    socket
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    socket
                        .set_write_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let tls = rustls::ServerConnection::new(Arc::clone(&config)).unwrap();
                    let mut stream = rustls::StreamOwned::new(tls, socket);
                    let mut application_data = [0; 1024];
                    // The client may reject after the server sends Finished.
                    // No OAuth form may cross this untrusted connection.
                    assert!(
                        !matches!(std::io::Read::read(&mut stream, &mut application_data), Ok(length) if length > 0)
                    );
                    connections += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(error) => panic!("{error}"),
            }
        }
        connections
    });
    let http = reqwest::Client::builder()
        .no_proxy()
        .tls_built_in_native_certs(false)
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let error = transport::post_form(&http, &url, &[("code", "dummy-secret")])
        .await
        .unwrap_err();
    assert!(transport::describe(&error).contains("certificate"));
    assert!(!transport::describe(&error).contains("dummy-secret"));
    assert_eq!(server.join().unwrap(), 2);
}
