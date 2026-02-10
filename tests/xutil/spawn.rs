use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use x_one::xutil::{extract_panic_message, spawn_safe};

#[tokio::test]
async fn test_spawn_safe_normal_task_completes_successfully() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    let handle = spawn_safe(async move {
        flag_clone.store(true, Ordering::SeqCst);
    });

    handle.await.unwrap();
    assert!(flag.load(Ordering::SeqCst), "任务应该已经执行完成");
}

#[tokio::test]
async fn test_spawn_safe_panic_captured_not_propagated() {
    let handle = spawn_safe(async {
        panic!("test panic message");
    });

    // 外层 JoinHandle 应该正常返回 Ok，不会传播 panic
    let result = handle.await;
    assert!(result.is_ok(), "外层 JoinHandle 不应传播 panic");
}

#[tokio::test]
async fn test_spawn_safe_fire_and_forget_task_executes() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    // 不 await JoinHandle，验证任务仍然会执行
    let _handle = spawn_safe(async move {
        flag_clone.store(true, Ordering::SeqCst);
    });

    // 等待足够时间让任务完成
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        flag.load(Ordering::SeqCst),
        "fire-and-forget 任务应该已执行"
    );
}

#[tokio::test]
async fn test_spawn_safe_returns_joinhandle_can_await() {
    let handle = spawn_safe(async {
        // 空任务
    });

    let result = handle.await;
    assert!(result.is_ok(), "await JoinHandle 应返回 Ok(())");
}

#[test]
fn test_extract_panic_message_from_str() {
    let payload: Box<dyn Any + Send> = Box::new("hello panic");
    assert_eq!(extract_panic_message(payload), "hello panic");
}

#[test]
fn test_extract_panic_message_from_string() {
    let payload: Box<dyn Any + Send> = Box::new(String::from("string panic"));
    assert_eq!(extract_panic_message(payload), "string panic");
}

#[test]
fn test_extract_panic_message_from_unknown_type() {
    let payload: Box<dyn Any + Send> = Box::new(42_i32);
    assert_eq!(extract_panic_message(payload), "unknown panic");
}
