# AGENTS.md

> Standard agent instructions for the `tyr` project.
> Symlinked as `CLAUDE.md`, `CURSOR.md`, `.github/copilot-instructions.md` for tool compatibility.

---

## What this project is

**`tyr`** (TypeScript-in-Rust) is a minimal, secure TypeScript subset interpreter written in Rust,
designed specifically to execute code written by AI agents. It is the TypeScript equivalent of
[pydantic/monty](https://github.com/pydantic/monty).

The core thesis: LLMs produce faster, cheaper, more reliable results when they write code instead of
making sequential tool calls. `tyr` makes that possible for TypeScript/JavaScript stacks without
containers, sandbox services, or running untrusted code directly on the host.

**What tyr can do:**
- Execute a safe subset of TypeScript — enough for an agent to express what it wants to do
- Block all host access by default: filesystem, env vars, network, `require`, `import`
- Expose host functions to the sandbox — only functions you explicitly register
- Snapshot VM state to bytes at external function call boundaries — resume later in any process
- Start in microseconds (no WASM cold start, no container, no process fork)
- Be called from Rust, TypeScript/JavaScript (napi-rs), or Python (PyO3)
- Enforce resource limits: memory, execution time, stack depth, allocation count

**What tyr cannot do (by design):**
- Access the standard library beyond a safe subset (`console`, `JSON`, `Math`, `Date`, `Array`, `Object`, `Promise`)
- Use `import` / `require` / dynamic imports
- Access `process`, `globalThis`, `eval`, `Function()`, `setTimeout`/`setInterval`
- Define classes with inheritance (plain object literals and closures only — for now)
- Use generators, proxies, WeakMap/WeakRef, or `with` statements

---

## Repository layout

```
tyr/
├── crates/
│   ├── tyr-core/           # Parser integration (oxc), IR, bytecode compiler, VM, snapshot
│   │   ├── src/
│   │   │   ├── parser/     # oxc_parser integration — AST → TyrIR
│   │   │   ├── compiler/   # TyrIR → Bytecode
│   │   │   ├── vm/         # Stack-based bytecode executor
│   │   │   ├── value.rs    # Value enum — the runtime type system
│   │   │   ├── snapshot.rs # Serialize/deserialize mid-execution VM state
│   │   │   ├── sandbox.rs  # Resource limits, host function bridge
│   │   │   └── error.rs    # TyrError — all error types
│   │   └── tests/
│   ├── tyr-js/             # napi-rs bindings → @tyr/core npm package
│   ├── tyr-py/             # PyO3 bindings → tyr-py pip package
│   └── tyr-wasm/           # wasm-bindgen target for browser/edge use
├── examples/
│   ├── basic/
│   ├── aws-lambda/         # Lambda + Bedrock Converse pattern
│   └── snapshot-resume/    # Step Functions / DynamoDB snapshot pattern
├── scripts/
│   ├── startup_perf.ts     # Benchmark: tyr vs QuickJS vs isolated-vm vs Docker
│   └── build_all.sh
├── AGENTS.md               # This file
├── CLAUDE.md -> AGENTS.md
├── Cargo.toml
├── package.json            # workspace root for JS packages
└── README.md
```

---

## Architecture overview

### Parser

`tyr` uses **[oxc_parser](https://github.com/oxc-project/oxc)** — the fastest TypeScript/JavaScript
parser available in Rust. It does NOT use SWC (too heavy) or write its own parser.

The parser phase produces `TyrIR` — a flat, typed intermediate representation that is intentionally
simpler than the full TypeScript AST. Unsupported syntax causes an immediate, descriptive
`TyrError::UnsupportedSyntax` rather than silent failure.

### Supported syntax subset

| Feature | Supported |
|---|---|
| `const`, `let` declarations | ✅ |
| `function`, arrow functions | ✅ |
| `async function`, `await` | ✅ |
| `if` / `else` | ✅ |
| `for...of`, `while` | ✅ |
| `return`, `throw` | ✅ |
| `try` / `catch` | ✅ |
| Object literals `{}` | ✅ |
| Array literals `[]` | ✅ |
| Destructuring | ✅ |
| Template literals | ✅ |
| Optional chaining `?.` | ✅ |
| Nullish coalescing `??` | ✅ |
| Type annotations (stripped) | ✅ |
| `import` / `require` | ❌ sandbox violation |
| `class` (with inheritance) | ❌ not yet |
| `eval`, `Function()` | ❌ sandbox violation |
| Generators | ❌ not yet |
| `process`, `global`, `globalThis` | ❌ sandbox violation |

### Value system

```rust
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<str>),
    Array(HeapRef<Vec<Value>>),
    Object(HeapRef<IndexMap<Arc<str>, Value>>),
    Function(FunctionRef),
    Promise(PromiseState),
}
```

All heap-allocated values use `HeapRef<T>` — a reference-counted wrapper that participates in
`DropWithHeap`. See **Memory safety** section below.

### VM

Stack-based bytecode VM, same model as CPython / Monty. Approximately 40 instructions.

The VM runs synchronously for sync code. For `async` functions, it drives a cooperative executor:
each `await` suspends the current frame and polls registered host futures. The VM never spawns
OS threads.

**Suspension at external calls**: when the VM encounters a call to a registered external function,
it does NOT call the function directly. Instead it:
1. Serializes the current stack frame + continuation into a `TyrSnapshot`
2. Returns `VmState::Suspended { function_name, args, snapshot }`

The caller is responsible for invoking the actual function and calling `snapshot.resume(return_value)`.
This design makes snapshotting trivial — no continuations need to cross async boundaries.

### Snapshotting

`TyrSnapshot` and `TyrRun` implement `serde::Serialize` + `serde::Deserialize`. Snapshots are
small (single-digit kilobytes for typical agent code). They can be stored in DynamoDB, S3, Redis,
or any bytes store.

```rust
// Suspend and serialize
let snapshot: TyrSnapshot = vm.run()?;   // Returns Suspended variant
let bytes: Vec<u8> = snapshot.dump()?;   // serde + bincode

// Resume in another process
let snapshot = TyrSnapshot::load(&bytes)?;
let result = snapshot.resume(return_value)?;
```

---

## Memory safety — CRITICAL

`tyr` will execute untrusted, potentially malicious code. Memory safety is non-negotiable.

### HeapGuard pattern

All types that contain `HeapRef<T>` implement `DropWithHeap`. These MUST be cleaned up correctly
on **every code path** — not just the happy path, but also:
- Early returns via `?`
- `continue` in loops
- Conditional branches that skip cleanup

**Use `heap_guard!` macro (preferred):**
```rust
let arr = heap_guard!(vm.heap, Value::Array(HeapRef::new(vec![])));
// arr is automatically dropped when scope exits, on any path
```

**Never do:**
```rust
let arr = Value::Array(HeapRef::new(vec![]));
if some_condition {
    return Err(...); // LEAK — arr not dropped via DropWithHeap
}
arr.drop_with_heap(&mut vm.heap);
```

A missed `drop_with_heap` leaks reference counts and eventually corrupts the heap. The compiler
will not catch this — it requires discipline and code review.

### Sandbox invariants — NEVER violate

1. **No host filesystem access** — `std::fs`, `std::path`, file descriptors are forbidden in
   `tyr-core`. Filesystem operations are only accessible through explicitly registered host functions.

2. **No env var access** — `std::env::var` is forbidden in `tyr-core`.

3. **No network access** — `std::net`, `tokio::net`, `reqwest` etc. are forbidden in `tyr-core`.

4. **No `eval` equivalent** — there is no mechanism to compile new code at runtime from within
   the sandbox.

5. **Resource limits are enforced before execution** — memory limit, execution time limit, and
   stack depth limit are checked before each instruction dispatch, not just at external calls.

If you are ever unsure whether something violates sandbox invariants: it does. Ask before merging.

---

## Development commands

```bash
# Build all crates
make build

# Run all tests
make test

# Build JS bindings (debug)
make build-js

# Build JS bindings (release, for publishing)
make build-js-release

# Run startup performance benchmark
make bench

# Lint (clippy + fmt check)
make lint

# Format
make format

# Build Python bindings
make build-py

# Build WASM target
make build-wasm
```

---

## Testing philosophy

Every language feature in the supported subset MUST have:
1. A positive test (it executes correctly)
2. A negative test (unsupported syntax produces the right error)
3. A sandbox escape test (no way to access host resources through this feature)

Tests live in `crates/tyr-core/tests/`. Name files after the feature: `array_methods.rs`,
`async_await.rs`, `snapshot_resume.rs`, etc.

Snapshot tests use `insta` for readable diffs on serialized state.

Performance regression tests run in CI via `criterion`. The startup latency target is **< 1ms**
for first execution after binary load.

---

## Language bindings

### TypeScript / JavaScript (napi-rs)

Public API in `crates/tyr-js/`. Follow the `@pydantic/monty` API shape as closely as possible —
this makes migration between the two easy.

```typescript
import { Tyr, TyrSnapshot, runTyrAsync } from '@tyr/core'

// Basic sync
const t = new Tyr('x + 1', { inputs: ['x'] })
const result = t.run({ inputs: { x: 41 } })  // 42

// External functions + async
const t2 = new Tyr(`const data = await fetchData(url); return data.length`, {
  inputs: ['url'],
  externalFunctions: ['fetchData'],
})

const result2 = await runTyrAsync(t2, {
  inputs: { url: 'https://example.com' },
  externalFunctions: {
    fetchData: async (url: string) => ({ length: 42 }),
  },
})

// Iterative / snapshot
let progress = t2.start({ inputs: { url: 'https://example.com' } })
if (progress instanceof TyrSnapshot) {
  const bytes = progress.dump()
  // store bytes somewhere...
  progress = TyrSnapshot.load(bytes).resume({ returnValue: { length: 42 } })
}
```

### Python (PyO3)

Public API in `crates/tyr-py/`. Mirror `pydantic_monty` API shape.

### Lambda / napi prebuilt binaries

Ship prebuilt binaries for:
- `linux-x64-gnu` (Lambda x86_64)
- `linux-arm64-gnu` (Lambda arm64 / Graviton)
- `darwin-x64`, `darwin-arm64` (local dev)
- `win32-x64-msvc`

Use `@napi-rs/cli` matrix build in CI. Users never need a C++ compiler.

---

## What good looks like

A feature is complete when:
- It passes the feature test suite
- It passes the sandbox escape suite
- The performance benchmark shows no regression
- The JS and Python APIs expose it with proper types
- A usage example exists in `examples/`
- CHANGELOG.md is updated

A PR that adds a feature without tests will not be merged.

---

## Relationship to other projects

- **[pydantic/monty](https://github.com/pydantic/monty)** — direct inspiration, Python equivalent.
  Study its architecture. Respect its design decisions. Don't copy its code.
- **[oxc](https://github.com/oxc-project/oxc)** — parser dependency. Don't replace it.
- **[boa](https://github.com/boa-dev/boa)** — full JS interpreter in Rust. Useful as reference for
  VM design. Not a dependency — too heavy.
- **[isolated-vm](https://github.com/laverdet/isolated-vm)** — V8-based alternative. `tyr` is
  lighter, snapshotable, and doesn't require a C++ compiler on the user's machine.
- **[@sebastianwessel/quickjs](https://github.com/sebastianwessel/quickjs)** — WASM-based alternative.
  `tyr` starts faster and supports snapshotting. Not a drop-in replacement — different security model.
