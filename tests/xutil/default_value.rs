use x_one::xutil::{default_if_empty, take_or_default};

// ---- default_if_empty（借用版本）----

#[test]
fn test_default_if_empty_str_empty() {
    assert_eq!(default_if_empty("", "fallback"), "fallback");
}

#[test]
fn test_default_if_empty_str_non_empty() {
    assert_eq!(default_if_empty("hello", "fallback"), "hello");
}

#[test]
fn test_default_if_empty_string_empty() {
    let value = String::new();
    let fallback = String::from("fallback");
    assert_eq!(default_if_empty(&value, &fallback), "fallback");
}

#[test]
fn test_default_if_empty_string_non_empty() {
    let value = String::from("hello");
    let fallback = String::from("fallback");
    assert_eq!(default_if_empty(&value, &fallback), "hello");
}

#[test]
fn test_default_if_empty_vec_empty() {
    let value: Vec<i32> = vec![];
    let fallback = vec![1, 2, 3];
    assert_eq!(default_if_empty(&value, &fallback), &vec![1, 2, 3]);
}

#[test]
fn test_default_if_empty_vec_non_empty() {
    let value = vec![42];
    let fallback = vec![1, 2, 3];
    assert_eq!(default_if_empty(&value, &fallback), &vec![42]);
}

#[test]
fn test_default_if_empty_option_none() {
    let value: Option<i32> = None;
    let fallback = Some(99);
    assert_eq!(default_if_empty(&value, &fallback), &Some(99));
}

#[test]
fn test_default_if_empty_option_some() {
    let value = Some(42);
    let fallback = Some(99);
    assert_eq!(default_if_empty(&value, &fallback), &Some(42));
}

// ---- take_or_default（所有权版本）----

#[test]
fn test_take_or_default_string_empty() {
    assert_eq!(take_or_default(String::new(), "fallback"), "fallback");
}

#[test]
fn test_take_or_default_string_non_empty() {
    assert_eq!(
        take_or_default(String::from("hello"), "fallback"),
        "hello"
    );
}

#[test]
fn test_take_or_default_vec_empty() {
    let result: Vec<i32> = take_or_default(vec![], vec![1, 2]);
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn test_take_or_default_vec_non_empty() {
    let result: Vec<i32> = take_or_default(vec![42], vec![1, 2]);
    assert_eq!(result, vec![42]);
}

#[test]
fn test_take_or_default_option_none() {
    assert_eq!(take_or_default(None, Some(99)), Some(99));
}

#[test]
fn test_take_or_default_option_some() {
    assert_eq!(take_or_default(Some(42), Some(99)), Some(42));
}
