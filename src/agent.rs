use crate::dom::{ElementType, InteractiveElement};
use ollama_rs::{Ollama, generation::completion::request::GenerationRequest, models::ModelOptions};
use std::collections::HashMap;

pub struct Agent {
    model_name: String,
    ollama: Ollama,
    pub max_label_attempts: usize,
    pub temperature: f32,
}

impl Agent {
    pub fn new(model: &str) -> Self {
        let ollama = Ollama::default();
        Self {
            model_name: model.to_string(),
            max_label_attempts: 5,
            ollama,
            temperature: 0.4,
        }
    }

    pub fn set_model(&mut self, new_model: &str) {
        self.model_name = new_model.to_string();
    }

    pub fn with_label_attempts(mut self, attempts: usize) -> Self {
        self.max_label_attempts = attempts;
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    pub fn model(&self) -> &str {
        &self.model_name
    }

    pub async fn ask(&self, prompt: &str) -> Result<String, String> {
        let options = ModelOptions::default().temperature(self.temperature);
        let request =
            GenerationRequest::new(self.model_name.clone(), prompt.to_string()).options(options);

        match self.ollama.generate(request).await {
            Ok(response) => Ok(response.response.trim().to_string()),
            Err(e) => Err(format!("LLM error: {}", e)),
        }
    }

    async fn prompt_for_label(
        &self,
        instruction: &str,
        elements: &[&InteractiveElement],
    ) -> Result<String, String> {
        let label_map: HashMap<String, &InteractiveElement> = elements
            .iter()
            .filter_map(|el| el.label.as_ref().map(|l| (l.clone(), *el)))
            .collect();

        let summary = elements
            .iter()
            .map(|el| {
                format!(
                    "[{}] <{}> - text: \"{}\", selector: {}",
                    el.label.clone().unwrap_or("?".into()),
                    el.tag,
                    el.text.clone().unwrap_or_default(),
                    el.selector
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        for attempt in 0..self.max_label_attempts {
            let prompt = if attempt == 0 {
                format!(
                    r#"
You are controlling a headless browser. Here are the visible elements:

{summary}

Instruction: "{instruction}"

Respond ONLY with the label of the best matching element (e.g., "A", "B"). Do NOT add explanations.
"#
                )
            } else {
                format!(
                    r#"
Your previous answer was invalid — you must ONLY return a label from this list: [{}].

Try again. Respond with just one label.

Instruction: "{instruction}"
"#,
                    label_map.keys().cloned().collect::<Vec<_>>().join(", ")
                )
            };

            let response = self.ask(&prompt).await?;
            let label = response.trim();

            if label_map.contains_key(label) {
                return Ok(label.to_string());
            } else {
                println!(
                    "[Agent] Attempt {} failed. Got '{}', not in DOM labels.",
                    attempt + 1,
                    label
                );
            }
        }

        Err(format!(
            "Agent failed to return a valid label after {} attempts.",
            self.max_label_attempts
        ))
    }

    async fn infer_action(&self, instruction: &str) -> Result<ElementType, String> {
        let prompt = format!(
            r#"
Given the instruction below, determine whether the user wants to CLICK something or TYPE something into a form.

Instruction: "{instruction}"

Respond with exactly one word: "click" or "type"
            "#
        );

        let response = self.ask(&prompt).await?;
        match response.to_lowercase().as_str() {
            "click" => Ok(ElementType::Clickable),
            "type" => Ok(ElementType::Typable),
            other => Err(format!("Invaild action returned: {}", other)),
        }
    }

    fn filter_elements<'a>(
        &self,
        elements: &'a [InteractiveElement],
        action: &ElementType,
    ) -> Vec<&'a InteractiveElement> {
        elements
            .iter()
            .filter(|el| &el.element_type == action)
            .collect()
    }

    pub async fn decide_label(
        &self,
        instruction: &str,
        elements: &[InteractiveElement],
    ) -> Result<String, String> {
        let action = self.infer_action(instruction).await?;
        let filtered = self.filter_elements(elements, &action);

        if filtered.is_empty() {
            return Err(format!(
                "No elements found matching inferred action: {:?}",
                action
            ));
        }

        self.prompt_for_label(instruction, &filtered).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{BrowserClient, BrowserOptions};
    use crate::dom::{ElementType, extract_interactive_elements};
    use tokio;

    #[tokio::test]
    async fn test_agent_decide_label() {
        let mut browser = BrowserClient::connect(BrowserOptions::new()).await.unwrap();

        browser
            .navigate("https://www.youtube.com/feed/trending?bp=6gQJRkVleHBsb3Jl")
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let elements = extract_interactive_elements(&mut browser).await.unwrap();

        assert!(!elements.is_empty(), "No interactive elements found!");

        let agent = Agent::new("llama3.2")
            .with_temperature(0.4)
            .with_label_attempts(5);

        let instruction = "Find a movie trailer and click on it.";

        let label = agent.decide_label(instruction, &elements).await.unwrap();

        println!("Instruction: {}", instruction);
        println!("Selected label: {}", label);

        let found = elements
            .iter()
            .find(|el| el.label.as_deref() == Some(&label))
            .expect("Returned label not found in DOM");

        println!(
            "Matched element: <{}> - {}",
            found.tag,
            found.text.clone().unwrap_or_default()
        );

        // Optionally click the element (if clickable)
        if found.element_type == ElementType::Clickable {
            browser.click_element(&found.selector).await.unwrap();
            println!("Clicked element with selector: {}", &found.selector);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        browser.shutdown().await.expect("Browser shutdown failed");
    }
}
