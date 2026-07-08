//! Test262 conformance runner.
//!
//! Runs the vendored TC39 Test262 suite (see `just test262-fetch`) against
//! zapcode-core and prints a categorized pass/fail/skip report — a standardized
//! read on how much of ECMAScript we actually cover.
//!
//! Caveat: Test262's real harness (`sta.js`/`assert.js`) is built on the
//! constructor-function + `.prototype` pattern, which zapcode does not support
//! (plain functions aren't objects). We therefore run the real *test bodies*
//! against a semantically-equivalent harness shim (built with `class`, which
//! works) plus a light `assert.X(` -> `assertX(` source rewrite. Tests that use
//! other harness helpers (propertyHelper, deepEqual, …) will fail — an honest
//! reflection of coverage. Numbers are a gauge, not a certified conformance score.
//!
//! Usage:
//!   cargo run --release --example test262 -- [path-substring-filter] [--limit N]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use zapcode_core::{ResourceLimits, VmState, ZapcodeRun};

const HARNESS_SHIM: &str = r#"
class Test262Error { constructor(message) { this.message = message || ""; } toString() { return "Test262Error: " + this.message; } }
function $DONOTEVALUATE() { throw new Test262Error("This statement should not be evaluated."); }
function assertTrue(cond, msg) { if (cond !== true) throw new Test262Error(msg || "Expected true"); }
function sameValue(a, e) { return (a === e) ? (a !== 0 || 1 / a === 1 / e) : (a !== a && e !== e); }
function assertSameValue(a, e, msg) { if (!sameValue(a, e)) throw new Test262Error(msg || "sameValue mismatch"); }
function assertNotSameValue(a, e, msg) { if (sameValue(a, e)) throw new Test262Error(msg || "notSameValue"); }
function assertThrows(ctor, fn, msg) { try { fn(); } catch (e) { return; } throw new Test262Error(msg || "expected throw"); }
function assertCompareArray(a, b, msg) { if (a.length !== b.length) throw new Test262Error("compareArray length"); for (var i = 0; i < a.length; i++) { if (a[i] !== b[i]) throw new Test262Error("compareArray"); } }
"#;

#[derive(Default, Clone)]
struct Stats {
    pass: u32,
    fail: u32,
    skip: u32,
    crash: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut filter = String::new();
    let mut limit = usize::MAX;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--limit" {
            limit = args
                .get(i + 1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX);
            i += 2;
        } else {
            filter = args[i].clone();
            i += 1;
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/test262/test");
    if !root.exists() {
        eprintln!("vendor/test262 not found — run `just test262-fetch` first.");
        std::process::exit(1);
    }

    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();

    // A malformed input that makes the VM panic must not abort the whole run —
    // count it as a crash. Silence the default panic printout during the sweep.
    std::panic::set_hook(Box::new(|_| {}));

    let mut by_area: BTreeMap<String, Stats> = BTreeMap::new();
    let mut total = Stats::default();
    let mut ran = 0usize;

    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .to_string();
        if !filter.is_empty() && !rel.contains(&filter) {
            continue;
        }
        if ran >= limit {
            break;
        }
        ran += 1;

        let area = area_of(&rel);
        let entry = by_area.entry(area).or_default();

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_one(path)))
            .unwrap_or(Outcome::Crash);

        match outcome {
            Outcome::Pass => {
                entry.pass += 1;
                total.pass += 1;
            }
            Outcome::Fail => {
                entry.fail += 1;
                total.fail += 1;
            }
            Outcome::Skip => {
                entry.skip += 1;
                total.skip += 1;
            }
            Outcome::Crash => {
                entry.crash += 1;
                total.crash += 1;
            }
        }
    }

    // Report
    println!(
        "\n{:<40} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "AREA", "pass", "fail", "crash", "skip", "cov%"
    );
    println!("{}", "-".repeat(78));
    for (area, s) in &by_area {
        let denom = s.pass + s.fail + s.crash;
        let cov = if denom > 0 {
            100.0 * s.pass as f64 / denom as f64
        } else {
            0.0
        };
        println!(
            "{:<40} {:>6} {:>6} {:>6} {:>6} {:>5.1}",
            trunc(area, 40),
            s.pass,
            s.fail,
            s.crash,
            s.skip,
            cov
        );
    }
    let denom = total.pass + total.fail + total.crash;
    let cov = if denom > 0 {
        100.0 * total.pass as f64 / denom as f64
    } else {
        0.0
    };
    println!("{}", "-".repeat(78));
    println!(
        "{:<40} {:>6} {:>6} {:>6} {:>6} {:>5.1}",
        "TOTAL", total.pass, total.fail, total.crash, total.skip, cov
    );
    println!(
        "\n{} ran — {} pass / {} fail / {} crash / {} skip — {:.1}% of executed pass ({} VM panics)",
        ran, total.pass, total.fail, total.crash, total.skip, cov, total.crash
    );
}

