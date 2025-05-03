use crate::jobs::BrowserJob;
use crate::types::{AgentMemory, BrowserError, InteractiveElement, MemoryEntry, TextElement};
use ollama_rs::{Ollama, generation::completion::request::GenerationRequest, models::ModelOptions};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Agent {
    ollama: Ollama,
    model: String,
    pub memory: AgentMemory,
    pub temperature: f32,
    pub executed_jobs: Vec<BrowserJob>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentPlan {
    pub markdown_todo: String,
    pub jobs: Vec<BrowserJob>,
}

impl Agent {
    pub fn new(model: &str, memory: AgentMemory) -> Self {
        Self {
            ollama: Ollama::default(),
            model: model.to_string(),
            memory,
            temperature: 0.4,
            executed_jobs: vec![],
        }
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    pub async fn plan(
        &self,
        user_prompt: &str,
        current_url: &str,
        interactive_elements: &[InteractiveElement],
        text_elements: &[TextElement],
    ) -> Result<AgentPlan, BrowserError> {
        let history_json = self.memory.to_json()?;
        let interact = serde_json::to_string_pretty(interactive_elements).unwrap_or_default();
        let text = serde_json::to_string_pretty(text_elements).unwrap_or_default();

        let few_shot = r#"Task: Search for 'Rust async book' and open the first result
Checklist:
- [x] Navigate to https://duckduckgo.com
- [x] Type 'Rust async book' in the search input
- [x] Click the first result link

Jobs:
```json
[
  {"Navigate": "https://duckduckgo.com"},
  {"Type": {"selector": "input[name=q]", "text": "Rust async book"}},
  {"Click": ".result__a"}
]
```"#;

        let context = format!(
            "Step 1: You are a senior web automation engineer. Analyze this user task:\n> {user_prompt}\n\nStep 2: Reason step-by-step using the context below and determine how to solve it.\n\nStep 3: You are now a markdown expert. Write a checklist of the required browser actions in markdown.\n\nStep 4: You are now a JSON expert. Output a list of BrowserJobs that complete the task using this format:\n```json\n[{{ \"Navigate\": \"url\" }}, {{ \"Type\": {{ \"selector\": \"selector\", \"text\": \"value\" }} }}, {{ \"Click\": \"selector\" }}]\n```\n\nContext:\nURL: {current_url}\nMemory: {history_json}\nInteractive Elements: {interact}\nText Elements: {text}\n\nExample:\n{few_shot}"
        );

        let req = GenerationRequest::new(self.model.clone(), context)
            .options(ModelOptions::default().temperature(self.temperature));

        let res = self
            .ollama
            .generate(req)
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        let output = res.response.trim();
        let (markdown, jobs_json) = Self::split_plan_response(output)?;
        let jobs: Vec<BrowserJob> = serde_json::from_str(jobs_json)
            .map_err(|e| BrowserError::OperationError(format!("Failed to parse jobs JSON: {e}")))?;

        Ok(AgentPlan {
            markdown_todo: markdown.to_string(),
            jobs,
        })
    }

    fn split_plan_response(response: &str) -> Result<(&str, &str), BrowserError> {
        let parts: Vec<&str> = response.splitn(2, "```json").collect();
        if parts.len() != 2 {
            return Err(BrowserError::OperationError(
                "Missing JSON block in LLM output".to_string(),
            ));
        }
        let markdown = parts[0].trim();
        let json_block = parts[1].split("```\n").next().unwrap_or("").trim();
        Ok((markdown, json_block))
    }

    pub async fn run_jobs(
        &mut self,
        jobs: Vec<BrowserJob>,
        page_url: Option<String>,
        client: &mut crate::BrowserClient,
    ) -> Result<(), BrowserError> {
        for job in jobs.clone() {
            let mut attempts = 0;
            loop {
                match job.run(client).await {
                    Ok(_) => {
                        let entry = MemoryEntry::new(&job, page_url.clone());
                        self.memory.add(entry);
                        self.executed_jobs.push(job.clone());
                        break;
                    }
                    Err(e) if attempts < 2 => {
                        attempts += 1;
                        eprintln!("Retrying job: {job:?} due to error: {e}");
                    }
                    Err(e) => {
                        eprintln!("Agent failed to run job: {job:?} - {e}");
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn evaluate_instruction_adherence(
        &self,
        planned_jobs: &[BrowserJob],
    ) -> Result<f32, BrowserError> {
        if planned_jobs.is_empty() {
            return Err(BrowserError::OperationError(
                "No planned jobs to evaluate.".into(),
            ));
        }

        let total = planned_jobs.len();
        let matched = planned_jobs
            .iter()
            .zip(&self.executed_jobs)
            .filter(|(a, b)| a == b)
            .count();

        Ok(matched as f32 / total as f32)
    }

    pub async fn llm_judge_evaluation(
        &self,
        instruction: &str,
        executed_summary: &str,
    ) -> Result<String, BrowserError> {
        let prompt = format!(
            "Instruction: {instruction}\nExecuted: {executed_summary}\n\nDid these actions follow the instruction? Explain briefly."
        );

        let req = GenerationRequest::new(self.model.clone(), prompt)
            .options(ModelOptions::default().temperature(self.temperature));

        let res = self
            .ollama
            .generate(req)
            .await
            .map_err(|e| BrowserError::OperationError(e.to_string()))?;

        Ok(res.response.trim().to_string())
    }
}
