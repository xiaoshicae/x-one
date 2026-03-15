use x_one::xflow::FlowData;

#[test]
fn test_flow_data_new() {
    let data = FlowData::<String, i32>::new("hello".to_string());
    assert_eq!(data.request, "hello");
    assert_eq!(data.response, 0); // i32 默认值
}

#[test]
fn test_flow_data_with_response() {
    let data = FlowData::with_response("req", 42);
    assert_eq!(data.request, "req");
    assert_eq!(data.response, 42);
}

#[test]
fn test_flow_data_extra_set_get() {
    let mut data = FlowData::<(), ()>::new(());
    data.set("key", "value".to_string());
    assert_eq!(data.get::<String>("key"), Some(&"value".to_string()));
    assert!(data.contains_key("key"));
    assert!(!data.contains_key("nonexistent"));
}

#[test]
fn test_flow_data_extra_get_mut() {
    let mut data = FlowData::<(), ()>::new(());
    data.set("counter", 0i32);
    if let Some(v) = data.get_mut::<i32>("counter") {
        *v += 1;
    }
    assert_eq!(data.get::<i32>("counter"), Some(&1));
}

#[test]
fn test_flow_data_extra_remove() {
    let mut data = FlowData::<(), ()>::new(());
    data.set("temp", 42i32);
    assert!(data.contains_key("temp"));
    let removed = data.remove::<i32>("temp");
    assert_eq!(removed, Some(42));
    assert!(!data.contains_key("temp"));
}

#[test]
fn test_flow_data_extra_type_mismatch() {
    let mut data = FlowData::<(), ()>::new(());
    data.set("num", 42i32);
    // 尝试用错误类型获取
    assert!(data.get::<String>("num").is_none());
}

#[test]
fn test_flow_data_with_flow() {
    use x_one::xflow::{Flow, Step};

    // 使用 FlowData 作为 Flow 的数据类型
    let flow = Flow::new("order")
        .step(
            Step::new("validate").process(|data: &mut FlowData<String, Vec<String>>| {
                data.response.push(format!("validated: {}", data.request));
                data.set("validated", true);
                Ok(())
            }),
        )
        .step(
            Step::new("process").process(|data: &mut FlowData<String, Vec<String>>| {
                let validated = data.get::<bool>("validated").copied().unwrap_or(false);
                if validated {
                    data.response.push("processed".to_string());
                }
                Ok(())
            }),
        );

    let mut data = FlowData::new("order-123".to_string());
    let result = flow.execute(&mut data);
    assert!(result.success());
    assert_eq!(data.response, vec!["validated: order-123", "processed"]);
}

#[test]
fn test_flow_data_get_mut_missing_key_returns_none() {
    let mut data = FlowData::<(), ()>::new(());
    assert!(data.get_mut::<i32>("nonexistent").is_none());
}

#[test]
fn test_flow_data_get_mut_wrong_type_returns_none() {
    let mut data = FlowData::<(), ()>::new(());
    data.set("key", 42i32);
    assert!(data.get_mut::<String>("key").is_none());
}

#[test]
fn test_flow_data_remove_missing_key_returns_none() {
    let mut data = FlowData::<(), ()>::new(());
    assert!(data.remove::<i32>("nonexistent").is_none());
}

#[test]
fn test_flow_data_remove_wrong_type_returns_none() {
    let mut data = FlowData::<(), ()>::new(());
    data.set("key", 42i32);
    assert!(data.remove::<String>("key").is_none());
}
