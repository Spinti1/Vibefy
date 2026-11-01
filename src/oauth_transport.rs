//! OAuth transport: verified HTTPS, bounded requests and safe connection retries.

use std::error::Error;
use std::time::Duration;

/// Retry only failure to establish the connection. Once a request may have
/// reached Spotify, do not replay a one-use authorization code or refresh token.
pub(crate) async fn post_form(
    http: &reqwest::Client,
    url: &str,
    form: &[(&str, &str)],
) -> Result<reqwest::Response, reqwest::Error> {
    let mut retried = false;
    loop {
        match http
            .post(url)
            .timeout(Duration::from_secs(20))
            .form(form)
            .send()
            .await
        {
            Err(error) if !retried && error.is_connect() && !error.is_timeout() => {
                retried = true;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            result => return result,
        }
    }
}

pub(crate) fn describe(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        return "Spotify sign-in timed out. Check your connection and try signing in again.";
    }
    let mut causes: Vec<&dyn Error> = Vec::new();
    let mut cause = error.source();
    while let Some(current) = cause {
        causes.push(current);
        cause = current.source();
    }
    // Inspect privately; never display source strings, URLs, proxy credentials,
    // request bodies or OAuth response contents in errors or logs.
    classify(
        causes.iter().map(|cause| cause.to_string()),
        error.is_connect(),
    )
}

fn classify(causes: impl IntoIterator<Item = String>, connect: bool) -> &'static str {
    let mut certificate = false;
    let mut dns = false;
    let mut proxy = false;
    for cause in causes {
        let cause = cause.to_lowercase();
        certificate |= cause.contains("certificate")
            || cause.contains("unknownissuer")
            || cause.contains("certnotvalid")
            || cause.contains("invalid peer");
        dns |= cause.contains("dns")
            || cause.contains("resolve")
            || cause.contains("name or service not known");
        proxy |= cause.contains("proxy") || cause.contains("tunnel");
    }
    if certificate {
        "Cannot verify Spotify's HTTPS certificate. Check Windows date/time, Windows updates and your network's HTTPS certificate configuration."
    } else if proxy {
        "The configured proxy could not connect to Spotify. Check Settings > Network and your proxy settings, then sign in again."
    } else if dns {
        "Cannot resolve accounts.spotify.com. Check your internet connection and DNS, then sign in again."
    } else if connect {
        "Cannot connect to Spotify. Check your connection, VPN/proxy settings and whether your network permits accounts.spotify.com, then sign in again."
    } else {
        "The connection to Spotify was interrupted. Sign in again to request a new authorization code."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn diagnostics_classify_without_exposing_underlying_secrets() {
        for (cause, expected) in [
            ("invalid peer certificate UnknownIssuer", "certificate"),
            ("dns error: failed to resolve name", "DNS"),
            ("proxy tunnel connection failed", "proxy"),
            ("connection refused", "Cannot connect"),
        ] {
            let cause =
                format!("{cause} https://dummy-user:dummy-password@private.test/?code=dummy-code");
            let message = classify([cause], true);
            assert!(message.contains(expected));
            assert!(!message.contains("dummy"));
            assert!(!message.contains("private.test"));
        }
    }

    #[tokio::test]
    async fn an_http_rejection_is_returned_without_retry() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/token", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let length = socket.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..length]).contains("POST /token"));
            socket
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(400), listener.accept())
                    .await
                    .is_err()
            );
        });
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let response = post_form(&http, &url, &[("code", "fixture")])
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn a_response_lost_after_sending_is_not_replayed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/token", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            drop(socket);
            assert!(
                tokio::time::timeout(Duration::from_millis(400), listener.accept())
                    .await
                    .is_err()
            );
        });
        let http = reqwest::Client::builder()
            .no_proxy()
            .retry(reqwest::retry::never())
            .build()
            .unwrap();
        let error = post_form(&http, &url, &[("code", "fixture")])
            .await
            .unwrap_err();
        assert!(!error.is_connect());
        assert!(describe(&error).contains("interrupted"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn only_connect_failures_retry_once_without_sending_the_form_to_the_proxy() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(3), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut bytes = [0; 4096];
                let length = socket.read(&mut bytes).await.unwrap();
                let text = String::from_utf8_lossy(&bytes[..length]);
                assert!(text.starts_with("CONNECT accounts.spotify.com:443"));
                assert!(!text.contains("dummy-code"));
                socket.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
            }
            assert!(
                tokio::time::timeout(Duration::from_millis(400), listener.accept())
                    .await
                    .is_err()
            );
        });
        let http = reqwest::Client::builder()
            .no_proxy()
            .proxy(reqwest::Proxy::all(proxy).unwrap())
            .retry(reqwest::retry::never())
            .build()
            .unwrap();
        let error = post_form(
            &http,
            "https://accounts.spotify.com/api/token",
            &[("code", "dummy-code")],
        )
        .await
        .unwrap_err();
        assert!(error.is_connect());
        assert!(describe(&error).contains("proxy"));
        server.await.unwrap();
    }
}
