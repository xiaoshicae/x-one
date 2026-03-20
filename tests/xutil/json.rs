use std::collections::HashMap;
use x_one::xutil::json::*;

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

#[test]
fn test_to_json_string_indent_struct() {
    #[derive(serde::Serialize)]
    struct Info {
        name: String,
        count: u32,
    }
    let info = Info {
        name: "test".to_string(),
        count: 10,
    };
    let result = to_json_string_indent(&info);
    assert!(result.contains(r#""name": "test""#));
    assert!(result.contains(r#""count": 10"#));
    assert!(result.contains('\n'));
}

/// 自定义类型，Serialize 实现会返回错误
struct FailSerialize;

impl serde::Serialize for FailSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("intentional failure"))
    }
}

#[test]
fn test_to_json_string_serialize_error_returns_empty() {
    let result = to_json_string(&FailSerialize);
    assert_eq!(result, "", "序列化失败应返回空字符串");
}

#[test]
fn test_to_json_string_indent_serialize_error_returns_empty() {
    let result = to_json_string_indent(&FailSerialize);
    assert_eq!(result, "", "序列化失败应返回空字符串");
}
