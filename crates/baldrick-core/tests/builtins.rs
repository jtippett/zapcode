use baldrick_core::vm::{eval_ts, eval_ts_with_output};
use baldrick_core::Value;

// ── Console ──────────────────────────────────────────────────────────

#[test]
fn test_console_log() {
    let (_, stdout) = eval_ts_with_output("console.log(\"hello\")").unwrap();
    assert_eq!(stdout, "hello\n");
}

#[test]
fn test_console_log_multiple_args() {
    let (_, stdout) = eval_ts_with_output("console.log(1, 2, 3)").unwrap();
    assert_eq!(stdout, "1 2 3\n");
}

#[test]
fn test_console_log_multiline() {
    let (_, stdout) = eval_ts_with_output(
        "console.log(\"a\"); console.log(\"b\")",
    ).unwrap();
    assert_eq!(stdout, "a\nb\n");
}

// ── Math ─────────────────────────────────────────────────────────────

#[test]
fn test_math_pi() {
    let result = eval_ts("Math.PI").unwrap();
    match result {
        Value::Float(f) => assert!((f - std::f64::consts::PI).abs() < 1e-10),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_math_floor() {
    let result = eval_ts("Math.floor(4.7)").unwrap();
    assert_eq!(result, Value::Float(4.0));
}

#[test]
fn test_math_ceil() {
    let result = eval_ts("Math.ceil(4.1)").unwrap();
    assert_eq!(result, Value::Float(5.0));
}

#[test]
fn test_math_abs() {
    let result = eval_ts("Math.abs(-42)").unwrap();
    assert_eq!(result, Value::Float(42.0));
}

#[test]
fn test_math_max() {
    let result = eval_ts("Math.max(1, 5, 3)").unwrap();
    assert_eq!(result, Value::Float(5.0));
}

#[test]
fn test_math_min() {
    let result = eval_ts("Math.min(1, 5, 3)").unwrap();
    assert_eq!(result, Value::Float(1.0));
}

#[test]
fn test_math_sqrt() {
    let result = eval_ts("Math.sqrt(16)").unwrap();
    assert_eq!(result, Value::Float(4.0));
}

#[test]
fn test_math_round() {
    let result = eval_ts("Math.round(4.5)").unwrap();
    assert_eq!(result, Value::Float(5.0));
}

// ── String methods ───────────────────────────────────────────────────

#[test]
fn test_string_to_upper() {
    let result = eval_ts("\"hello\".toUpperCase()").unwrap();
    assert_eq!(result, Value::String("HELLO".into()));
}

#[test]
fn test_string_to_lower() {
    let result = eval_ts("\"HELLO\".toLowerCase()").unwrap();
    assert_eq!(result, Value::String("hello".into()));
}

#[test]
fn test_string_includes() {
    let result = eval_ts("\"hello world\".includes(\"world\")").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_starts_with() {
    let result = eval_ts("\"hello\".startsWith(\"hel\")").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_ends_with() {
    let result = eval_ts("\"hello\".endsWith(\"llo\")").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_index_of() {
    let result = eval_ts("\"hello\".indexOf(\"ll\")").unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_string_trim() {
    let result = eval_ts("\"  hello  \".trim()").unwrap();
    assert_eq!(result, Value::String("hello".into()));
}

#[test]
fn test_string_split() {
    let result = eval_ts("\"a,b,c\".split(\",\")").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::String("a".into()));
            assert_eq!(arr[1], Value::String("b".into()));
            assert_eq!(arr[2], Value::String("c".into()));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_string_replace() {
    let result = eval_ts("\"hello world\".replace(\"world\", \"rust\")").unwrap();
    assert_eq!(result, Value::String("hello rust".into()));
}

#[test]
fn test_string_repeat() {
    let result = eval_ts("\"ab\".repeat(3)").unwrap();
    assert_eq!(result, Value::String("ababab".into()));
}

#[test]
fn test_string_slice() {
    let result = eval_ts("\"hello\".slice(1, 4)").unwrap();
    assert_eq!(result, Value::String("ell".into()));
}

// ── Array methods ────────────────────────────────────────────────────

#[test]
fn test_array_includes() {
    let result = eval_ts("[1, 2, 3].includes(2)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_index_of() {
    let result = eval_ts("[10, 20, 30].indexOf(20)").unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_array_join() {
    let result = eval_ts("[1, 2, 3].join(\"-\")").unwrap();
    assert_eq!(result, Value::String("1-2-3".into()));
}

#[test]
fn test_array_slice() {
    let result = eval_ts("[1, 2, 3, 4, 5].slice(1, 4)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Int(2));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_concat() {
    let result = eval_ts("[1, 2].concat([3, 4])").unwrap();
    match result {
        Value::Array(arr) => assert_eq!(arr.len(), 4),
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_reverse() {
    let result = eval_ts("[1, 2, 3].reverse()").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Int(3));
            assert_eq!(arr[1], Value::Int(2));
            assert_eq!(arr[2], Value::Int(1));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

// ── JSON ─────────────────────────────────────────────────────────────

#[test]
fn test_json_stringify() {
    let result = eval_ts("JSON.stringify({a: 1, b: 2})").unwrap();
    assert_eq!(result, Value::String("{\"a\":1,\"b\":2}".into()));
}

#[test]
fn test_json_parse() {
    let result = eval_ts("JSON.parse('{\"x\":42}').x").unwrap();
    assert_eq!(result, Value::Int(42));
}

// ── Object static methods ────────────────────────────────────────────

#[test]
fn test_object_keys() {
    let result = eval_ts("Object.keys({a: 1, b: 2, c: 3})").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::String("a".into()));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_object_values() {
    let result = eval_ts("Object.values({a: 1, b: 2})").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Value::Int(1));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_is_array() {
    let result = eval_ts("Array.isArray([1, 2])").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_is_array_false() {
    let result = eval_ts("Array.isArray(42)").unwrap();
    assert_eq!(result, Value::Bool(false));
}
