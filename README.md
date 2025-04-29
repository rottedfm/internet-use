# fuji

> A minimal AI-powered browser automation library written in Rust — fast, focused, and inspired by [`browser-use`](https://github.com/browser-use/browser-use).

---

## ⚠️ Disclaimer

This project is a **from-scratch rewrite of [`browser-use`](https://github.com/browser-use/browser-use)** with the goal of improving logic, performance, and flexibility using Rust. It is not a drop-in replacement, but aims to achieve similar functionality in a lighter, safer, and faster implementation.

---

## 🎯 Project Goals

- Keep the codebase **minimal and composable**
- Focus on **LLM-guided decision making** over scripted automation
- Improve **action reliability**, **DOM understanding**, and **retry behavior**
- Avoid bloated dependencies or overengineered abstractions

---

## ✅ Core Components

- `BrowserClient`: Manages WebDriver sessions, tab control, navigation, actions
- `Agent`: Interprets LLM output and determines next steps
- `DOM`: Extracts relevant elements and text from pages
- `Memory`: Lightweight session memory (clicked elements, visited labels)

---

## 📋 TODO: Minimal Fully Functional Clone

### 🧠 Agent Logic
- [ ] Use structured JSON outputs from LLM (no free-text parsing)
- [ ] Add basic agent loop: `observe → plan → act → reflect`
- [ ] Support retry logic per element, not per prompt
- [ ] Add `detect_popup()` function to identify modal overlays or alerts
- [ ] Add basic prompt builder using visible DOM text as context
- [ ] Add a way to give users feedback on their prompt

### 🧭 BrowserClient Actions
- [x] `click_element(selector)`
- [x] `type_into(selector, text)`
- [x] `wait_for_element(selector)`
- [x] `navigate_to(url)`
- [x] `open_tab()` / `close_tab()` / `switch_tab(index)`
- [X] `scroll_to(selector)`
- [X] `capture_screenshot()`
- [X] `extract_elements_with_text()`
- [X] `push_browser_log` 
- [ ] `detect_modal_or_popup()` ← for agent use


### 💾 Memory
- [x] Track clicked elements
- [x] Track label attempts
- [ ] Add session log format: `prompt`, `DOM hash`, `action`, `result`
- [ ] Add pushing memory to browser log
- [ ] Add text memory

### 🧪 Testing
- [ ] Dedicated test script
- [ ] Integration tests with static test pages (form, login, modal)
- [ ] Validate agent behavior with known prompts
- [ ] Log actions with timestamps for audit/debug

