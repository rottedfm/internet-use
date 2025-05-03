pub fn chat_prompt_red_ui() -> String {
    r#"
    (function() {
        if (document.getElementById('iu-prompt-wrapper')) return;

        const style = document.createElement('style');
        style.textContent = `
            #iu-prompt-wrapper {
                position: fixed;
                bottom: 20px;
                left: 20px;
                z-index: 9999;
                display: flex;
                flex-direction: column;
                gap: 12px;
                font-family: sans-serif;
            }

            .iu-box {
                width: 340px;
                padding: 12px;
                background: rgba(0, 0, 0, 0.0);
                color: red;
                border-radius: 10px;
                font-size: 14px;
            }

            #iu-prompt-input {
                width: 100%;
                padding: 10px;
                border: 1px solid red;
                border-radius: 8px;
                background: rgba(0, 0, 0, 0.2);
                color: red;
                font-size: 14px;
            }

            #iu-output-textarea {
                width: 100%;
                height: 120px;
                resize: none;
                background: rgba(0, 0, 0, 0.2);
                border: 1px solid red;
                color: red;
                font-size: 13px;
                padding: 10px;
                border-radius: 8px;
                font-family: monospace;
            }
        `;
        document.head.appendChild(style);

        const wrapper = document.createElement('div');
        wrapper.id = 'iu-prompt-wrapper';

        const promptBox = document.createElement('div');
        promptBox.className = 'iu-box';

        const input = document.createElement('input');
        input.id = 'iu-prompt-input';
        input.placeholder = 'Prompt...';
        input.type = 'text';
        promptBox.appendChild(input);
        wrapper.appendChild(promptBox);

        const outputBox = document.createElement('div');
        outputBox.className = 'iu-box';

        const outputArea = document.createElement('textarea');
        outputArea.id = 'iu-output-textarea';
        outputArea.placeholder = 'Agent output...';
        outputBox.appendChild(outputArea);
        wrapper.appendChild(outputBox);

        document.body.appendChild(wrapper);

        // Flag input as "ready" when Enter is pressed
        input.addEventListener('keydown', function(e) {
            if (e.key === 'Enter') {
                e.preventDefault();
                input.setAttribute('data-submitted', 'true');
            }
        });
    })();
    "#
    .to_string()
}
