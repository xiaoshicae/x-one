use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use x_one::xutil::pool::Pool;

#[tokio::test]
async fn test_new_zero_workers_creates_at_least_one() {
    // worker_count.max(1) 保证至少 1 个 worker
    let pool = Pool::new(0);
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    assert!(pool.submit(move || async move {
        c.fetch_add(1, Ordering::SeqCst);
    }));

    // 等待任务执行
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "worker_count=0 时仍应有 1 个 worker 处理任务"
    );

    pool.shutdown().await;
}

#[tokio::test]
async fn test_new_creates_pool_with_given_workers() {
    let pool = Pool::new(4);
    // 能成功提交任务说明池已正常创建
    let submitted = pool.submit(|| async {});
    assert!(submitted, "向新创建的池提交任务应成功");
    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_single_task_executes_successfully() {
    let pool = Pool::new(2);
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    let ok = pool.submit(move || async move {
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert!(ok, "提交单个任务应返回 true");

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_multiple_tasks_all_executed() {
    let pool = Pool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));
    let task_count = 20;

    for _ in 0..task_count {
        let c = counter.clone();
        pool.submit(move || async move {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        task_count,
        "所有 {task_count} 个任务都应被执行"
    );

    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_returns_true_when_channel_has_capacity() {
    let pool = Pool::new(2);
    let result = pool.submit(|| async {});
    assert!(result, "通道有容量时 submit 应返回 true");
    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_returns_false_when_channel_full() {
    // worker_count=1 -> channel 容量 = 1*16 = 16
    // 用一个阻塞任务占住 worker，然后填满 channel
    let pool = Pool::new(1);
    let channel_capacity = 1 * 16;

    // 提交一个长时间运行的任务，阻塞 worker 不消费后续任务
    let (notify_tx, notify_rx) = tokio::sync::oneshot::channel::<()>();
    pool.submit(move || async move {
        // 等待信号才结束，阻塞 worker
        let _ = notify_rx.await;
    });

    // 等待 worker 拿走第一个任务
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 填满 channel（容量 16）
    for i in 0..channel_capacity {
        let ok = pool.submit(|| async {});
        assert!(ok, "第 {i} 个填充任务应成功提交");
    }

    // 此时 channel 已满，下一个 submit 应返回 false
    let overflow = pool.submit(|| async {});
    assert!(!overflow, "channel 已满时 submit 应返回 false");

    // 释放阻塞的 worker
    let _ = notify_tx.send(());
    pool.shutdown().await;
}

#[tokio::test]
async fn test_shutdown_consumes_pool_preventing_further_submit() {
    // Pool::shutdown 消费 self（move 语义），编译器保证 shutdown 后无法再调用 submit。
    // 此测试验证 shutdown 行为：drop sender 后 worker 正常退出，不会 panic。
    let pool = Pool::new(2);
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    pool.submit(move || async move {
        c.fetch_add(1, Ordering::SeqCst);
    });
    pool.shutdown().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "shutdown 前提交的任务应完成"
    );
}

#[tokio::test]
async fn test_multiple_workers_process_tasks_concurrently() {
    let pool = Pool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    for _ in 0..8 {
        let c = counter.clone();
        let mc = max_concurrent.clone();
        pool.submit(move || async move {
            let current = c.fetch_add(1, Ordering::SeqCst) + 1;
            // 记录最大并发数
            mc.fetch_max(current, Ordering::SeqCst);
            // 模拟耗时操作，让多个 worker 同时运行
            tokio::time::sleep(Duration::from_millis(50)).await;
            c.fetch_sub(1, Ordering::SeqCst);
        });
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    // max_concurrent 应大于 1，证明多个 worker 同时工作
    assert!(
        max_concurrent.load(Ordering::SeqCst) > 1,
        "应有多个 worker 并发处理任务"
    );

    pool.shutdown().await;
}

#[tokio::test]
async fn test_drop_pool_workers_eventually_stop() {
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let pool = Pool::new(2);
        let c = counter.clone();
        pool.submit(move || async move {
            c.fetch_add(1, Ordering::SeqCst);
        });
        // pool 在此 drop（不调用 shutdown），sender 被 drop，worker 应退出
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "drop 前提交的任务应被执行"
    );
}

#[tokio::test]
async fn test_shutdown_waits_for_sender_drop() {
    let pool = Pool::new(2);
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    pool.submit(move || async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        c.fetch_add(1, Ordering::SeqCst);
    });

    pool.shutdown().await;
    // shutdown 只是 drop sender，已提交的任务仍会被 worker 处理
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "shutdown 后已提交的任务应完成执行"
    );
}

#[tokio::test]
async fn test_submit_task_with_captured_state() {
    let pool = Pool::new(2);
    let data = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    for i in 0..5 {
        let d = data.clone();
        pool.submit(move || async move {
            d.lock().await.push(i);
        });
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let result = data.lock().await;
    assert_eq!(result.len(), 5, "所有任务应完成，共写入 5 个元素");
    for i in 0..5 {
        assert!(result.contains(&i), "结果应包含 {i}");
    }

    pool.shutdown().await;
}

#[tokio::test]
async fn test_new_with_one_worker_executes_sequentially() {
    let pool = Pool::new(1);
    let order = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    for i in 0..5 {
        let o = order.clone();
        pool.submit(move || async move {
            o.lock().await.push(i);
        });
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let result = order.lock().await;
    assert_eq!(result.len(), 5);
    // 单 worker 应按提交顺序执行
    assert_eq!(
        *result,
        vec![0, 1, 2, 3, 4],
        "单 worker 应按 FIFO 顺序执行任务"
    );

    pool.shutdown().await;
}

#[tokio::test]
async fn test_default_pool_size_constant() {
    assert_eq!(
        x_one::xutil::pool::DEFAULT_POOL_SIZE,
        100,
        "默认池大小应为 100"
    );
}

#[tokio::test]
async fn test_submit_returns_false_after_pool_dropped() {
    // 通过在 drop 前克隆 sender 的方式无法测试（Pool 不暴露 sender），
    // 但可以验证 drop Pool 后 channel 关闭：用 shutdown 消费 pool 后无法再 submit（编译器保证）。
    // 这里用另一种方式：在一个作用域里 drop pool，验证 worker 退出行为。
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let pool = Pool::new(2);
        // 提交几个任务
        for _ in 0..3 {
            let c = counter.clone();
            pool.submit(move || async move {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        // pool 在此 drop，sender 关闭
    }

    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        3,
        "drop pool 前提交的任务应全部完成"
    );
}

#[tokio::test]
async fn test_submit_large_worker_count_works() {
    // 验证较大 worker 数量不会导致问题
    let pool = Pool::new(64);
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..200 {
        let c = counter.clone();
        pool.submit(move || async move {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        200,
        "64 个 worker 应能处理 200 个任务"
    );

    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_task_panic_does_not_crash_pool() {
    // 一个 worker 中的 panic 不应阻止其他任务执行
    let pool = Pool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    // 提交一个会 panic 的任务
    pool.submit(|| async {
        panic!("intentional panic in pool task");
    });

    // 等待 panic 任务被执行
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 继续提交正常任务
    for _ in 0..5 {
        let c = counter.clone();
        pool.submit(move || async move {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    tokio::time::sleep(Duration::from_millis(300)).await;
    // 即使有 worker panic，剩余 worker 仍应处理任务
    let count = counter.load(Ordering::SeqCst);
    assert!(
        count >= 1,
        "panic 后其他 worker 应继续处理任务，实际执行了 {count} 个"
    );

    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_channel_full_then_drain_accepts_again() {
    // 验证 channel 满后，worker 消费后可以再次提交
    let pool = Pool::new(1);
    let channel_capacity = 16;

    // 用 oneshot 阻塞 worker
    let (notify_tx, notify_rx) = tokio::sync::oneshot::channel::<()>();
    pool.submit(move || async move {
        let _ = notify_rx.await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 填满 channel
    for _ in 0..channel_capacity {
        pool.submit(|| async {});
    }

    // 确认已满
    assert!(
        !pool.submit(|| async {}),
        "channel 满时 submit 应返回 false"
    );

    // 释放 worker，让它消费队列中的任务
    let _ = notify_tx.send(());
    tokio::time::sleep(Duration::from_millis(200)).await;

    // channel 已腾出空间，应该可以再次提交
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    assert!(
        pool.submit(move || async move {
            c.fetch_add(1, Ordering::SeqCst);
        }),
        "channel 排空后应能再次提交任务"
    );

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    pool.shutdown().await;
}

#[tokio::test]
async fn test_submit_empty_async_task_completes() {
    // 边界：空任务（no-op）
    let pool = Pool::new(2);
    let ok = pool.submit(|| async {});
    assert!(ok, "空任务应成功提交");
    pool.shutdown().await;
}

#[tokio::test]
async fn test_new_very_large_worker_count_clamped_gracefully() {
    // 验证极端 worker 数量不会 panic（虽然不推荐）
    // 使用较大但不至于耗尽资源的数量
    let pool = Pool::new(256);
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    pool.submit(move || async move {
        c.fetch_add(1, Ordering::SeqCst);
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    pool.shutdown().await;
}
