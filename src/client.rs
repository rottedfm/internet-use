use fantoccini::{
    Client, ClientBuilder,
    wd::{Capabilities, WindowHandle},
};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tokio::time::Duration;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("WebDriver connection failed: {0}")]
    ConnectionError(String),

    #[error("Browser operation failed: {0}")]
    OperationError(String),

    #[error("Invalid browser configuration: {0}")]
    ConfigError(String),
}

/// Configuration options for intializing a browser session.
#[derive(Debug, Clone, Serialize)]
pub struct BrowserOptions {
    /// Weather the browser should run in headless mode.
    pub headless: bool,
    /// Optional window dimensions (width, height).
    pub window_size: Option<(u32, u32)>,
    /// Optional proxy URL to use for HTTP/HTTPS traffic.
    pub proxy: Option<String>,
    /// Optional user agent string override.
    pub user_agent: Option<String>,
    /// Timeout for browser operations, in seconds.
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
    /// Creates a new `BrowserOptions` instance with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets headless mode (true = no UI).
    pub fn headless(mut self, enabled: bool) -> Self {
        self.headless = enabled;
        self
    }

    /// Sets the browser window size.
    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.window_size = Some((width, height));
        self
    }

    /// Sets a proxy server for the browser session
    pub fn proxy(mut self, proxy_url: &str) -> Self {
        self.proxy = Some(proxy_url.to_string());
        self
    }

    /// Overrides the browser's default user agent string.
    pub fn user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }

    /// Sets the timeout for operations (in seconds).
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }
}

/// High-level browser automation client powered by 'fantoccini'.
pub struct BrowserClient {
    /// The underlying WebDriver client instance.
    pub client: Client,
    /// Configuration options used to initialize the browser.
    options: BrowserOptions,
    /// The current active tab/window handle.
    current_tab: Option<WindowHandle>,
}

impl BrowserClient {
    /// Connects to the WebDriver server with the given options and returns a `BrowserClient`.
    pub async fn connect(options: BrowserOptions) -> Result<Self, BrowserError> {
        let mut caps = Capabilities::new();

        let mut firefox_options = json!({
            "args": if options.headless {
                vec!["-headless"]
            } else {
                vec![]
            }
        });

        if let Some(ua) = &options.user_agent {
            firefox_options["prefs"] = json!({
                "general.useragent.override": ua
            });
        }

        caps.insert("moz:firefoxOptions".to_string(), firefox_options);

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

        let client = ClientBuilder::native()
            .capabilities(caps)
            .connect("http://localhost:4444")
            .await
            .map_err(|e| BrowserError::ConnectionError(e.to_string()))?;

        if let Some((width, height)) = options.window_size {
            client
                .set_window_size(width, height)
                .await
                .map_err(|e| BrowserError::OperationError(e.to_string()))?;
        }

        let handles = client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        let current_tab = handles.first().cloned();

        Ok(Self {
            client,
            options,
            current_tab,
        })
    }

    /// Navigates the current tab to the given URL.
    pub async fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        self.client
            .goto(url)
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Navigates to DuckDuckGo and performs a search with the given query.
    pub async fn search_duckduckgo(&mut self, query: &str) -> Result<(), BrowserError> {
        let url = format!("https://duckduckgo.com/?q={}", query);
        self.navigate(&url).await
    }

