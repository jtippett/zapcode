use baldrick_core::vm::eval_ts;

fn main() {
    divan::main();
}

#[divan::bench]
fn simple_expression() -> baldrick_core::Value {
    eval_ts("1 + 2 * 3").unwrap()
}

#[divan::bench]
fn variable_arithmetic() -> baldrick_core::Value {
    eval_ts("const x = 42; const y = 58; x + y").unwrap()
}

#[divan::bench]
fn function_call() -> baldrick_core::Value {
    eval_ts("function add(a, b) { return a + b; } add(1, 2)").unwrap()
}

#[divan::bench]
fn loop_100() -> baldrick_core::Value {
    eval_ts("let sum = 0; for (let i = 0; i < 100; i++) { sum += i; } sum").unwrap()
}

#[divan::bench]
fn fibonacci_10() -> baldrick_core::Value {
    eval_ts(
        "function fib(n) { if (n <= 1) { return n; } return fib(n - 1) + fib(n - 2); } fib(10)",
    )
    .unwrap()
}

#[divan::bench]
fn object_creation() -> baldrick_core::Value {
    eval_ts("const obj = { a: 1, b: 2, c: 3 }; obj.a + obj.b + obj.c").unwrap()
}

#[divan::bench]
fn array_creation() -> baldrick_core::Value {
    eval_ts("[1, 2, 3, 4, 5]").unwrap()
}

#[divan::bench]
fn string_concat() -> baldrick_core::Value {
    eval_ts("\"hello\" + \" \" + \"world\"").unwrap()
}

#[divan::bench]
fn template_literal() -> baldrick_core::Value {
    eval_ts("const name = \"world\"; `hello ${name}`").unwrap()
}
