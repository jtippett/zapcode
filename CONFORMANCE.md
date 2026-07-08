# ECMAScript conformance (Test262)

zapcode implements a **subset** of JavaScript (TypeScript with types stripped).
To measure that subset against a standard rather than ad-hoc probes, this fork
includes a runner for [TC39 Test262](https://github.com/tc39/test262), the
official ECMAScript conformance suite.

```bash
just test262-fetch                 # clone the suite into vendor/ (gitignored)
just test262                       # full report
just test262 built-ins/Array       # filter by path substring
just test262 language --limit 2000 # cap the number of tests
```

## How it works

Test262's real harness (`sta.js`/`assert.js`) is built on the constructor-function
+ `.prototype` pattern, which zapcode does **not** support (plain functions aren't
objects — see gaps below), so it cannot load. The runner instead executes the real
test *bodies* against a semantically-equivalent harness shim (built with `class`,
which works) plus a light `assert.X(` → `assertX(` rewrite. A positive test passes
if it runs without throwing; a negative test passes if it throws.

Deliberately skipped (not counted): `module`/`async` tests, `eval`/`Function`
dynamic-code tests (a sandbox exclusion, not a coverage gap), `intl402`,
`staging`, and tests using un-shimmed harness helpers (`verifyProperty`, `$262`).

The numbers are a **gauge, not a certified score** — the shim and rewrite are
approximate.

## Headline result (commit-pinned suite)

~**22%** of executed tests pass overall, **0 VM panics**. But the denominator is
dominated by APIs that are explicit non-goals for a minimal agent sandbox, so the
raw number understates coverage of the language we care about. Reading by area:

- **Strong (80–100%):** identifiers, keywords, reserved words, line-terminators,
  literals, ASI, block-scope, comments.
- **Moderate (30–55%):** expressions, statements, destructuring, `Math`,
  `WeakMap`/`WeakSet`, `types`, `Reflect`, global code.
- **Weak / non-goal (<20%):** `Temporal`, `TypedArray`/`DataView`/`ArrayBuffer`,
  `Atomics`, `Proxy`, `Symbol`, `BigInt`, `Promise`/generators/async, `RegExp`
  (rejected by design), `Date`, URI codecs. Also `Object`/`Array` score low
  because most of their tests exercise property descriptors, getters/setters, and
  `Symbol.species` — which need the object-model features below.

## Biggest coverage lever

**Plain functions are not objects.** They can't hold properties (`f.x = 1`
throws), have no `.prototype`, and `new` only works on `class`, not `function`.
The classic constructor-function pattern doesn't work, which is why Test262's own
harness can't load and why much of `built-ins/Object` and `built-ins/Array` fails.
Making functions first-class objects (property bag + prototype + `new` on
functions) is the single change that would move the most tests — and it is a
non-trivial change to the value model (serialization, cloning, `new`/property
dispatch). Tracked as a decision, not yet done.

Quick wins by comparison: `parseInt`/`parseFloat` are entirely missing (0%) and
are simple global builtins.
