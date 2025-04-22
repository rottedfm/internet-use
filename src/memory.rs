use chrono;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Memory-related errors.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Failed to serialize memory: {0}")]
    SerializationError(String),

    #[error("Invaild memory operation: {0}")]
    InvaildOperation(String),
}

/// Represents a record of interaction with a DOM element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub selector: String,
    pub action: String,
    pub timestamp: String,
    pub page_url: Option<String>,
}

/// Configuration options for intitializing memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptions {
    /// Max number of records to retain.
    pub max_entries: usize,
}

impl Default for MemoryOptions {
    fn default() -> Self {
        Self { max_entries: 50 }
    }
}

/// In-memory interaction history store.
#[derive(Debug, Default)]
pub struct AgentMemory {
    /// Ordered list of recent interactions.
    history: Vec<MemoryEntry>,
    /// Quick lookup for deduplication
    visited_selectors: HashSet<String>,
    /// Optional configuration.
    options: MemoryOptions,
}

impl AgentMemory {
    /// Create a new memory store with optional config.
    pub fn new(options: MemoryOptions) -> Self {
        Self {
            history: Vec::new(),
            visited_selectors: HashSet::new(),
            options,
        }
    }

    /// Adds a new memory entry.
    pub fn record(
        &mut self,
        selector: &str,
        action: &str,
        page_url: Option<&str>,
    ) -> Result<(), MemoryError> {
        let timestamp = chrono::Utc::now().to_rfc3339();

        let entry = MemoryEntry {
            selector: selector.to_string(),
            action: action.to_string(),
            timestamp,
            page_url: page_url.map(str::to_string),
        };

        self.visited_selectors.insert(selector.to_string());
        self.history.push(entry);

        // Trim if we exceed max_entries
        if self.history.len() > self.options.max_entries {
            if let Some(oldest) = self.history.first() {
                self.visited_selectors.remove(&oldest.selector);
            }
            self.history.remove(0);
        }

        Ok(())
    }

    /// Checks if a selector has been visited before.
    pub fn has_visited(&self, selector: &str) -> bool {
        self.visited_selectors.contains(selector)
    }

    /// Returns full history of interactions.
    pub fn all(&self) -> &[MemoryEntry] {
        &self.history
    }

    /// Clears all memory
    pub fn clear(&mut self) {
        self.history.clear();
        self.visited_selectors.clear();
    }

    /// Saves memory to a JSON string.
    pub fn to_json(&self) -> Result<String, MemoryError> {
        serde_json::to_string_pretty(&self.history)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))
    }

    /// Loads memory from a JSON string.
    pub fn from_json(json_str: &str, options: MemoryOptions) -> Result<Self, MemoryError> {
        let parsed: Vec<MemoryEntry> = serde_json::from_str(json_str)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;

        let mut memory = Self::new(options);
        for entry in parsed {
            memory.visited_selectors.insert(entry.selector.clone());
            memory.history.push(entry);
        }

        Ok(memory)
    }
}
