use langchain_rust::{language_models::llm::LLM, llm::ollama::client::Ollama};

pub struct OllamaAgent {
    model: Ollama,
}

impl OllamaAgent {
    pub fn new(model_name: &str) -> Self {
        let model = Ollama::default().with_model(model_name);
        Self { model }
    }

    pub async fn ask(&self, prompt: &str) -> Result<String, String> {
        self.model
            .invoke(prompt)
            .await
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("LLM error: {}", e))
    }
}
