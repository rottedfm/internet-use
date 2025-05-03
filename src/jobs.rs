use serde::{Deserialize, Serialize};

use crate::BrowserClient;
use crate::types::BrowserError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BrowserJob {
    Navigate(String),
    Click(String),
    Type { selector: String, text: String },
    WaitFor(String),
    ScrollTo(String),
    Screenshot { prefix: String },
}

impl BrowserJob {
    pub async fn run(&self, client: &mut BrowserClient) -> Result<(), BrowserError> {
        match self {
            BrowserJob::Navigate(url) => client.navigate(url).await,
            BrowserJob::Click(selector) => client.click_element(selector).await,
            BrowserJob::Type { selector, text } => {
                client.send_keys_to_element(selector, text).await
            }
            BrowserJob::WaitFor(selector) => client.wait_for_element(selector).await.map(|_| ()),
            BrowserJob::ScrollTo(selector) => client.scroll_to(selector).await,
            BrowserJob::Screenshot { prefix } => {
                let dir = std::path::Path::new("screenshots");
                std::fs::create_dir_all(dir).ok();
                client.capture_screenshot(dir, prefix).await.map(|_| ())
            }
        }
    }
}

pub async fn run_all_jobs(
    client: &mut BrowserClient,
    jobs: &[BrowserJob],
) -> Result<(), BrowserError> {
    for (i, job) in jobs.iter().enumerate() {
        if let Err(err) = job.run(client).await {
            eprintln!("Job {} failed: {:?}", i, err);
            return Err(err);
        }
    }
    Ok(())
}
