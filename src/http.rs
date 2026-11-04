//! Shared direct HTTP client. System, environment and manual proxies are disabled.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::settings::ProxyConfig;

/// A cheap-to-clone handle to the process HTTP client.
///
/// Replacing the inner client updates every clone: the Web API, token
/// refresh, artwork, lyrics and updates share the same connection pool.
#[derive(Clone)]
pub struct Http {
    inner: Arc<RwLock<Result<reqwest::Client, String>>>,
}

impl Http {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Ok(client))),
        }
    }

    pub fn from_proxy(proxy: &ProxyConfig) -> Result<Self, String> {
        build_client(proxy).map(Self::new)
    }

    /// Keep the interface available to repair settings without permitting
    /// requests to bypass the configuration that failed to build.
    pub fn unavailable(error: String) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Err(error))),
        }
    }

    pub fn replace(&self, client: reqwest::Client) {
        *self.inner.write().unwrap_or_else(|lock| lock.into_inner()) = Ok(client);
    }

    pub fn block(&self, error: String) {
        *self.inner.write().unwrap_or_else(|lock| lock.into_inner()) = Err(error);
    }

    pub fn client(&self) -> Result<reqwest::Client, String> {
        self.inner
            .read()
            .unwrap_or_else(|lock| lock.into_inner())
            .clone()
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::from_proxy(&ProxyConfig::Off).expect("direct HTTP client")
    }
}

impl From<reqwest::Client> for Http {
    fn from(client: reqwest::Client) -> Self {
        Self::new(client)
    }
}

pub fn build_client(proxy: &ProxyConfig) -> Result<reqwest::Client, String> {
    client_builder(proxy)?
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.without_url().to_string())
}

/// Collection mutations must not be replayed after an uncertain response.
/// Use a direct connection and disable retries and redirects.
pub(crate) fn build_collection_client(proxy: &ProxyConfig) -> Result<reqwest::Client, String> {
    client_builder(proxy)?
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| error.without_url().to_string())
}

pub fn build_blocking(
    proxy: &ProxyConfig,
    timeout: Duration,
) -> Result<reqwest::blocking::Client, String> {
    blocking_builder(proxy)?
        .timeout(timeout)
        .build()
        .map_err(|error| error.without_url().to_string())
}

fn client_builder(proxy: &ProxyConfig) -> Result<reqwest::ClientBuilder, String> {
    apply_proxy(reqwest::Client::builder().user_agent(user_agent()), proxy)
}

pub(crate) fn blocking_builder(
    proxy: &ProxyConfig,
) -> Result<reqwest::blocking::ClientBuilder, String> {
    apply_blocking_proxy(
        reqwest::blocking::Client::builder().user_agent(user_agent()),
        proxy,
    )
}

fn apply_proxy(
    builder: reqwest::ClientBuilder,
    _proxy: &ProxyConfig,
) -> Result<reqwest::ClientBuilder, String> {
    Ok(builder.no_proxy())
}
fn apply_blocking_proxy(
    builder: reqwest::blocking::ClientBuilder,
    _proxy: &ProxyConfig,
) -> Result<reqwest::blocking::ClientBuilder, String> {
    Ok(builder.no_proxy())
}

fn user_agent() -> &'static str {
    concat!("Vibefy/", env!("CARGO_PKG_VERSION"))
}