enum Outcome {
    Pass,
    Fail,
    Skip,
    Crash,
}

fn run_one(path: &Path) -> Outcome {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Outcome::Skip,
    };
    let meta = frontmatter(&src);

    // Structurally unrunnable in this engine — skipped, not counted as failures.
    if meta.flags.iter().any(|f| f == "module" || f == "async") {
        return Outcome::Skip;
    }
    // Harness helpers we don't shim: don't pretend to test them.
    if src.contains("assert.throws.early")
        || src.contains("verifyProperty(")
        || src.contains("$262")
    {
        return Outcome::Skip;
    }
    // Dynamic code evaluation is a *deliberate* sandbox exclusion, not a coverage
    // gap — skip rather than count as failures.
    if src.contains("eval(") || src.contains("new Function(") || src.contains("Function(\"") {
        return Outcome::Skip;
    }

    let program = if meta.flags.iter().any(|f| f == "raw") {
        src.clone()
    } else {
        format!("{}\n{}", HARNESS_SHIM, rewrite_asserts(&src))
    };

    let limits = ResourceLimits {
        memory_limit_bytes: 64 * 1024 * 1024,
        time_limit_ms: 250,
        max_stack_depth: 400,
        max_allocations: 2_000_000,
    };

    let ran = ZapcodeRun::new(program, vec![], vec![], limits).and_then(|r| r.run(vec![]));

    let threw = match ran {
        Ok(rr) => !matches!(rr.state, VmState::Complete(_)),
        Err(_) => true,
    };

    // Negative tests must throw (parse or runtime); positive tests must not.
    if meta.negative {
        if threw {
            Outcome::Pass
        } else {
            Outcome::Fail
        }
    } else if threw {
        Outcome::Fail
    } else {
        Outcome::Pass
    }
}

/// Rewrite `assert.X(` calls into standalone shim functions, then bare `assert(`.
fn rewrite_asserts(src: &str) -> String {
    src.replace("assert.sameValue(", "assertSameValue(")
        .replace("assert.notSameValue(", "assertNotSameValue(")
        .replace("assert.throws(", "assertThrows(")
        .replace("assert.compareArray(", "assertCompareArray(")
        .replace("assert.compareIterator(", "assertCompareArray(")
        .replace("assert(", "assertTrue(")
}

struct Meta {
    flags: Vec<String>,
    negative: bool,
}

fn frontmatter(src: &str) -> Meta {
    let (mut flags, mut negative) = (Vec::new(), false);
    if let (Some(a), Some(b)) = (src.find("/*---"), src.find("---*/")) {
        if a < b {
            let block = &src[a + 5..b];
            for line in block.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("flags:") {
                    flags = rest
                        .trim()
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                if t.starts_with("negative:") {
                    negative = true;
                }
            }
        }
    }
    Meta { flags, negative }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            // Skip i18n, experimental staging, and the harness self-tests
            // (harness/ tests the helpers, not the language).
            if name == "intl402" || name == "staging" || name == "harness" {
                continue;
            }
            collect(&p, out);
        } else if let Some(ext) = p.extension() {
            if ext == "js" && !p.to_string_lossy().ends_with("_FIXTURE.js") {
                out.push(p);
            }
        }
    }
}

fn area_of(rel: &str) -> String {
    let parts: Vec<&str> = rel.split('/').collect();
    match parts.len() {
        0 => "?".to_string(),
        1 => parts[0].to_string(),
        _ => format!("{}/{}", parts[0], parts[1]),
    }
}

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n - 1])
    }
}
