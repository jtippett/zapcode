# AI Agent Examples (Python)

Two ways to build AI agents with Zapcode in Python.

## Setup

```bash
pip install zapcode zapcode-ai anthropic
# or: uv pip install zapcode zapcode-ai anthropic
export ANTHROPIC_API_KEY=sk-ant-...
```

## Run

```bash
# Recommended — zapcode-ai wrapper
python ai_agent_zapcode_ai.py

# Raw Anthropic SDK + manual snapshot/resume loop
python ai_agent_anthropic.py
```

## What's here

| File | Description |
|---|---|
| `ai_agent_zapcode_ai.py` | **Recommended** — uses `zapcode-ai` wrapper with Anthropic SDK |
| `ai_agent_anthropic.py` | Raw Anthropic SDK + manual snapshot/resume loop |
