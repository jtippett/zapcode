# AI Agent Examples (TypeScript)

Three ways to build AI agents with Zapcode, from high-level to low-level.

## Setup

```bash
npm install
export ANTHROPIC_API_KEY=sk-ant-...
```

## Run

```bash
# Recommended — zapcode-ai wrapper
npm run agent

# Vercel AI SDK with streamText
npm run agent:vercel

# Raw Anthropic SDK + manual snapshot/resume loop
npm run agent:anthropic
```

## What's here

| File | Description |
|---|---|
| `ai-agent-zapcode-ai.ts` | **Recommended** — uses `@unchartedfr/zapcode-ai` wrapper with Vercel AI SDK |
| `ai-agent-vercel-ai.ts` | Vercel AI SDK with `generateText` and `streamText` |
| `ai-agent-anthropic.ts` | Raw Anthropic SDK + manual snapshot/resume loop |
