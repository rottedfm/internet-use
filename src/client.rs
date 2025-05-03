use crate::types::{BrowserError, BrowserOptions, InteractiveElement, TextElement};

use chrono::Local;
use fantoccini::{
    Client, ClientBuilder,
    wd::{Capabilities, WindowHandle},
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::time::Duration;

pub struct BrowserClient {
    pub client: Client,
    pub options: BrowserOptions,
    pub current_tab: Option<WindowHandle>,
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
        self.wait_for_element(selector).await?;

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
        self.wait_for_element(selector).await?;

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

    pub async fn scroll_to(&mut self, selector: &str) -> Result<(), BrowserError> {
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

    pub async fn list_tabs(&mut self) -> Result<Vec<WindowHandle>, BrowserError> {
        self.client
            .windows()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub fn current_tab_handle(&self) -> Option<&WindowHandle> {
        self.current_tab.as_ref()
    }

    pub async fn shutdown(self) -> Result<(), BrowserError> {
        self.client
            .close()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn extract_interactive_elements(
        &self,
    ) -> Result<Vec<InteractiveElement>, BrowserError> {
        let js = r##"
        const interactive = [];
        const elements = document.querySelectorAll("button, a, input, textarea, [onclick]");

        for (const el of elements) {
            if (!(el instanceof Element)) continue;
            let selector = el.tagName.toLowerCase();
            if (el.id) selector += "#" + el.id;
            interactive.push({
                selector,
                tag: el.tagName,
                text: el.innerText.trim(),
                type: el.getAttribute("type") || "",
                placeholder: el.getAttribute("placeholder") || ""
            });
        }
        return interactive;
        "##;

        let result = self
            .client
            .execute(js, vec![])
            .await
            .map_err(|e| BrowserError::DomExtractionError(e.to_string()))?;
        serde_json::from_value(result).map_err(|e| BrowserError::DomExtractionError(e.to_string()))
    }

    pub async fn extract_text_elements(&self) -> Result<Vec<TextElement>, BrowserError> {
        let js = r##"
        const texts = [];
        const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
            acceptNode: node => {
                if (node.parentNode &&
                    node.parentNode.nodeName !== "SCRIPT" &&
                    node.parentNode.nodeName !== "STYLE" &&
                    node.textContent.trim().length > 0) {
                    return NodeFilter.FILTER_ACCEPT;
                }
                return NodeFilter.FILTER_REJECT;
            }
        });

        let index = 1;
        let node = walker.nextNode();
        while (node) {
            const parent = node.parentNode;
            const selector = parent.tagName.toLowerCase() + (parent.id ? "#" + parent.id : "");
            texts.push({
                selector,
                text: node.textContent.trim(),
                index: index++
            });
            node = walker.nextNode();
        }
        return texts;
        "##;

        let result = self
            .client
            .execute(js, vec![])
            .await
            .map_err(|e| BrowserError::DomExtractionError(e.to_string()))?;
        serde_json::from_value(result).map_err(|e| BrowserError::DomExtractionError(e.to_string()))
    }

    pub async fn inject_js(&mut self, script: &str) -> Result<serde_json::Value, BrowserError> {
        self.client
            .execute(script, vec![])
            .await
            .map_err(|e| BrowserError::OperationError(format!("JS injection failed: {}", e)))
    }

    pub async fn save_local_storage(&self) -> Result<Value, BrowserError> {
        let script = r#"(() => {
            const data = {};
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                data[key] = localStorage.getItem(key);
            }
            return data;
        })();"#;

        self.client
            .execute(script, vec![])
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn restore_local_storage(&self, data: &Value) -> Result<(), BrowserError> {
        let script = format!(
            r#"(() => {{
            const data = {};
            for (const key in Object.keys(data)) {{
                localStorage.setItem(key, data[key]);
            }}
        }})();"#,
            data
        );

        self.client
            .execute(&script, vec![])
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_title(&self) -> Result<String, BrowserError> {
        self.client
            .title()
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn save_session(&self, path: &Path) -> Result<(), BrowserError> {
        let storage = self.save_local_storage().await?;
        fs::write(path, storage.to_string())
            .map_err(|e| BrowserError::OperationError(e.to_string()))
    }

    pub async fn restore_session(&self, path: &Path) -> Result<(), BrowserError> {
        let content =
            fs::read_to_string(path).map_err(|e| BrowserError::OperationError(e.to_string()))?;
        let data: Value = serde_json::from_str(&content)
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;
        self.restore_local_storage(&data).await
    }
}
