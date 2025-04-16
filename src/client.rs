use fantoccini::{Client, ClientBuilder, wd::Capabilities};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tokio::time::Duration;

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

pub struct BrowserClient {
    client: Client,
    options: BrowserOptions,
}

impl BrowserClient {
    pub async fn connect(options: BrowserOptions) -> Result<Self, BrowserError> {
        let mut caps = Capabilities::new();

        // Firefox-specific options
        let mut firefox_options = json!({
            "args": if options.headless {
                vec!["-headless"]
            } else {
                vec![]
            }
        });

        // Add user agent if specifed
        if let Some(ua) = &options.user_agent {
            firefox_options["prefs"] = json!({
                "general.useragent.override": ua
            });
        }

        caps.insert("moz:firefoxOptions".to_string(), firefox_options);

        // Configure proxy
        if let Some(proxy) = &options.proxy {
            caps.insert(
                "proxy".to_string(),
                json!({
                    "proxyType": "manual",
                    "httpProxy": proxy,
                    "sslProxy" : proxy
                }),
            );
        }

        // Build client with capabilities
        let client = ClientBuilder::native()
            .capabilities(caps)
            .connect("http://localhost:4444")
            .await
            .map_err(|e| BrowserError::ConnectionError(e.to_string()))?;

        // Set window size
        if let Some((width, height)) = options.window_size {
            client
                .set_window_size(width, height)
                .await
                .map_err(|e| BrowserError::OperationError(e.to_string()))?;
        }

        Ok(Self { client, options })
    }

    pub async fn search_duckduckgo(&mut self, query: &str) -> Result<(), BrowserError> {
        let url = format!("https://duckduckgo.com/?q={}", query);

        self.client
            .goto(url.as_str())
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        self.client
            .goto(url)
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn shutdown(self) -> Result<(), BrowserError> {
        self.client
            .close()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    #[tokio::test]
    async fn test_browser() {
        let options = BrowserOptions::new().window_size(1920, 1080);

        let mut client = BrowserClient::connect(options).await.unwrap();

        client.search_duckduckgo("jordan 1s").await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        client
            .navigate("https://github.com/browser-use")
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        client.shutdown().await.unwrap();
    }
}
