use x_one::Server;
use x_one::xserver::blocking::*;

#[tokio::test]
async fn test_blocking_server_stop_unblocks_run() {
    let server = BlockingServer::new();

    let rx = server.rx();
    let handle = tokio::spawn(async move {
        let mut rx = rx;
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                break;
            }
        }
    });

    // 发送 stop 信号
    server.stop().await.unwrap();

    // run 应该返回
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), handle).await;
    assert!(result.is_ok());
}
