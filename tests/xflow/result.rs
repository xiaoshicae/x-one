use x_one::XOneError;
use x_one::xflow::{Dependency, Flow, Step, StepError};

// --- StepError 测试 ---

#[test]
fn test_step_error_display_contains_name_and_dependency() {
    let err = StepError::new(
        "validate",
        Dependency::Strong,
        XOneError::Other("bad".into()),
    );
    let msg = err.to_string();
    assert!(msg.contains("validate"), "应包含处理器名称");
    assert!(msg.contains("strong"), "应包含依赖类型");
    assert!(msg.contains("bad"), "应包含原始错误消息");
}

#[test]
fn test_step_error_source_returns_inner_error() {
    use std::error::Error;
    let err = StepError::new("step1", Dependency::Weak, XOneError::Other("inner".into()));
    let source = err.source().expect("应有 source");
    assert!(source.to_string().contains("inner"));
}

#[test]
fn test_step_error_accessors() {
    let err = StepError::new("save", Dependency::Weak, XOneError::Other("db down".into()));
    assert_eq!(err.processor_name(), "save");
    assert_eq!(err.dependency(), Dependency::Weak);
    assert!(err.err().to_string().contains("db down"));
}

// --- ExecuteResult 测试 ---

#[test]
fn test_execute_result_success() {
    let mut data = ();
    let result = Flow::<()>::new("empty").execute(&mut data);
    assert!(result.success());
    assert!(result.err().is_none());
}

#[test]
fn test_execute_result_with_error() {
    let flow = Flow::new("fail")
        .step(Step::new("boom").process(|_data: &mut ()| Err(XOneError::Other("exploded".into()))));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(!result.success());
    assert!(result.err().is_some());
    assert_eq!(result.err().unwrap().processor_name(), "boom");
}

#[test]
fn test_execute_result_skipped_errors() {
    let flow = Flow::new("test").step(
        Step::weak("optional").process(|_data: &mut ()| Err(XOneError::Other("skipped".into()))),
    );

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(result.success(), "弱依赖失败不影响整体成功");
    assert!(result.has_skipped_errors());
    assert_eq!(result.skipped_errors().len(), 1);
    assert_eq!(result.skipped_errors()[0].processor_name(), "optional");
}

#[test]
fn test_execute_result_display_success() {
    let mut data = ();
    let result = Flow::<()>::new("ok").execute(&mut data);
    assert!(result.to_string().contains("succeeded"));
}

#[test]
fn test_execute_result_display_failure_with_rollback() {
    let flow = Flow::new("test")
        .step(Step::new("s1").process(|_data: &mut ()| Ok(())))
        .step(Step::new("s2").process(|_data: &mut ()| Err(XOneError::Other("fail".into()))));

    let mut data = ();
    let result = flow.execute(&mut data);
    let display = result.to_string();

    assert!(display.contains("failed"), "应包含失败信息");
    assert!(display.contains("rolled back"), "应包含回滚信息");
}

#[test]
fn test_execute_result_debug() {
    let mut data = ();
    let result = Flow::<()>::new("ok").execute(&mut data);
    let debug = format!("{:?}", result);
    assert!(debug.contains("ExecuteResult"));
    assert!(debug.contains("success"));
}
