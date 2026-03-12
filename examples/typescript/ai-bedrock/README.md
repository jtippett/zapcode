# AWS Bedrock Example (TypeScript)

Zapcode + Vercel AI SDK + AWS Bedrock.

## Prerequisites

AWS credentials must be configured (env vars, `~/.aws/credentials`, or IAM role) with access to the Bedrock model specified by `MODEL_ID` in your target `AWS_REGION`.

## Setup

```bash
npm install
```

## Run

```bash
npm start

# Override model/region
MODEL_ID=eu.anthropic.claude-sonnet-4-20250514-v1:0 AWS_REGION=eu-west-1 npm start
```
