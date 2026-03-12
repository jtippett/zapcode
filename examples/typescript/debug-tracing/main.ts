/**
 * Zapcode debug & tracing example.
 *
 * Demonstrates:
 *   - Logging LLM-generated code, tool calls, and output
 *   - `autoFix: true` — catches execution errors and feeds them back to the LLM
 *   - `printTrace()` — displays the execution trace tree with timing
 *
 * Prerequisites:
 *   npm install
 *   AWS credentials configured (env vars, ~/.aws/credentials, or IAM role)
 *
 * Run: npm start
 */

import { zapcode, type ExecutionResult } from "@unchartedfr/zapcode-ai";
import { generateText } from "ai";
import { createAmazonBedrock } from "@ai-sdk/amazon-bedrock";
import { fromNodeProviderChain } from "@aws-sdk/credential-providers";

// --- Bedrock setup ---
const REGION = process.env.AWS_REGION ?? "eu-west-1";

const bedrock = createAmazonBedrock({
  credentialProvider: fromNodeProviderChain(),
  region: REGION,
});

const MODEL_ID = process.env.MODEL_ID ?? "global.amazon.nova-2-lite-v1:0";
const model = bedrock(MODEL_ID);

// --- Zapcode setup with autoFix ---
const { system, tools, printTrace } = zapcode({
  autoFix: true,
  system: "You are a helpful assistant that can look up weather and do math.",
  tools: {
    getWeather: {
      description:
        "Get current weather for a city. Returns { condition: string, temp: number }",
      parameters: {
        city: { type: "string", description: "City name" },
      },
      execute: async ({ city }) => {
        const data: Record<string, { condition: string; temp: number }> = {
          London: { condition: "Overcast", temp: 12 },
          Tokyo: { condition: "Clear", temp: 26 },
          Paris: { condition: "Sunny", temp: 22 },
          "New York": { condition: "Rain", temp: 14 },
        };
        return data[city as string] ?? { condition: "Unknown", temp: 0 };
      },
    },
    searchFlights: {
      description:
        "Search flights between two cities. Returns Array<{ from, to, airline, flight, price, departure }>",
      parameters: {
        from: { type: "string", description: "Departure city" },
        to: { type: "string", description: "Arrival city" },
      },
      execute: async ({ from, to }) => {
        return [
          { from, to, airline: "BA", flight: "BA123", price: 450, departure: "08:00" },
          { from, to, airline: "AF", flight: "AF456", price: 380, departure: "14:30" },
        ];
      },
    },
  },
});

// --- Debug: log each step's generated code, tool calls, and output ---
function logExecution(result: ExecutionResult) {
  // Print the generated code
  const indented = result.code.split("\n").map((l) => "  " + l).join("\n");
  console.log(`\n[zapcode] Code:\n${indented}`);

  // Print each tool call
  for (const tc of result.toolCalls) {
    const argsStr = (tc.args as unknown[]).map((a) => JSON.stringify(a)).join(", ");
    console.log(`[zapcode] Tool call: ${tc.name}(${argsStr}) → ${JSON.stringify(tc.result)}`);
  }

  // Print output or error
  if (result.error) {
    console.log(`[zapcode] Error: ${result.error}`);
  } else {
    console.log(`[zapcode] Output: ${JSON.stringify(result.output)}`);
  }
}

// --- Run ---
async function main() {
  console.log(`Model: ${MODEL_ID} | Region: ${REGION}`);
  console.log(`Debug: ON | AutoFix: ON`);

  const t0 = performance.now();

  const result = await generateText({
    model,
    system,
    tools,
    maxSteps: 10,
    messages: [
      {
        role: "user",
        content:
          "What's the weather in Tokyo and Paris? Find flights from the colder city to the warmer one.",
      },
    ],
    onStepFinish: (step) => {
      // Log every execute_code tool call result
      for (const toolResult of step.toolResults) {
        if (toolResult.toolName === "execute_code") {
          logExecution(toolResult.result as ExecutionResult);
        }
      }
    },
  });

  const totalMs = (performance.now() - t0).toFixed(0);

  console.log("\nAnswer:", result.text);
  console.log(`\n--- Timing ---`);
  console.log(`Total (LLM + Zapcode): ${totalMs}ms`);
  console.log(`Steps: ${result.steps.length}`);
  const toolCallCount = result.steps.reduce(
    (count, step) => count + step.toolCalls.length,
    0,
  );
  console.log(`Tool calls: ${toolCallCount}`);
  console.log(
    `Usage: ${result.usage.promptTokens} prompt + ${result.usage.completionTokens} completion = ${result.usage.totalTokens} tokens`,
  );

  // Print the full execution trace tree
  console.log(`\n--- Execution Trace ---`);
  printTrace();
}

main().catch(console.error);
