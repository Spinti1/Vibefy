use serde_json::json;
use spotifast::youtube_browser::{self as browser, Profile};

#[test]
fn navigation_is_limited_to_real_google_and_youtube_https_hosts() {
    for url in [
        "https://accounts.google.com/ServiceLogin",
        "https://music.youtube.com/library",
        "https://consent.youtube.com/",
    ] {
        assert!(browser::allowed_navigation(url));
    }
    for url in [
        "http://music.youtube.com",
        "https://youtube.com.evil.test/",
        "https://notgoogle.com/",
        "https://accounts.google.com@evil.test/",
        "file:///C:/Windows/win.ini",
        "https://music.youtube.com:8443",
        "javascript:alert(1)",
    ] {
        assert!(!browser::allowed_navigation(url), "{url}");
    }
    assert!(!browser::music_origin("https://accounts.google.com/"));
    assert!(browser::music_origin("https://music.youtube.com/library"));
}

#[test]
fn profile_identifiers_cannot_escape_the_owned_directory() {
    assert!(browser::valid_id("0123456789abcdef0123456789abcdef"));
    for value in [
        "",
        "..",
        "../../other",
        "0123456789abcdef0123456789abcdef/..",
        "gggggggggggggggggggggggggggggggg",
    ] {
        assert!(!browser::valid_id(value));
    }
    let mut profile: Profile = serde_json::from_value(json!({"id":"../other"})).unwrap();
    assert!(profile.bind(&std::env::temp_dir()).is_err());
    let mut profile: Profile = serde_json::from_value(
        json!({"id":"0123456789abcdef0123456789abcdef", "root":"C:/outside"}),
    )
    .unwrap();
    profile.bind(&std::env::temp_dir()).unwrap();
    assert_eq!(
        serde_json::to_value(profile).unwrap(),
        json!({"id":"0123456789abcdef0123456789abcdef"})
    );
}

#[test]
fn page_messages_require_the_correct_origin_and_request_nonce() {
    let message = json!({"id":"expected", "status":200, "body":"{\"ok\":true}"}).to_string();
    assert!(
        browser::response_message("https://evil.test", "expected", &message)
            .unwrap()
            .is_none()
    );
    assert!(
        browser::response_message(browser::ORIGIN, "other", &message)
            .unwrap()
            .is_none()
    );
    let response = browser::response_message(browser::ORIGIN, "expected", &message)
        .unwrap()
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, json!({"ok":true}));
}

#[test]
fn malformed_unconfirmed_and_oversized_responses_are_rejected() {
    for value in [
        json!({"id":"a","error":true}),
        json!({"id":"a","status":0,"body":"{}"}),
        json!({"id":"a","status":200,"body":"<html>"}),
        json!({"id":"a","status":200}),
    ] {
        assert!(browser::response_message(browser::ORIGIN, "a", &value.to_string()).is_err());
    }
    assert!(
        browser::response_message(browser::ORIGIN, "a", &"x".repeat(16 * 1024 * 1024 + 1)).is_err()
    );
    let value = json!({"id":"a","status":429,"retryAfter":"60","body":"{}"});
    let response = browser::response_message(browser::ORIGIN, "a", &value.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(response.retry_after.as_deref(), Some("60"));
}

#[test]
fn only_library_and_playlist_endpoints_are_available() {
    for value in [
        "browse",
        "account/account_menu",
        "playlist/create",
        "browse/edit_playlist",
    ] {
        assert!(browser::valid_endpoint(value));
    }
    for value in [
        "https://evil.test/",
        "../player",
        "player",
        "delete_account",
        "browse?redirect=https://evil.test",
    ] {
        assert!(!browser::valid_endpoint(value));
    }
}

#[test]
fn navigation_diagnostics_exclude_sensitive_url_components() {
    let value = browser::navigation_diagnostic(
        "https://user:password@accounts.youtube.com/private-account?code=secret&email=example#token",
        "starting",
        2,
    );
    assert_eq!(
        value,
        json!({"host":"accounts.youtube.com","event":"starting","seconds":2})
    );
    let diagnostic = value.to_string();
    for secret in ["password", "private-account", "secret", "example", "token"] {
        assert!(!diagnostic.contains(secret));
    }
    assert!(browser::navigation_diagnostic("not a URL", "failed", 3)["host"].is_null());
}

#[test]
fn official_regional_google_signin_redirects_are_allowed_without_lookalikes() {
    for domain in include_str!("../assets/google-signin-domains.txt")
        .lines()
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
    {
        assert!(
            browser::allowed_navigation(&format!("https://accounts.{domain}/accounts/SetSID")),
            "{domain}"
        );
        assert!(
            !browser::allowed_navigation(&format!("https://accounts.{domain}.evil.test/")),
            "{domain}"
        );
        assert!(
            !browser::allowed_navigation(&format!("https://evil{domain}/")),
            "{domain}"
        );
    }
    for url in [
        "https://accounts.google.ro/accounts/SetSID",
        "https://accounts.google.co.uk/",
        "https://accounts.google.com.au/",
    ] {
        assert!(browser::allowed_navigation(url));
    }
    for url in [
        "http://accounts.google.ro",
        "https://accounts.google.invalid",
        "https://google.ro@evil.test",
        "https://accounts.google.ro:8443",
        "https://google.ro.evil.test",
    ] {
        assert!(!browser::allowed_navigation(url));
    }
}
