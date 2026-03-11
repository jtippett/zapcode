# Rust Examples

## Prerequisites

- [Rust toolchain](https://rustup.rs/)

## Run

```bash
cargo run --example basic
```

> **Note:** The examples crate is excluded from the workspace. It has its own `Cargo.toml` that depends on `zapcode-core` via path.

## What's here

| File | Description |
|---|---|
| `basic.rs` | Simple expressions, inputs, external functions (snapshot/resume), snapshot serialization |
