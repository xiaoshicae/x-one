use x_one::xserver::*;
use x_one::{Server, XOneError};

struct MockServer {
    should_fail: bool,
}

impl Server for MockServer {
    async fn run(&self) -> Result<(), XOneError> {
        if self.should_fail {
            Err(XOneError::Server("mock failure".to_string()))
        } else {
            Ok(())
        }
    }

    async fn stop(&self) -> Result<(), XOneError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_server_run_success() {
    let server = MockServer { should_fail: false };
    let result: Result<(), XOneError> = server.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_run_failure() {
    let server = MockServer { should_fail: true };
    let result: Result<(), XOneError> = server.run().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_server_stop() {
    let server = MockServer { should_fail: false };
    let result: Result<(), XOneError> = server.stop().await;
    assert!(result.is_ok());
}

#[test]
fn test_invoke_before_stop_hooks_safe_empty() {
    // 没有注册任何 hook 时应成功
    let result: Result<(), XOneError> = invoke_before_stop_hooks_safe();
    assert!(result.is_ok());
}
