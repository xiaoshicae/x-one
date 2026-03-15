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
