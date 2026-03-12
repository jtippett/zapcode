# Debug & Tracing Example (Python)

Demonstrates Zapcode's debug mode, auto-fix error recovery, and execution tracing.

## Features

- **`debug=True`** — Prints the LLM-generated code, external tool calls, and output for each execution
- **`auto_fix=True`** — When the LLM generates code that fails, the error is returned as a tool result instead of raising, letting the LLM self-correct on the next step
- **`print_trace()`** — Displays the full execution trace tree (parse -> compile -> execute) with timing

## Prerequisites

AWS credentials must be configured (env vars, `~/.aws/credentials`, or IAM role) with access to the Bedrock model specified by `MODEL_ID` in your target `AWS_REGION`.

## Setup

```bash
pip install zapcode-ai boto3
# or: uv pip install zapcode-ai boto3
```

## Run

```bash
python main.py

# With a specific model
MODEL_ID=anthropic.claude-sonnet-4-20250514 python main.py
```
