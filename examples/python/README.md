# Python Examples

## Setup

First, build the Baldrick Python bindings (requires Rust toolchain + maturin):

### With uv (recommended)

```bash
uv sync                    # install dependencies + build baldrick from source
uv sync --extra ai         # also install anthropic SDK for the AI agent example
```

### With pip

```bash
pip install maturin
cd ../../crates/baldrick-py
maturin develop --release
cd ../../examples/python
pip install anthropic      # for the AI agent example
```

## Run

```bash
# Basic usage
python basic.py                # or: uv run basic.py

# AI agent with Anthropic SDK (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY=sk-ant-...
python ai_agent_anthropic.py   # or: uv run ai_agent_anthropic.py
```

## What's here

| File | Description |
|---|---|
| `basic.py` | Simple expressions, inputs, data processing, snapshot/resume, serialization |
| `ai_agent_anthropic.py` | Claude writes TypeScript code, Baldrick executes it with getWeather/searchFlights tools |
