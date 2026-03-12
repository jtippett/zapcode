# Examples

Examples organized by language, then by topic.

```text
examples/
├── typescript/
│   ├── basic/            Simple expressions, inputs, snapshot/resume, classes
│   ├── ai-agent/         AI agent with Anthropic SDK, Vercel AI SDK, zapcode-ai
│   ├── ai-bedrock/       AWS Bedrock integration
│   └── debug-tracing/    Debug mode, autoFix, execution tracing
├── python/
│   ├── basic/            Simple expressions, inputs, snapshot/resume
│   ├── ai-agent/         AI agent with Anthropic SDK, zapcode-ai
│   ├── ai-bedrock/       AWS Bedrock Converse API
│   └── debug-tracing/    Debug mode, autoFix, execution tracing
├── rust/
│   └── basic/            Simple expressions, inputs, snapshot/resume
└── wasm/
    └── basic/            Browser playground (single HTML file)
```

## Quick start

Each example has its own `README.md` with setup and run instructions. Pick a language and topic:

```bash
# TypeScript — basic usage (no API key needed)
cd examples/typescript/basic && npm install && npm start

# Python — basic usage (no API key needed)
cd examples/python/basic && pip install zapcode && python main.py

# Rust — basic usage
cd examples/rust/basic && cargo run --example basic

# WASM — open in browser (macOS: open, Linux: xdg-open, Windows: start)
xdg-open examples/wasm/basic/index.html
```
