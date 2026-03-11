# CLAUDE.md

> AI coding assistant instructions for the `tyr` project.
> See AGENTS.md for the full architecture reference. This file adds
> Claude-Code-specific guidance on top of it.

---

## Read AGENTS.md first

Before writing any code in this repository, read `AGENTS.md` in full. It defines:
- What `tyr` is and what it must never do
- The full architecture (parser → IR → bytecode → VM → snapshot)
- The `HeapGuard` / `DropWithHeap` memory safety pattern
- The five sandbox invariants that must never be violated
- The definition of "done" for any feature

Do not skip this. The sandbox invariants in particular will save you from introducing
security vulnerabilities that are hard to detect and easy to ship.

---

## Codebase orientation

Start here when working on a new area:

| Area | Entry point |
|---|---|
| Parsing TypeScript | `crates/tyr-core/src/parser/mod.rs` |
| IR definition | `crates/tyr-core/src/parser/ir.rs` |
| Bytecode instructions | `crates/tyr-core/src/compiler/instruction.rs` |
| VM main loop | `crates/tyr-core/src/vm/mod.rs` |
| Value / type system | `crates/tyr-core/src/value.rs` |
| Snapshot / resume | `crates/tyr-core/src/snapshot.rs` |
| Resource limits | `crates/tyr-core/src/sandbox.rs` |
| JS bindings API | `crates/tyr-js/src/lib.rs` |
| Python bindings API | `crates/tyr-py/src/lib.rs` |

When in doubt about where something belongs: `tyr-core` is pure Rust with zero I/O. Bindings
crates only translate types and marshal calls into `tyr-core`. Never put business logic in
binding crates.

---

## How to add a new language feature

1. **Check the supported subset table in AGENTS.md first.** If the feature is explicitly listed
   as unsupported, do not add it without opening a discussion. Features are excluded intentionally.

2. **Add parser support** in `crates/tyr-core/src/parser/`. The parser walks the `oxc` AST and
   emits `TyrIR`. Unsupported nodes must emit `TyrError::UnsupportedSyntax` with the node's
   span information.

3. **Add compiler support** in `crates/tyr-core/src/compiler/`. The compiler lowers `TyrIR`
   to bytecode instructions. Add new `Instruction` variants only when necessary — prefer
   reusing existing instructions.

4. **Add VM dispatch** in `crates/tyr-core/src/vm/mod.rs`. The main `dispatch()` function
   matches on `Instruction`. Every new instruction needs:
   - Correct stack discipline (verify push/pop balance with the stack depth tracker)
   - Resource limit check before any allocation
   - `heap_guard!` for any `HeapRef` values created mid-dispatch

5. **Write tests** before considering the feature done. See AGENTS.md testing philosophy.

6. **Update JS and Python bindings** if the feature affects the public API surface.

---

## Memory safety checklist

Before submitting any code that touches `HeapRef`, `DropWithHeap`, or the VM dispatch loop,
mentally walk every code path:

- [ ] Does every `HeapRef` creation have a corresponding `heap_guard!` wrapper?
- [ ] Does every early return via `?` correctly drop all heap values in scope?
- [ ] Does every `continue` in a VM loop correctly drop values that won't reach end-of-scope?
- [ ] Does every conditional branch that returns early drop values from the non-returning branches?

The compiler cannot verify `DropWithHeap` discipline. You must reason about it manually.
If you are not certain, add a comment explaining why the drop is safe.

---

## Sandbox invariant checklist

Before submitting any code to `tyr-core`, verify:

- [ ] No `std::fs::*` usage
- [ ] No `std::env::*` usage
- [ ] No `std::net::*` or `tokio::net::*` usage
- [ ] No `unsafe` block without a `// SAFETY:` comment explaining why it cannot be exploited
- [ ] No way for guest code to call any function not in the registered `externalFunctions` map
- [ ] No way for guest code to read or write to any memory outside the VM heap

If you are implementing an external function bridge: the bridge must validate that the
function name exists in the registered set before suspending. An unregistered name must
produce `TyrError::UnknownExternalFunction`, not a panic or a silent no-op.

---

## oxc usage patterns

`tyr` uses `oxc_parser` for parsing and `oxc_ast` for AST traversal. A few patterns to follow:

```rust
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use oxc_allocator::Allocator;

// Always use SourceType::tsx() — it handles both TS and TSX
let allocator = Allocator::default();
let source_type = SourceType::tsx();
let ret: ParserReturn = Parser::new(&allocator, source, source_type).parse();

if !ret.errors.is_empty() {
    return Err(TyrError::ParseError(format_oxc_errors(&ret.errors, source)));
}
```

When walking the AST:
- Use `match` exhaustively — never `_` wildcard on statement or expression nodes.
  An unhandled node should produce `TyrError::UnsupportedSyntax`, not be silently ignored.
- Preserve span information in IR nodes for error messages. Users (and LLMs) need to know
  which line caused the error.
