mod cli;

use clap::Parser;
use cli::{Cli, Commands};
use internet_use::{
    BrowserClient, BrowserError, BrowserOptions,
    agent::Agent,
    js,
    types::{AgentMemory, MemoryOptions},
};

#[tokio::main]
async fn main() -> Result<(), BrowserError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Open { url } => {
            let mut client =
                BrowserClient::connect(BrowserOptions::default().headless(false)).await?;

            client.navigate(&url).await?;
            client.inject_js(&js::chat_prompt_red_ui()).await?;

            println!("🌐 Browser opened at {url}. Enter prompts in the red box. Ctrl+C to exit.");

            let mut agent = Agent::new("llama3", AgentMemory::new(MemoryOptions::default()));

            loop {
                let prompt_value = client
                    .client
                    .execute(
                        r#"
                        const input = document.getElementById("iu-prompt-input");
                        if (input && input.getAttribute("data-submitted") === "true") {
                            input.setAttribute("data-submitted", "false");
                            return input.value;
                        }
                        return null;
                    "#,
                        vec![],
                    )
                    .await
                    .unwrap();

                if let Some(prompt) = prompt_value.as_str() {
                    if !prompt.trim().is_empty() {
                        println!("🤖 Prompt received: {prompt}");

                        let interactive = client
                            .extract_interactive_elements()
                            .await
                            .unwrap_or_default();
                        let texts = client.extract_text_elements().await.unwrap_or_default();

                        if let Ok(plan) = agent.plan(prompt, &url, &interactive, &texts).await {
                            let js_output = format!(
                                r#"
                                const output = document.getElementById("iu-output-textarea");
                                if (output) {{
                                    output.value = `{}`;
                                }}
                            "#,
                                plan.markdown_todo.replace('`', "\\`") // escape backticks for JS
                            );
                            client.inject_js(&js_output).await?;
                        }
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
}
