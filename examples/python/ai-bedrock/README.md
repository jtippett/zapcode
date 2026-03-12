# AWS Bedrock Example (Python)

Zapcode + AWS Bedrock Converse API.

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

# Override model/region
MODEL_ID=eu.anthropic.claude-sonnet-4-20250514-v1:0 AWS_REGION=eu-west-1 python main.py
```
