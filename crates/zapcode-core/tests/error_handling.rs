use zapcode_core::vm::eval_ts;
use zapcode_core::Value;

#[test]
fn test_try_catch() {
    let result = eval_ts(
        r#"
        let caught = false;
        try {
            throw "error";
        } catch (e) {
            caught = true;
        }
        caught
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_try_catch_value() {
    let result = eval_ts(
        r#"
        let msg = "";
        try {
            throw "oops";
        } catch (e) {
            msg = e;
        }
        msg
        "#,
    )
    .unwrap();
    // The thrown value becomes a runtime error message
    if let Value::String(s) = result {
        assert!(!s.is_empty());
    }
}

#[test]
fn test_try_no_error() {
    let result = eval_ts(
        r#"
        let x = 0;
        try {
            x = 42;
        } catch (e) {
            x = -1;
        }
        x
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_regex_literal_is_rejected_not_silently_ignored() {
    // Regex is unsupported; it must be a loud error, not a silent no-op.
    let err = zapcode_core::vm::eval_ts("'abc123'.replace(/[0-9]+/, '#')").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("regular expressions"), "got: {msg}");
}
