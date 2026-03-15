use x_one::xpipeline::RunResult;

#[test]
fn test_run_result_default_success() {
    let result = RunResult::default();
    assert!(result.success());
    assert!(!result.has_errors());
    assert_eq!(result.to_string(), "");
}

#[test]
fn test_run_result_with_errors() {
    let result = RunResult {
        errors: vec![x_one::xpipeline::StepError {
            processor_name: "test".to_string(),
            err: x_one::XOneError::Other("fail".to_string()),
        }],
    };
    assert!(!result.success());
    assert!(result.has_errors());
    assert!(result.to_string().contains("pipeline failed"));
}

#[test]
fn test_step_error_display() {
    let err = x_one::xpipeline::StepError {
        processor_name: "my-proc".to_string(),
        err: x_one::XOneError::Other("something broke".to_string()),
    };
    let display = err.to_string();
    assert!(display.contains("my-proc"));
    assert!(display.contains("something broke"));
}

#[test]
fn test_step_error_is_std_error() {
    let err = x_one::xpipeline::StepError {
        processor_name: "proc".to_string(),
        err: x_one::XOneError::Other("err".to_string()),
    };
    // StepError implements std::error::Error
    let _: &dyn std::error::Error = &err;
}