- Do not use `oxc_transformer` or `oxc_semantic` — they add weight we don't need.

---

## Async / await implementation notes

Async is the hardest part of `tyr`. Key invariants:

**The VM is single-threaded.** There is no Tokio runtime inside the VM. Async host functions
are driven by the VM's cooperative executor, not by Tokio's task scheduler.

**`await` on a host function** suspends the current frame and returns
`VmState::Suspended`. The caller (Rust, JS, Python) is responsible for resolving the
future and calling `resume()`. The VM resumes on the same thread.

**`await` on an internal `Promise`** (e.g., `Promise.resolve(42)`) is handled entirely
inside the VM without suspending.

**Do not** try to integrate `tokio::spawn` or `async_std` into the VM executor. The VM
must remain embeddable in any async runtime — or no runtime at all.

---

## napi-rs binding patterns

```rust
// crates/tyr-js/src/lib.rs

#[napi]
pub struct Tyr {
    inner: tyr_core::TyrRun,
}

#[napi]
impl Tyr {
    #[napi(constructor)]
    pub fn new(code: String, options: Option<TyrOptions>) -> napi::Result<Self> {
        let opts = options.unwrap_or_default();
        let inner = tyr_core::TyrRun::new(code, opts.into())
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self { inner })
    }

    #[napi]
    pub fn run(&self, options: RunOptions) -> napi::Result<TyrResult> {
        // Sync execution — blocks the JS thread
        // Only use for quick scripts. Async scripts should use runTyrAsync.
        self.inner
            .run(options.into())
            .map(Into::into)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}
```

Keep binding layer thin. All logic lives in `tyr-core`. Bindings only:
- Convert types between Rust and host language
- Map `TyrError` to host-idiomatic errors
- Expose the public API surface documented in AGENTS.md

---

## Performance targets

These are checked in CI. Do not ship a regression:

| Metric | Target |
|---|---|
| First execution latency (simple expression) | < 1ms |
| Snapshot size (typical agent code, 10 external calls) | < 10KB |
| Snapshot + resume round-trip | < 2ms |
| Memory overhead per VM instance | < 2MB |

Run benchmarks with `make bench`. The benchmark suite is in `crates/tyr-core/benches/`.

---

## What to do when you're unsure

1. **Unsupported syntax**: emit `TyrError::UnsupportedSyntax` with the span. Do not silently skip.
2. **Sandbox boundary question**: if in doubt, block it. It is always safer to deny and explain
   than to allow and regret.
3. **Memory safety question**: add a `// SAFETY:` comment or restructure to avoid `unsafe`.
4. **API design question**: follow `@pydantic/monty`'s API shape. Consistency across the two
   projects benefits users migrating between Python and TypeScript stacks.
5. **Performance vs correctness**: always choose correctness. Optimize only with a benchmark
   that proves the tradeoff is worth it.

---

## Quick reference: public API

### TypeScript / JavaScript

```typescript
import { Tyr, TyrSnapshot, runTyrAsync, TyrError } from '@tyr/core'

// Compile once, run many times
const t = new Tyr(code, {
  inputs: ['x', 'url'],              // Variable names injected at runtime
  externalFunctions: ['fetch', 'db'], // Function names the sandbox may call
  typeCheck: true,                    // Run oxc type synthesis (optional)
  memoryLimitMb: 32,
  timeLimitMs: 5000,
})

// Sync run
const result = t.run({ inputs: { x: 42, url: 'https://...' } })
console.log(result.output)  // Value
console.log(result.stdout)  // string

// Async run (drives async/await inside the sandbox)
const result2 = await runTyrAsync(t, {
  inputs: { url: 'https://...' },
  externalFunctions: {
    fetch: async (url: string) => ({ status: 200, body: '...' }),
    db: async (query: string) => [{ id: 1 }],
  },
})

// Iterative / snapshot
let progress = t.start({ inputs: { url: 'https://...' } })
if (progress instanceof TyrSnapshot) {
  console.log(progress.functionName)  // 'fetch'
  console.log(progress.args)          // ['https://...']
  const bytes = progress.dump()       // Uint8Array
  // ... store bytes, resume later
  const loaded = TyrSnapshot.load(bytes)
  progress = loaded.resume({ returnValue: { status: 200, body: '...' } })
}
if (progress instanceof TyrComplete) {
  console.log(progress.output)
}
```

### Rust

```rust
use tyr_core::{TyrRun, TyrValue, ResourceLimits, RunOptions};

let runner = TyrRun::new(
    code.to_string(),
    vec!["url".to_string()],
    vec!["fetch".to_string()],
    ResourceLimits::default(),
)?;

// Start — pauses at first external call
let state = runner.start(vec![TyrValue::String("https://...".into())])?;

match state {
    VmState::Suspended { function_name, args, snapshot } => {
        let return_val = TyrValue::Object(/* actual fetch result */);
        let final_state = snapshot.resume(return_val)?;
    }
    VmState::Complete(value) => println!("Result: {:?}", value),
}
```
