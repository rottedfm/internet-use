use chrono::Local;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::Duration;

use crate::jobs::BrowserJob;

//
// ---------- Error Types ----------
//
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

    #[error("Memory error: {0}")]
    MemoryError(String),
}

//
// ---------- DOM Types ----------
//
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

//
// ---------- Browser Config ----------
//
#[derive(Debug, Clone, Serialize)]
pub struct BrowserOptions {
    pub headless: bool,
    pub window_size: Option<(u32, u32)>,
    pub proxy: Option<String>,
    pub user_agent: Option<String>,
    pub timeout: Duration,
    pub persist_path: Option<String>, // NEW: Optional file path for storing memory/cookies
}

impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            headless: false,
            window_size: Some((1920, 1080)),
            proxy: None,
            user_agent: None,
            timeout: Duration::from_secs(30),
            persist_path: None,
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

    pub fn persist_path(mut self, path: &str) -> Self {
        self.persist_path = Some(path.to_string());
        self
    }
}

//
// ---------- Memory Types ----------
//
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub timestamp: String,
    pub page_url: Option<String>,
    pub page_title: Option<String>,
    pub dom_snapshot: Option<String>,
    pub action: String,
    pub selector: Option<String>,
    pub job: BrowserJob,
}

impl MemoryEntry {
    pub fn new(job: &BrowserJob, page_url: Option<String>) -> Self {
        let timestamp = Local::now().to_rfc3339();
        let (action, selector) = match job {
            BrowserJob::Navigate(url) => ("Navigate".to_string(), Some(url.clone())),
            BrowserJob::Click(sel) => ("Click".to_string(), Some(sel.clone())),
            BrowserJob::Type { selector, .. } => ("Type".to_string(), Some(selector.clone())),
            BrowserJob::WaitFor(sel) => ("WaitFor".to_string(), Some(sel.clone())),
            BrowserJob::ScrollTo(sel) => ("ScrollTo".to_string(), Some(sel.clone())),
            BrowserJob::Screenshot { prefix } => ("Screenshot".to_string(), Some(prefix.clone())),
        };

        Self {
            timestamp,
            page_url,
            page_title: None,
            dom_snapshot: None,
            action,
            selector,
            job: job.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptions {
    pub max_entries: usize,
}

impl Default for MemoryOptions {
    fn default() -> Self {
        Self { max_entries: 50 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemory {
    history: Vec<MemoryEntry>,
    options: MemoryOptions,
}

impl AgentMemory {
    pub fn new(options: MemoryOptions) -> Self {
        Self {
            history: Vec::new(),
            options,
        }
    }

    pub fn add(&mut self, entry: MemoryEntry) {
        if self.history.len() >= self.options.max_entries {
            self.history.remove(0);
        }
        self.history.push(entry);
    }

    pub fn last(&self) -> Option<&MemoryEntry> {
        self.history.last()
    }

    pub fn last_n(&self, n: usize) -> Vec<&MemoryEntry> {
        self.history.iter().rev().take(n).collect()
    }

    pub fn all(&self) -> &Vec<MemoryEntry> {
        &self.history
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub fn to_json(&self) -> Result<String, BrowserError> {
        serde_json::to_string_pretty(&self.history)
            .map_err(|e| BrowserError::MemoryError(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, BrowserError> {
        let history: Vec<MemoryEntry> =
            serde_json::from_str(json).map_err(|e| BrowserError::MemoryError(e.to_string()))?;
        Ok(Self {
            history,
            options: MemoryOptions::default(),
        })
    }

    pub fn persist_to_file(&self, path: &str) -> Result<(), BrowserError> {
        std::fs::write(path, self.to_json()?).map_err(|e| BrowserError::MemoryError(e.to_string()))
    }

    pub fn load_from_file(path: &str) -> Result<Self, BrowserError> {
        let data =
            std::fs::read_to_string(path).map_err(|e| BrowserError::MemoryError(e.to_string()))?;
        Self::from_json(&data)
    }
}
