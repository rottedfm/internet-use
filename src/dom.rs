use crate::client::{BrowserClient, BrowserError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ElementType {
    Clickable,
    Typable,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteractiveElement {
    pub tag: String,
    pub element_type: ElementType,
    pub selector: String,
    pub text: Option<String>,
    pub attributes: Option<Value>,
    pub label: Option<String>,
}

pub async fn extract_interactive_elements(
    client: &mut BrowserClient,
) -> Result<Vec<InteractiveElement>, BrowserError> {
    let script = r#"
    return (function() {
        try {
            function getLabel(index) {
                let label = '';
                while (index >= 0) {
                    label = String.fromCharCode(65 + (index % 26)) + label;
                    index = Math.floor(index / 26) - 1;
                }
                return label;
            }

            const colors = [
                '#ff595e', '#ffca3a', '#8ac926', '#1982c4',
                '#6a4c93', '#f72585', '#b5179e', '#7209b7',
                '#3a0ca3', '#4361ee', '#4cc9f0', '#f77f00'
            ];

            const elements = Array.from(
                document.querySelectorAll('a, button, input, textarea, select, [role="button"], [tabindex]')
            ).filter(el => {
                const tag = el.tagName.toLowerCase();
                const type = el.getAttribute('type') || '';
                const rect = el.getBoundingClientRect();
                return (
                    el.offsetParent !== null &&
                    rect.width > 0 && rect.height > 0 &&
                    (
                        tag === 'a' ||
                        tag === 'button' ||
                        tag === 'textarea' ||
                        tag === 'select' ||
                        (tag === 'input' && type !== 'hidden') ||
                        el.hasAttribute('onclick') ||
                        el.getAttribute('role') === 'button' ||
                        el.hasAttribute('tabindex')
                    )
                );
            });

            elements.forEach((el, i) => {
                const label = getLabel(i);
                const color = colors[i % colors.length];

                el.setAttribute('data-ai-label', label);
                el.style.outline = `2px solid ${color}`;
                el.style.position = 'relative';

                const badge = document.createElement('div');
                badge.textContent = label;
                badge.style.position = 'absolute';
                badge.style.top = '0';
                badge.style.left = '0';
                badge.style.zIndex = '9999';
                badge.style.background = color;
                badge.style.color = '#fff';
                badge.style.fontSize = '12px';
                badge.style.padding = '2px 5px';
                badge.style.borderRadius = '4px';
                badge.style.fontFamily = 'monospace';
                badge.style.pointerEvents = 'none';

                el.appendChild(badge);
            });

            return elements.map((el, i) => {
                const tag = el.tagName.toLowerCase();
                const isInput = tag === 'input' || tag === 'textarea' || tag === 'select';

                // Improved selector generation
                let selector = tag;
                if (el.id) selector += '#' + el.id;
                if (el.name) selector += `[name="${el.name}"]`;
                if (el.className) selector += '.' + el.className.trim().split(/\s+/).join('.');

                // Capture relevant attributes
                const attributes = {};
                for (let attr of el.attributes) {
                    attributes[attr.name] = attr.value;
                }

                // Ensure we capture common fields used for inputs
                ['name', 'type', 'placeholder'].forEach(attr => {
                    const val = el.getAttribute(attr);
                    if (val && !attributes[attr]) attributes[attr] = val;
                });

                let rawText = (el.innerText || el.value || '').trim();
                if (rawText.length > 1000) rawText = rawText.slice(0, 1000) + '...';

                return {
                    tag,
                    element_type: isInput ? 'typable' : 'clickable',
                    selector,
                    text: rawText || null,
                    attributes,
                    label: el.getAttribute('data-ai-label')
                };
            });
        } catch (err) {
            return { error: err.toString() };
        }
    })();
    "#;

    let value = client
        .client
        .execute(script, vec![])
        .await
        .map_err(|e| BrowserError::OperationError(e.to_string()))?;

    let result: Vec<InteractiveElement> = match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(e) => {
            println!("Failed to parse value: {:#?}", value);
            return Err(BrowserError::OperationError(format!("Invalid JSON: {}", e)));
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::client::BrowserOptions;
    use tokio;

    #[tokio::test]
    async fn test_dom_extraction() {
        let mut client = BrowserClient::connect(BrowserOptions::new()).await.unwrap();

        client
            .navigate("https://duckduckgo.com")
            .await
            .expect("Navigation failed");

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let elements = extract_interactive_elements(&mut client)
            .await
            .expect("DOM extraction failed");

        let json = serde_json::to_string_pretty(&elements).expect("Failed to serialize elements");
        println!("{json}");

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        client.shutdown().await.unwrap();
    }
}
