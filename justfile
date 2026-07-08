# zapcode (fork) commands. Run `just --list`.

# Run the core test suite.
test:
    cargo test -p zapcode-core

# Format + lint.
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

# Fetch the TC39 Test262 conformance suite into vendor/ (gitignored, ~50k tests).
test262-fetch:
    git clone --depth 1 https://github.com/tc39/test262.git vendor/test262

# Run the Test262 conformance report. Optional path filter, e.g.
#   just test262 built-ins/Array
#   just test262 language/statements --limit 500
test262 *ARGS:
    cargo run --release --example test262 -- {{ARGS}}
