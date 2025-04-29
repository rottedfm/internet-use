use chrono::{Local, format};
use fantoccini::{
    Client, ClientBuilder,
    wd::{Capabilities, WindowHandle},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
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

    #[error("Failed to extract elements: {0}")]
    DomExtractionError(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum InteractiveElementType {
    Clickable,
    Typable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElement {
    pub selector: String,
    pub text: String,
    pub index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteractiveElement {
    pub selector: String,
    pub tag: String,
    pub text: String,
    pub r#type: String,
    pub placeholder: String,
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

    /// Extracts clickable, typeable, text elements, and highlights and labelss
    /// Extracts clickable, typeable, and text elements, while also highlighting them on the page.
    pub async fn extract_elements_with_text(
        &self,
    ) -> Result<(Vec<InteractiveElement>, Vec<TextElement>), BrowserError> {
        let script = r##"
(() => {
    const interactive = [];
    const texts = [];
    let index = 1;
    let letterCode = 65; // 'A'

    function lightYellowColor() {
        return `rgba(255, 255, 153, 0.5)`; // light yellow
    }

    function getNextLetter() {
        const letter = String.fromCharCode(letterCode);
        letterCode++;
        if (letterCode > 90) letterCode = 65;
        return letter;
    }

    function generateUniqueSelector(el) {
        if (!(el instanceof Element)) return "";
        const path = [];
        while (el.nodeType === Node.ELEMENT_NODE) {
            let selector = el.nodeName.toLowerCase();
            if (el.id) {
                selector += "#" + el.id;
                path.unshift(selector);
                break;
            } else {
                let sibling = el;
                let siblingIndex = 1;
                while ((sibling = sibling.previousElementSibling)) {
                    if (sibling.nodeName.toLowerCase() === selector)
                        siblingIndex++;
                }
                if (siblingIndex > 1) {
                    selector += ":nth-of-type(" + siblingIndex + ")";
                }
            }
            path.unshift(selector);
            el = el.parentNode;
        }
        return path.join(" > ");
    }

    // --- Global Info Panel ---
    const infoPanel = document.createElement("div");
    infoPanel.style.position = "fixed";
    infoPanel.style.top = "10px";
    infoPanel.style.right = "10px";
    infoPanel.style.width = "300px";
    infoPanel.style.maxHeight = "400px";
    infoPanel.style.overflowY = "auto";
    infoPanel.style.background = "rgba(255, 255, 255, 0.05)";
    infoPanel.style.color = "red";
    infoPanel.style.border = "2px solid red";
    infoPanel.style.borderRadius = "0px"; // square
    infoPanel.style.padding = "10px";
    infoPanel.style.boxShadow = "0 4px 12px rgba(0,0,0,0.15)";
    infoPanel.style.fontSize = "12px";
    infoPanel.style.zIndex = "10000";
    infoPanel.style.display = "none";
    infoPanel.style.transition = "opacity 0.2s, background 0.2s";
    document.body.appendChild(infoPanel);

    let hideTimer = null;

    function showInfo(content) {
        clearTimeout(hideTimer);
        infoPanel.innerHTML = content;
        infoPanel.style.display = "block";
        infoPanel.style.opacity = "1";
    }

    function hideInfo() {
        hideTimer = setTimeout(() => {
            infoPanel.style.opacity = "0";
            setTimeout(() => {
                infoPanel.style.display = "none";
            }, 200);
        }, 250);
    }

    function attachHoverEvents(target, infoContent, type) {
        target.addEventListener("mouseenter", () => {
            showInfo(infoContent);
            target.style.zIndex = "9999";
            if (type === "interactive") {
                target.style.border = "2px solid red";
            } else if (type === "text") {
                target.style.backgroundColor = lightYellowColor();
            }
        });
        target.addEventListener("mouseleave", () => {
            hideInfo();
            target.style.zIndex = "";
            if (type === "interactive") {
                target.style.border = "";
            } else if (type === "text") {
                target.style.backgroundColor = "";
            }
        });
    }

    // --- Step 1: clickable elements ---
    const allInteractiveElements = document.querySelectorAll("button, a, input, textarea, [onclick]");
    for (const el of allInteractiveElements) {
        el.style.position = "relative"; // for z-index stacking

        const selector = generateUniqueSelector(el);
        const label = `[${getNextLetter()}]`;

        const infoContent = `
            <strong>Interactive Element</strong><br/>
            Label: ${label}<br/>
            Selector: <code>${selector}</code><br/>
            Tag: ${el.tagName}<br/>
            Type: ${el.getAttribute("type") || "N/A"}<br/>
            Placeholder: ${el.getAttribute("placeholder") || "N/A"}
        `;
        attachHoverEvents(el, infoContent, "interactive");

        interactive.push({
            selector,
            tag: el.tagName,
            text: el.innerText.trim(),
            type: el.getAttribute("type") || "",
            placeholder: el.getAttribute("placeholder") || ""
        });
    }

    // --- Step 2: text blocks ---
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
        acceptNode: function(node) {
            if (node.parentNode &&
                node.parentNode.nodeName !== "SCRIPT" &&
                node.parentNode.nodeName !== "STYLE" &&
                node.textContent.trim().length > 0) {
                return NodeFilter.FILTER_ACCEPT;
            }
            return NodeFilter.FILTER_REJECT;
        }
    });

    const parentMap = new Map();
    let node = walker.nextNode();

    while (node) {
        const parent = node.parentNode;
        if (!parentMap.has(parent)) {
            parentMap.set(parent, []);
        }
        parentMap.get(parent).push(node);
        node = walker.nextNode();
    }

    for (const [parent, nodes] of parentMap.entries()) {
        const fullText = nodes.map(n => n.textContent.trim()).join(" ").trim();
        if (fullText.length === 0) continue;

        const wrapper = document.createElement("span");
        wrapper.style.borderRadius = "4px";
        wrapper.style.padding = "2px 6px";
        wrapper.style.margin = "1px";
        wrapper.style.display = "inline-block";
        wrapper.style.cursor = "pointer";

        const selector = generateUniqueSelector(parent);
        const label = `[${index}]`;

        const infoContent = `
            <strong>Text Block</strong><br/>
            Label: ${label}<br/>
            Selector: <code>${selector}</code><br/>
            Content: ${fullText}
        `;
        attachHoverEvents(wrapper, infoContent, "text");

        const textNode = document.createTextNode(fullText);
        wrapper.appendChild(textNode);

        for (const n of nodes) {
            parent.removeChild(n);
        }
        parent.insertBefore(wrapper, parent.firstChild);

        texts.push({
            selector,
            text: fullText,
            index: index
        });

        index++;
    }

    return { interactive, texts };
})();
"##;

        let res = self
            .client
            .execute(script, vec![])
            .await
            .map_err(|e| BrowserError::DomExtractionError(e.to_string()))?;

        let result = res.clone();

        let interactive = result
            .get("interactive")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let texts = result
            .get("texts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok((interactive, texts))
    }

    /// Navigates the current tab to the given URL.
    pub async fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        self.push_browser_log(&format!("Navigating to {}", url))
            .await?;

        self.client
            .goto(url)
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Navigates to DuckDuckGo and performs a search with the given query.
    pub async fn search_duckduckgo(&mut self, query: &str) -> Result<(), BrowserError> {
        let url = format!("https://duckduckgo.com/?q={}", query);
        self.push_browser_log(&format!("Searching DuckDuckGo for '{}'", query))
            .await?;
        self.navigate(&url).await
    }

    /// Navigates back in the browser history.
    pub async fn back(&mut self) -> Result<(), BrowserError> {
        self.push_browser_log("Navigating back").await?;
        self.client
            .back()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Navigates forward in the browser history.
    pub async fn forward(&mut self) -> Result<(), BrowserError> {
        self.push_browser_log("Navigating forward").await?;
        self.client
            .forward()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    /// Pushes a new message into the floating browser log panel.
    pub async fn push_browser_log(&self, message: &str) -> Result<(), BrowserError> {
        let script = r#"
(() => {
    if (!window.pushBrowserLog) {
        const logContainer = document.createElement("div");
        logContainer.id = "browser-log";
        logContainer.style.position = "fixed";
        logContainer.style.top = "10px";
        logContainer.style.left = "10px";
        logContainer.style.width = "500px"; // Expanded width
        logContainer.style.maxHeight = "500px";
        logContainer.style.overflowY = "auto";
        logContainer.style.background = "rgba(0, 0, 0, 0.05)"; // Slight dark transparent background
        logContainer.style.color = "red";
        logContainer.style.fontSize = "12px";
        logContainer.style.padding = "10px";
        logContainer.style.zIndex = "99999";
        logContainer.style.border = "none"; 
        logContainer.style.borderRadius = "0px"; 
        logContainer.style.pointerEvents = "none"; // Doesn't block clicks
        logContainer.style.fontFamily = "monospace";
        logContainer.style.boxShadow = "0 4px 12px rgba(0,0,0,0.2)";
        document.body.appendChild(logContainer);

        window.pushBrowserLog = function(msg) {
            const timestamp = new Date().toISOString().split('T')[1].split('.')[0];
            const entry = document.createElement("div");
            entry.style.padding = "2px 0"; // Tight log spacing
            entry.style.margin = "1px 0";
            entry.style.fontFamily = "monospace";
            entry.style.whiteSpace = "pre-wrap";
            entry.style.color = "red"; // Red text
            entry.textContent = `[${timestamp}] ${msg}`;
            logContainer.appendChild(entry);

            // Only scroll after 15 entries
            if (logContainer.childElementCount > 5) {
                logContainer.scrollTop = logContainer.scrollHeight;
            }
        };
    }
    window.pushBrowserLog(arguments[0]);
})();
"#;
        self.client
            .execute(script, vec![json!(message)])
            .await
            .map(|_| ())
            .map_err(|e| BrowserError::OperationError(format!("Failed to push browser log: {}", e)))
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
        self.push_browser_log(&format!("Clicking element '{}'", selector))
            .await?;
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
        self.push_browser_log(&format!("Typing '{}' into '{}'", text, selector))
            .await?;
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

    /// Scrolls to a element on screen
    pub async fn scroll_to(&mut self, selector: &str) -> Result<(), BrowserError> {
        self.push_browser_log(&format!("Scrolling to '{}'", selector))
            .await?;
        let js = r#"
        const el = document.querySelector(arguments[0]);
        if (el) {
            el.scrollIntoView({ behavior: 'smooth', block: 'center', inline: 'center' });
            return true;
        }
        return false;
        "#;

        let res = self
            .client
            .execute(js, vec![serde_json::to_value(selector).unwrap()])
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        match res.as_bool() {
            Some(true) => Ok(()),
            _ => Err(BrowserError::OperationError(format!(
                "Element not found or failed to scroll: {selector}"
            ))),
        }
    }

    /// Capture a timestamped screenshot of the BrowserClient
    pub async fn capture_screenshot(
        &mut self,
        output_dir: &Path,
        prefix: &str,
    ) -> Result<PathBuf, BrowserError> {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S%.3f");
        let filename = format!("{prefix}-{timestamp}.png");
        let path = output_dir.join(filename);

        let png_data = self
            .client
            .screenshot()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        fs::write(&path, &png_data).map_err(|e| BrowserError::OperationError(e.to_string()))?;

        Ok(path)
    }

    /// Opens a new browser tab and switches to it.
    pub async fn open_tab(&mut self) -> Result<(), BrowserError> {
        self.push_browser_log("Opening new tab").await?;
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
        self.push_browser_log(&format!("Switching to tab {}", index))
            .await?;

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
        self.push_browser_log(&format!("Closing tab {}", index))
            .await?;

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
    async fn browser_extract_elements_test() {
        let options = BrowserOptions::new().headless(false);
        let mut client = BrowserClient::connect(options).await.unwrap();

        client.navigate("https://duckduckgo.com/").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let (interactive_elements, text_elements) =
            client.extract_elements_with_text().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        client.click_element("#searchbox_input").await.unwrap();

        client
            .send_keys_to_element("#searchbox_input", "Minecraft Movie")
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        client
            .click_element("button.iconButton_button__A_Uiu:nth-child(2)")
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        client.navigate("https://github.com/").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        client.back().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        client.shutdown().await.unwrap();
        println!("✅ Shutdown cleanly.");
    }
}
