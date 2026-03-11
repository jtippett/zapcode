# TypeScript Examples

## Setup

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (for building the native addon)
- [Node.js](https://nodejs.org/) (v18+)

### Build the native addon

```bash
cd ../../crates/zapcode-js
npm install @napi-rs/cli --save-dev
npx napi build --release --platform --js index.js --dts index.d.ts
```

### Install dependencies

```bash
npm install
```

## Run

```bash
# Basic usage (no API key needed)
npm run basic

# AI agent with @unchartedfr/zapcode-ai wrapper (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY=sk-ant-...
npm run agent

# AI agent with raw Anthropic SDK
npm run agent:anthropic

# AI agent with Vercel AI SDK
npm run agent:vercel
```

## What's here

| File | Description |
|---|---|
| `basic.ts` | Simple expressions, inputs, data processing, classes, resource limits |
| `ai-agent-zapcode-ai.ts` | **Recommended** — uses `@unchartedfr/zapcode-ai` wrapper with Vercel AI SDK |
| `ai-agent-anthropic.ts` | Raw Anthropic SDK + manual snapshot/resume loop |
| `ai-agent-vercel-ai.ts` | Vercel AI SDK with manual code generation |
