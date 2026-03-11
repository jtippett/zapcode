# Python Examples

## Setup

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (for building the native module)
- Python 3.10+ (recommended: [pyenv](https://github.com/pyenv/pyenv))
- [uv](https://docs.astral.sh/uv/) or pip

### Create a virtualenv (recommended)

```bash
pyenv virtualenv 3.13.8 zapcode
pyenv local zapcode
```

### Build the native module

```bash
# Install maturin
uv pip install maturin

# Build and install zapcode
cd ../../crates/zapcode-py
maturin develop --release
```

### With uv (alternative)

```bash
uv sync                    # install dependencies + build zapcode from source
uv sync --extra ai         # also install anthropic SDK for the AI agent example
```

## Run

```bash
# Basic usage (no API key needed)
python basic.py

# AI agent with zapcode-ai wrapper (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY=sk-ant-...
python ai_agent_zapcode_ai.py

# AI agent with raw Anthropic SDK
python ai_agent_anthropic.py
```

## What's here

| File | Description |
|---|---|
| `basic.py` | Simple expressions, inputs, data processing, snapshot/resume, serialization |
| `ai_agent_zapcode_ai.py` | **Recommended** — uses `zapcode-ai` wrapper with Anthropic SDK |
| `ai_agent_anthropic.py` | Raw Anthropic SDK + manual snapshot/resume loop |
