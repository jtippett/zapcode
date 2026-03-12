# Debug & Tracing Example

Demonstrates Zapcode's debug mode, auto-fix error recovery, and execution tracing.

## Features

- **`debug: true`** — Prints the LLM-generated code, external tool calls, and output for each execution
- **`autoFix: true`** — When the LLM generates code that fails, the error is returned as a tool result instead of throwing, letting the LLM self-correct on the next step
- **`printTrace()`** — Displays the full execution trace tree (parse -> compile -> execute) with timing

## Setup

```bash
npm install
```

## Run

```bash
# Default model (Amazon Nova)
npm start

# With a specific model
MODEL_ID=anthropic.claude-sonnet-4-20250514 npm start
```

## Example output

```
Model: global.amazon.nova-2-lite-v1:0 | Region: eu-west-1
Debug: ON | AutoFix: ON

[zapcode] Code:
  const tokyo = await getWeather("Tokyo");
  const paris = await getWeather("Paris");
  const colder = tokyo.temp < paris.temp ? "Tokyo" : "Paris";
  const warmer = tokyo.temp < paris.temp ? "Paris" : "Tokyo";
  const flights = await searchFlights(colder, warmer);
  flights;

[zapcode] Tool call: getWeather("Tokyo") -> {"condition":"Clear","temp":26}
[zapcode] Tool call: getWeather("Paris") -> {"condition":"Sunny","temp":22}
[zapcode] Tool call: searchFlights("Paris", "Tokyo") -> [...]
[zapcode] Output: [{"from":"Paris","to":"Tokyo",...}]

--- Execution Trace ---
session [zapcode.tools: getWeather, searchFlights]     12.4ms
  attempt_1                                             8.2ms
    parse                                               0.1ms
    compile                                             0.0ms
    execute                                             8.1ms
```
