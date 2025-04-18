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
    pub client: Client,
    options: BrowserOptions,
    current_tab: Option<WindowHandle>,
}

impl BrowserClient {
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

    pub async fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        self.client
            .goto(url)
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn search_duckduckgo(&mut self, query: &str) -> Result<(), BrowserError> {
        let url = format!("https://duckduckgo.com/?q={}", query);
        self.navigate(&url).await
    }

    pub async fn back(&mut self) -> Result<(), BrowserError> {
        self.client
            .back()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn forward(&mut self) -> Result<(), BrowserError> {
        self.client
            .forward()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

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

    pub async fn click_element(&mut self, selector: &str) -> Result<(), BrowserError> {
        self.wait_for_tab_ready(selector).await?;

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

    pub async fn send_keys_to_element(
        &mut self,
        selector: &str,
        text: &str,
    ) -> Result<(), BrowserError> {
        self.wait_for_tab_ready(selector).await?;

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

    pub async fn source(&mut self) -> Result<String, BrowserError> {
        self.client
            .source()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

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

    /// Close the tab at a specific index (switches to it first)
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

        // Switch to the tab we want to close
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

        // After closing, update current tab to the first remaining one
        let remaining = self
            .client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        self.current_tab = remaining.first().cloned();
        Ok(())
    }

    pub async fn list_tabs(&mut self) -> Result<Vec<WindowHandle>, BrowserError> {
        self.client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub fn current_tab_handle(&self) -> Option<&WindowHandle> {
        self.current_tab.as_ref()
    }

    pub async fn wait_for_tab_ready(&mut self, wait_selector: &str) -> Result<(), BrowserError> {
        // Wait for tab to become usable and element to exist
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
