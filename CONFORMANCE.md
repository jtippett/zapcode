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

~**22.3%** of executed tests pass overall, **0 VM panics**. But the denominator is
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

## Progress

- **Global builtin functions** (`parseInt`, `parseFloat`, `isNaN`, `isFinite`,
  `Number`, `String`, `Boolean`) — added. `built-ins/parseInt` 0→73%,
  `parseFloat` 0→76%, `Boolean` 21→42%.
- **Functions are now objects** (bounded) — they hold own properties, expose
  `name`/`length`/`prototype`, and `new F()` builds a real `this` and copies the
  prototype's methods. The classic constructor pattern works, including the
  nested `F.prototype.method = function(){}` form:

  ```js
  function Box(v) { this.v = v; }
  Box.prototype.get = function () { return this.v; };
  new Box(42).get(); // 42
  ```

## Biggest remaining lever

**Closure capture is O(n²)/exponential.** Every function literal captures *all*
current globals *by value*, so a file that defines several functions in sequence
blows up (stack overflow / timeout) — which is why Test262's own multi-function
harness still can't load, and why many multi-function tests fail. Switching to
lexical capture (only free variables, by reference) is the single change that
would move the most tests now. It is a real change to the closure/scope model.

Also still missing for `built-ins/Object`/`Array`/`Function`: property
descriptors (`defineProperty`, getters/setters, enumerability), `Symbol`, and the
`bind`/`call`/`apply` function methods — most of those suites test these, not
behavior.
