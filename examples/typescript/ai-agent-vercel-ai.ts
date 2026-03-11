/**
 * AI Agent with Baldrick — using Vercel AI SDK + Anthropic.
 *
 * This example uses the Vercel AI SDK's `generateText` to ask Claude to
 * write TypeScript code, then executes it in Baldrick's sandbox.
 *
 * The pattern: instead of defining tools as JSON schema for the LLM to call
 * via tool_use, you let the LLM write code that calls typed functions.
 * This gives the LLM more expressive power (loops, conditionals, variables)
 * while Baldrick keeps execution sandboxed.
 *
 * Prerequisites:
 *   npm install ai @ai-sdk/anthropic @baldrick/core
 *   export ANTHROPIC_API_KEY=sk-...
 *
 * Run with: npx ts-node examples/typescript/ai-agent-vercel-ai.ts
 */

import { generateText } from "ai";
import { anthropic } from "@ai-sdk/anthropic";
import { Baldrick, BaldrickSnapshotHandle } from "@baldrick/core";

// --- Tool implementations ---

async function getWeather(city: string): Promise<{
  condition: string;
  temp: number;
  humidity: number;
}> {
  // Replace with a real API call
  const data: Record<string, any> = {
    London: { condition: "Overcast", temp: 12, humidity: 80 },
    Paris: { condition: "Sunny", temp: 22, humidity: 45 },
    Tokyo: { condition: "Clear", temp: 26, humidity: 55 },
  };
  return data[city] ?? { condition: "Unknown", temp: 0, humidity: 0 };
}

async function sendEmail(
  to: string,
  subject: string,
  body: string
): Promise<{ sent: boolean; messageId: string }> {
  console.log(`  [mock] Sending email to ${to}: "${subject}"`);
  return { sent: true, messageId: "msg_" + Math.random().toString(36).slice(2) };
}

const tools: Record<string, (...args: any[]) => Promise<any>> = {
  getWeather,
  sendEmail,
};

// --- Execute AI-generated code in Baldrick ---

async function executeInSandbox(
  code: string,
  inputs: Record<string, any>
): Promise<any> {
  const sandbox = new Baldrick(code, {
    inputs: Object.keys(inputs),
    externalFunctions: Object.keys(tools),
    timeLimitMs: 10000,
    memoryLimitMb: 16,
  });

  let state = sandbox.start(inputs);

  // Snapshot/resume loop: resolve each external call as it comes
  while (!state.completed) {
    const { functionName, args } = state;
    console.log(`  [sandbox] calling ${functionName}(${JSON.stringify(args)})`);

    const toolFn = tools[functionName];
    if (!toolFn) throw new Error(`Unknown function: ${functionName}`);

    const result = await toolFn(...args);
    console.log(`  [sandbox] ${functionName} ->`, result);

    const snapshot = BaldrickSnapshotHandle.load(state.snapshot);
    state = snapshot.resume(result);
  }

  return state.output;
}

// --- The agent ---

async function agent(userMessage: string) {
  const { text: code } = await generateText({
    model: anthropic("claude-sonnet-4-20250514"),
    system: `You write TypeScript code to answer user questions.
Available async functions (use await):
- getWeather(city: string) → { condition, temp, humidity }
- sendEmail(to: string, subject: string, body: string) → { sent, messageId }

Available inputs: userMessage (string)

Return ONLY TypeScript code. The last expression is the output value.
Do not wrap in markdown code fences.`,
    prompt: userMessage,
  });

  console.log("Generated code:\n", code, "\n");

  const result = await executeInSandbox(code, { userMessage });
  console.log("Result:", result);
  return result;
}

// --- Run ---

async function main() {
  console.log("=== Weather report ===\n");
  await agent("Get the weather in Paris and London, tell me which is warmer.");

  console.log("\n=== Weather + email ===\n");
  await agent(
    "Check the weather in Tokyo and email a summary to travel@example.com"
  );
}

main().catch(console.error);
