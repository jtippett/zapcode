"""
Zapcode debug & tracing example (Python).

Demonstrates:
  - Logging LLM-generated code, tool calls, and output
  - auto_fix=True — catches execution errors and feeds them back to the LLM
  - print_trace() — displays the execution trace tree with timing

Prerequisites:
  pip install zapcode-ai boto3
  AWS credentials configured (env vars, ~/.aws/credentials, or IAM role)

Run: python main.py
"""

import json
import os
import time

import boto3
from zapcode_ai import zapcode, ToolDefinition, ParamDef


# --- Bedrock setup ---
REGION = os.environ.get("AWS_REGION", "eu-west-1")
MODEL_ID = os.environ.get("MODEL_ID", "global.amazon.nova-2-lite-v1:0")

bedrock = boto3.client("bedrock-runtime", region_name=REGION)


# --- Tools ---
def get_weather(args):
    data = {
        "London": {"condition": "Overcast", "temp": 12},
        "Tokyo": {"condition": "Clear", "temp": 26},
        "Paris": {"condition": "Sunny", "temp": 22},
        "New York": {"condition": "Rain", "temp": 14},
    }
    return data.get(args["city"], {"condition": "Unknown", "temp": 0})


def search_flights(args):
    origin = args["from"]
    destination = args["to"]
    return [
        {"from": origin, "to": destination, "airline": "BA", "flight": "BA123", "price": 450, "departure": "08:00"},
        {"from": origin, "to": destination, "airline": "AF", "flight": "AF456", "price": 380, "departure": "14:30"},
    ]


# --- Zapcode setup with auto_fix ---
zap = zapcode(
    auto_fix=True,
    system="You are a helpful assistant that can look up weather and do math.",
    tools={
        "getWeather": ToolDefinition(
            description="Get current weather for a city. Returns { condition: string, temp: number }",
            parameters={"city": ParamDef(type="string", description="City name")},
            execute=get_weather,
        ),
        "searchFlights": ToolDefinition(
            description="Search flights between two cities. Returns Array<{ from, to, airline, flight, price, departure }>",
            parameters={
                "from": ParamDef(type="string", description="Departure city"),
                "to": ParamDef(type="string", description="Arrival city"),
            },
            execute=search_flights,
        ),
    },
)


# --- Debug: log generated code, tool calls, and output ---
def log_execution(result):
    # Print the generated code
    indented = "\n".join("  " + line for line in result.code.split("\n"))
    print(f"\n[zapcode] Code:\n{indented}")

    # Print each tool call
    for tc in result.tool_calls:
        args_str = ", ".join(json.dumps(a, default=str) for a in tc["args"])
        print(f"[zapcode] Tool call: {tc['name']}({args_str}) → {json.dumps(tc['result'], default=str)}")

    # Print output or error
    if result.error:
        print(f"[zapcode] Error: {result.error}")
    else:
        print(f"[zapcode] Output: {json.dumps(result.output, default=str)}")


def main():
    print(f"Model: {MODEL_ID} | Region: {REGION}")
    print("Debug: ON | AutoFix: ON")

    t0 = time.perf_counter()

    messages = [
        {"role": "user", "content": [{"text": "What's the weather in Tokyo and Paris? Find flights from the colder city to the warmer one."}]}
    ]

    tool_config = {
        "tools": [
            {
                "toolSpec": {
                    "name": "execute_code",
                    "description": "Execute TypeScript code in a secure sandbox. The code can call the available tool functions using await. The last expression is the return value.",
                    "inputSchema": {
                        "json": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "TypeScript code to execute in the sandbox",
                                }
                            },
                            "required": ["code"],
                        }
                    },
                }
            }
        ]
    }

    max_steps = 10
    steps = 0
    total_tokens = 0

    while steps < max_steps:
        steps += 1
        response = bedrock.converse(
            modelId=MODEL_ID,
            messages=messages,
            system=[{"text": zap.system}],
            toolConfig=tool_config,
        )

        total_tokens += response["usage"]["inputTokens"] + response["usage"]["outputTokens"]
        stop_reason = response["stopReason"]

        if stop_reason == "tool_use":
            assistant_content = response["output"]["message"]["content"]
            messages.append({"role": "assistant", "content": assistant_content})

            tool_results = []
            for block in assistant_content:
                if "toolUse" in block:
                    tool_use = block["toolUse"]
                    code = tool_use["input"]["code"]
                    result = zap.handle_tool_call(code)

                    # Debug: log the execution
                    log_execution(result)

                    if result.error:
                        tool_results.append({
                            "toolResult": {
                                "toolUseId": tool_use["toolUseId"],
                                "content": [{"text": result.error}],
                                "status": "error",
                            }
                        })
                    else:
                        tool_results.append({
                            "toolResult": {
                                "toolUseId": tool_use["toolUseId"],
                                "content": [{"json": {"output": result.output, "stdout": result.stdout}}],
                            }
                        })

            messages.append({"role": "user", "content": tool_results})
        elif stop_reason in ("end_turn", "stop_sequence"):
            text = ""
            for block in response["output"]["message"]["content"]:
                if "text" in block:
                    text += block["text"]

            total_ms = (time.perf_counter() - t0) * 1000

            print(f"\nAnswer: {text}")
            print("\n--- Timing ---")
            print(f"Total (LLM + Zapcode): {total_ms:.0f}ms")
            print(f"Steps: {steps}")
            print(f"Tokens: {total_tokens}")

            # Print the full execution trace tree
            print("\n--- Execution Trace ---")
            zap.print_trace()
            return
        else:
            raise RuntimeError(
                f"Bedrock Converse returned unexpected stop reason: {stop_reason}"
            )

    raise RuntimeError(
        f"Model did not produce a final answer within {max_steps} steps"
    )


if __name__ == "__main__":
    main()
