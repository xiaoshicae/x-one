use x_one::error::*;

#[test]
fn test_xone_error_display() {
    let e = XOneError::Hook("test hook error".to_string());
    assert_eq!(e.to_string(), "hook error: test hook error");
}

#[test]
fn test_xone_error_from_string() {
    let e: XOneError = "some error".to_string().into();
    assert_eq!(e.to_string(), "some error");
}

#[test]
fn test_xone_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let e: XOneError = io_err.into();
    assert!(e.to_string().contains("file not found"));
}

#[test]
fn test_xone_error_multi() {
    let e = XOneError::Multi(vec![
        XOneError::Server("server failed".to_string()),
        XOneError::Hook("hook failed".to_string()),
    ]);
    let msg = e.to_string();
    assert!(msg.contains("server failed"));
    assert!(msg.contains("hook failed"));
    assert!(msg.contains("multiple errors"));
}

#[test]
fn test_xone_error_variants() {
    assert_eq!(
        XOneError::Config("bad config".to_string()).to_string(),
        "config error: bad config"
    );
    assert_eq!(
        XOneError::Log("log init failed".to_string()).to_string(),
        "log error: log init failed"
    );
    assert_eq!(
        XOneError::Server("bind failed".to_string()).to_string(),
        "server error: bind failed"
    );
}
