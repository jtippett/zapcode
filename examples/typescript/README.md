# TypeScript Examples

## Setup

First, build the Baldrick native addon (requires Rust toolchain):

```bash
cd ../../crates/baldrick-js
cargo build -p baldrick-js --release
```

Then install dependencies:

```bash
# Pick your package manager
npm install
yarn install
pnpm install
bun install
```

## Run

```bash
# Basic usage
npm run basic          # or: bun run basic / yarn basic / pnpm basic

# AI agent with Anthropic SDK (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY=sk-ant-...
npm run agent:anthropic

# AI agent with Vercel AI SDK (requires ANTHROPIC_API_KEY)
npm run agent:vercel
```

## What's here

| File | Description |
|---|---|
| `basic.ts` | Simple expressions, inputs, data processing, classes, resource limits |
| `ai-agent-anthropic.ts` | Claude writes TypeScript code, Baldrick executes it with getWeather/searchFlights tools |
| `ai-agent-vercel-ai.ts` | Same pattern using Vercel AI SDK (`ai` + `@ai-sdk/anthropic`) |