    /// Navigates back in the browser history.
    pub async fn back(&mut self) -> Result<(), BrowserError> {
        self.client
            .back()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Navigates forward in the browser history.
    pub async fn forward(&mut self) -> Result<(), BrowserError> {
        self.client
            .forward()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Waits for an element matching the CSS selector to appear.
    /// Returns `true` if found, `false` if it times out.
    pub async fn wait_for_element(&mut self, element: &str) -> Result<bool, BrowserError> {
        match self
            .client
            .wait()
            .for_element(fantoccini::Locator::Css(element))
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Click an element matching the given CSS selector.
    pub async fn click_element(&mut self, selector: &str) -> Result<(), BrowserError> {
        self.wait_for_selector(selector).await?;

        let el = self
            .client
            .wait()
            .for_element(fantoccini::Locator::Css(selector))
            .await
            .map_err(|e| {
                BrowserError::OperationError(format!("Failed to find '{}': {}", selector, e))
            })?;

        el.click().await.map_err(|e| {
            BrowserError::OperationError(format!("Click failed '{}': {}", selector, e))
        })
    }

    /// Sends keys to an input or textarea matching the given selector.
    pub async fn send_keys_to_element(
        &mut self,
        selector: &str,
        text: &str,
    ) -> Result<(), BrowserError> {
        self.wait_for_selector(selector).await?;

        let el = self
            .client
            .wait()
            .for_element(fantoccini::Locator::Css(selector))
            .await
            .map_err(|e| {
                BrowserError::OperationError(format!("Failed to find '{}': {}", selector, e))
            })?;

        el.send_keys(text).await.map_err(|e| {
            BrowserError::OperationError(format!("Send keys failed '{}': {}", selector, e))
        })
    }

    /// Returns the full page source HTML of the current tab.
    pub async fn source(&mut self) -> Result<String, BrowserError> {
        self.client
            .source()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Opens a new browser tab and switches to it.
    pub async fn open_tab(&mut self) -> Result<(), BrowserError> {
        self.client
            .execute("window.open('about:blank', '_blank');", vec![])
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let handles = self
            .client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        if let Some(handle) = handles.last() {
            self.client
                .switch_to_window(handle.clone())
                .await
                .map_err(|e| BrowserError::OperationError(e.to_string()))?;
            self.current_tab = Some(handle.clone());
        }

        Ok(())
    }

    /// Switches to the tab at the specified index.
    pub async fn switch_tab(&mut self, index: usize) -> Result<(), BrowserError> {
        let handles = self
            .client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        if let Some(handle) = handles.get(index) {
            self.client
                .switch_to_window(handle.clone())
                .await
                .map_err(|e| BrowserError::OperationError(e.to_string()))?;
            self.current_tab = Some(handle.clone());
            Ok(())
        } else {
            Err(BrowserError::OperationError(format!(
                "No tab at index {} ({} tabs open)",
                index,
                handles.len()
            )))
        }
    }

    /// Close the tab at the specified index and updates the current tab handle.
    ///
    /// Fails if only one tab is open or if the index is invaild.
    pub async fn close_tab(&mut self, index: usize) -> Result<(), BrowserError> {
        let handles = self
            .client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        if handles.len() <= 1 {
            return Err(BrowserError::OperationError(
                "Cannot close the only remaining tab".into(),
            ));
        }

        if index >= handles.len() {
            return Err(BrowserError::OperationError(format!(
                "Tab index {} out of bounds ({} tabs open)",
                index,
                handles.len()
            )));
        }

        let handle_to_close = handles[index].clone();
        self.client
            .switch_to_window(handle_to_close.clone())
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        // Close it
        self.client
            .close_window()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        let remaining = self
            .client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        self.current_tab = remaining.first().cloned();
        Ok(())
    }

    /// Returns a list of all open window handles (tabs).
    pub async fn list_tabs(&mut self) -> Result<Vec<WindowHandle>, BrowserError> {
        self.client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Returns the current tab's window handle, if available.
    pub fn current_tab_handle(&self) -> Option<&WindowHandle> {
        self.current_tab.as_ref()
    }

    /// Waits for a specific CSS selector to be present in the current tab.
    pub async fn wait_for_selector(&mut self, wait_selector: &str) -> Result<(), BrowserError> {
        self.client
            .wait()
            .for_element(fantoccini::Locator::Css(wait_selector))
            .await
            .map(|_| ())
            .map_err(|e| {
                BrowserError::OperationError(format!(
                    "Page not ready (waiting for '{}'): {}",
                    wait_selector, e
                ))
            })
    }

    /// Shuts down the browser session and closes the webdriver.
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
    async fn browser_client_test() {
        let options = BrowserOptions::new();
        let mut client = BrowserClient::connect(options).await.unwrap();

        let total_tabs = 4;

        client.navigate("https://duckduckgo.com/").await.unwrap();

        // Open and navigate tabs
        for _ in 0..total_tabs {
            client.open_tab().await.unwrap();
            client.navigate("https://duckduckgo.com/").await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        let mut handles = client.list_tabs().await.unwrap();

        // Close all except the first tab (index 0)
        while handles.len() > 1 {
            // Always close the *last* one for safety
            let last_index = handles.len() - 1;
            client.close_tab(last_index).await.unwrap();

            handles = client.list_tabs().await.unwrap(); // refresh handles
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        client.switch_tab(0).await.unwrap();

        client
            .send_keys_to_element("input[name='q']", "I am a robot!")
            .await
            .unwrap();

        client
            .click_element(".iconButton_size-20__Ql3lL")
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;

        client.back().await.unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;

        client.forward().await.unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;

        let source = client.source().await.unwrap();

        println!("{}", source);

        client.shutdown().await.unwrap();
    }
}
