use fantoccini::Client;
use serde::Serialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("WebDriver connection failed: {0}")]
    ConnectionError(String),

    #[error("Browser operation faild: {0}")]
    OperationError(String),

    #[error("Invaild browser configuration: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserOptions {
    pub headless: bool,
    pub window_size: Option<(u32, u32)>,
    pub proxy: Option<String>,
    pub user_agent: Option<String>,
    pub timeout: Duration,
}

impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            headless: false,
            window_size: Some((1920, 1080)),
            proxy: None,
            user_agent: None,
            timeout: Duration::from_secs(30),
        }
    }
}

impl BrowserOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn headless(mut self, enabled: bool) -> Self {
        self.headless = enabled;
        self
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.window_size = Some((width, height));
        self
    }

    pub fn proxy(mut self, proxy_url: &str) -> Self {
        self.proxy = Some(proxy_url.to_string());
        self
    }

    pub fn user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }
}
