use x_one::xutil::json::*;
use std::collections::HashMap;

#[test]
fn test_to_json_string_map() {
    let mut m = HashMap::new();
    m.insert("key", "value");
    let result = to_json_string(&m);
    assert_eq!(result, r#"{"key":"value"}"#);
}

#[test]
fn test_to_json_string_struct() {
    #[derive(serde::Serialize)]
    struct Person {
        name: String,
        age: i32,
    }
    let p = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    let result = to_json_string(&p);
    assert!(result.contains(r#""name":"Alice""#));
    assert!(result.contains(r#""age":30"#));
}

#[test]
fn test_to_json_string_indent_map() {
    let mut m = HashMap::new();
    m.insert("key", "value");
    let result = to_json_string_indent(&m);
    assert!(result.contains("key"));
    assert!(result.contains("value"));
    assert!(result.contains('\n'));
}