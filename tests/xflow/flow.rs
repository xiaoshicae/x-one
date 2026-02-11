use std::sync::{Arc, Mutex};
use std::time::Duration;

use x_one::XOneError;
use x_one::xflow::{Dependency, ExecuteResult, Flow, Monitor, Processor, Step};

// --- 基础流程 ---

#[test]
fn test_empty_flow_succeeds() {
    let flow = Flow::<()>::new("empty");
    let mut data = ();
    let result = flow.execute(&mut data);
    assert!(result.success());
    assert!(!result.rolled());
}

#[test]
fn test_all_steps_succeed() {
    let flow = Flow::new("pipeline")
        .step(Step::new("s1").process(|data: &mut Vec<String>| {
            data.push("s1".into());
            Ok(())
        }))
        .step(Step::new("s2").process(|data: &mut Vec<String>| {
            data.push("s2".into());
            Ok(())
        }))
        .step(Step::new("s3").process(|data: &mut Vec<String>| {
            data.push("s3".into());
            Ok(())
        }));

    let mut data = Vec::new();
    let result = flow.execute(&mut data);

    assert!(result.success());
    assert_eq!(data, vec!["s1", "s2", "s3"]);
    assert!(!result.rolled());
}

// --- Strong 依赖失败 ---

#[test]
fn test_strong_failure_stops_execution() {
    let flow = Flow::new("test")
        .step(Step::new("s1").process(|data: &mut Vec<String>| {
            data.push("s1".into());
            Ok(())
        }))
        .step(
            Step::new("s2")
                .process(|_data: &mut Vec<String>| Err(XOneError::Other("s2 failed".into()))),
        )
        .step(Step::new("s3").process(|data: &mut Vec<String>| {
            data.push("s3".into());
            Ok(())
        }));

    let mut data = Vec::new();
    let result = flow.execute(&mut data);

    assert!(!result.success());
    assert_eq!(result.err().unwrap().processor_name(), "s2");
    // s3 不应该执行
    assert!(!data.contains(&"s3".to_string()));
}

// --- 回滚 ---

#[test]
fn test_strong_failure_triggers_rollback() {
    let flow = Flow::new("test")
        .step(
            Step::new("s1")
                .process(|data: &mut Vec<String>| {
                    data.push("s1:process".into());
                    Ok(())
                })
                .rollback(|data: &mut Vec<String>| {
                    data.push("s1:rollback".into());
                    Ok(())
                }),
        )
        .step(
            Step::new("s2")
                .process(|data: &mut Vec<String>| {
                    data.push("s2:process".into());
                    Ok(())
                })
                .rollback(|data: &mut Vec<String>| {
                    data.push("s2:rollback".into());
                    Ok(())
                }),
        )
        .step(
            Step::new("s3").process(|_data: &mut Vec<String>| Err(XOneError::Other("boom".into()))),
        );

    let mut data = Vec::new();
    let result = flow.execute(&mut data);

    assert!(!result.success());
    assert!(result.rolled());
    // 回滚应按逆序执行：s2 先回滚，再 s1
    assert_eq!(
        data,
        vec!["s1:process", "s2:process", "s2:rollback", "s1:rollback"]
    );
}

#[test]
fn test_rollback_order_is_reversed() {
    let order = Arc::new(Mutex::new(Vec::new()));

    let order_c = order.clone();
    let s1 = Step::new("s1")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            order_c.lock().unwrap().push("s1");
            Ok(())
        });

    let order_c = order.clone();
    let s2 = Step::new("s2")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            order_c.lock().unwrap().push("s2");
            Ok(())
        });

    let order_c = order.clone();
    let s3 = Step::new("s3")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            order_c.lock().unwrap().push("s3");
            Ok(())
        });

    let flow = Flow::new("test")
        .step(s1)
        .step(s2)
        .step(s3)
        .step(Step::new("s4").process(|_data: &mut ()| Err(XOneError::Other("fail".into()))));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(result.rolled());
    let order = order.lock().unwrap();
    assert_eq!(*order, vec!["s3", "s2", "s1"]);
}

// --- Weak 依赖 ---

#[test]
fn test_weak_failure_continues_execution() {
    let flow = Flow::new("test")
        .step(Step::new("s1").process(|data: &mut Vec<String>| {
            data.push("s1".into());
            Ok(())
        }))
        .step(
            Step::weak("optional")
                .process(|_data: &mut Vec<String>| Err(XOneError::Other("skip".into()))),
        )
        .step(Step::new("s3").process(|data: &mut Vec<String>| {
            data.push("s3".into());
            Ok(())
        }));

    let mut data = Vec::new();
    let result = flow.execute(&mut data);

    assert!(result.success());
    assert!(result.has_skipped_errors());
    assert_eq!(result.skipped_errors().len(), 1);
    // s3 应该正常执行
    assert_eq!(data, vec!["s1", "s3"]);
}

#[test]
fn test_weak_then_strong_failure() {
    let flow = Flow::new("test")
        .step(Step::new("s1").process(|data: &mut Vec<String>| {
            data.push("s1".into());
            Ok(())
        }))
        .step(
            Step::weak("w1")
                .process(|_data: &mut Vec<String>| Err(XOneError::Other("weak fail".into()))),
        )
        .step(
            Step::new("s3")
                .process(|_data: &mut Vec<String>| Err(XOneError::Other("strong fail".into()))),
        );

    let mut data = Vec::new();
    let result = flow.execute(&mut data);

    assert!(!result.success());
    assert!(result.has_skipped_errors());
    assert_eq!(result.skipped_errors().len(), 1);
    assert!(result.rolled());
    assert_eq!(result.err().unwrap().processor_name(), "s3");
}

#[test]
fn test_weak_failure_gets_rolled_back() {
    // 弱依赖失败的步骤也应该在回滚时被回滚
    let rollback_order = Arc::new(Mutex::new(Vec::new()));

    let rc = rollback_order.clone();
    let s1 = Step::new("s1")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            rc.lock().unwrap().push("s1");
            Ok(())
        });

    let rc = rollback_order.clone();
    let w1 = Step::weak("w1")
        .process(|_data: &mut ()| Err(XOneError::Other("weak fail".into())))
        .rollback(move |_data: &mut ()| {
            rc.lock().unwrap().push("w1");
            Ok(())
        });

    let flow = Flow::new("test").step(s1).step(w1).step(
        Step::new("s3").process(|_data: &mut ()| Err(XOneError::Other("strong fail".into()))),
    );

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(result.rolled());
    let order = rollback_order.lock().unwrap();
    // w1 和 s1 都应被回滚，且逆序
    assert_eq!(*order, vec!["w1", "s1"]);
}

// --- Panic 捕获 ---

#[test]
fn test_process_panic_is_caught() {
    let flow = Flow::new("test").step(Step::new("panic_step").process(|_data: &mut ()| {
        panic!("process exploded");
    }));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(!result.success());
    let err = result.err().unwrap();
    assert_eq!(err.processor_name(), "panic_step");
    assert!(err.err().to_string().contains("process exploded"));
}

#[test]
fn test_rollback_panic_is_caught_and_continues() {
    let rollback_calls = Arc::new(Mutex::new(Vec::new()));

    let rc = rollback_calls.clone();
    let s1 = Step::new("s1")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            rc.lock().unwrap().push("s1");
            Ok(())
        });

    let s2 = Step::new("s2")
        .process(|_data: &mut ()| Ok(()))
        .rollback(|_data: &mut ()| {
            panic!("rollback panic");
        });

    let flow = Flow::new("test")
        .step(s1)
        .step(s2)
        .step(Step::new("s3").process(|_data: &mut ()| Err(XOneError::Other("fail".into()))));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(result.rolled());
    assert!(result.has_rollback_errors());
    assert_eq!(result.rollback_errors().len(), 1);
    assert!(
        result.rollback_errors()[0]
            .err()
            .to_string()
            .contains("rollback panic")
    );

    // s1 的回滚仍然应该执行
    let calls = rollback_calls.lock().unwrap();
    assert!(calls.contains(&"s1"));
}

// --- 回滚失败不中断其他回滚 ---

#[test]
fn test_rollback_error_does_not_stop_other_rollbacks() {
    let rollback_calls = Arc::new(Mutex::new(Vec::new()));

    let rc = rollback_calls.clone();
    let s1 = Step::new("s1")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            rc.lock().unwrap().push("s1");
            Ok(())
        });

    let s2 = Step::new("s2")
        .process(|_data: &mut ()| Ok(()))
        .rollback(|_data: &mut ()| Err(XOneError::Other("rollback failed".into())));

    let rc = rollback_calls.clone();
    let s3 = Step::new("s3")
        .process(|_data: &mut ()| Ok(()))
        .rollback(move |_data: &mut ()| {
            rc.lock().unwrap().push("s3");
            Ok(())
        });

    let flow = Flow::new("test")
        .step(s1)
        .step(s2)
        .step(s3)
        .step(Step::new("s4").process(|_data: &mut ()| Err(XOneError::Other("fail".into()))));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(result.has_rollback_errors());
    // s1 和 s3 的回滚应该都执行了
    let calls = rollback_calls.lock().unwrap();
    assert!(calls.contains(&"s1"));
    assert!(calls.contains(&"s3"));
}

// --- 数据传递 ---

#[test]
fn test_data_flows_between_steps() {
    let flow = Flow::new("calc")
        .step(Step::new("init").process(|data: &mut i64| {
            *data = 10;
            Ok(())
        }))
        .step(Step::new("double").process(|data: &mut i64| {
            *data *= 2;
            Ok(())
        }))
        .step(Step::new("add5").process(|data: &mut i64| {
            *data += 5;
            Ok(())
        }));

    let mut data = 0i64;
    let result = flow.execute(&mut data);

    assert!(result.success());
    assert_eq!(data, 25); // (10 * 2) + 5
}

// --- Monitor 回调 ---

struct TestMonitor {
    events: Arc<Mutex<Vec<String>>>,
}

impl Monitor for TestMonitor {
    fn on_process_done(
        &self,
        _flow_name: &str,
        processor_name: &str,
        _dependency: Dependency,
        _duration: Duration,
        err: Option<&XOneError>,
    ) {
        let status = if err.is_some() { "fail" } else { "ok" };
        self.events
            .lock()
            .unwrap()
            .push(format!("process:{processor_name}:{status}"));
    }

    fn on_rollback_done(
        &self,
        _flow_name: &str,
        processor_name: &str,
        _duration: Duration,
        _err: Option<&XOneError>,
    ) {
        self.events
            .lock()
            .unwrap()
            .push(format!("rollback:{processor_name}"));
    }

    fn on_flow_done(&self, _flow_name: &str, _duration: Duration, result: &ExecuteResult) {
        let status = if result.success() { "ok" } else { "fail" };
        self.events.lock().unwrap().push(format!("flow:{status}"));
    }
}

#[test]
fn test_monitor_receives_all_callbacks_on_success() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = TestMonitor {
        events: events.clone(),
    };

    let flow = Flow::new("test")
        .monitor(monitor)
        .step(Step::new("s1").process(|_data: &mut ()| Ok(())))
        .step(Step::new("s2").process(|_data: &mut ()| Ok(())));

    let mut data = ();
    flow.execute(&mut data);

    let events = events.lock().unwrap();
    assert_eq!(*events, vec!["process:s1:ok", "process:s2:ok", "flow:ok"]);
}

#[test]
fn test_monitor_receives_callbacks_on_failure_with_rollback() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = TestMonitor {
        events: events.clone(),
    };

    let flow = Flow::new("test")
        .monitor(monitor)
        .step(
            Step::new("s1")
                .process(|_data: &mut ()| Ok(()))
                .rollback(|_data: &mut ()| Ok(())),
        )
        .step(Step::new("s2").process(|_data: &mut ()| Err(XOneError::Other("fail".into()))));

    let mut data = ();
    flow.execute(&mut data);

    let events = events.lock().unwrap();
    assert_eq!(
        *events,
        vec![
            "process:s1:ok",
            "process:s2:fail",
            "rollback:s1",
            "flow:fail"
        ]
    );
}

// --- 链式构建 ---

#[test]
fn test_chained_builder() {
    let flow = Flow::new("chain")
        .step(Step::new("a").process(|data: &mut String| {
            data.push('a');
            Ok(())
        }))
        .step(Step::weak("b").process(|data: &mut String| {
            data.push('b');
            Ok(())
        }))
        .step(Step::new("c").process(|data: &mut String| {
            data.push('c');
            Ok(())
        }));

    let mut data = String::new();
    let result = flow.execute(&mut data);

    assert!(result.success());
    assert_eq!(data, "abc");
}

// --- trait 实现者 ---

#[test]
fn test_custom_processor_trait_impl() {
    struct DoubleProcessor;

    impl Processor<i32> for DoubleProcessor {
        fn name(&self) -> &str {
            "double"
        }

        fn process(&self, data: &mut i32) -> Result<(), XOneError> {
            *data *= 2;
            Ok(())
        }

        fn rollback(&self, data: &mut i32) -> Result<(), XOneError> {
            *data /= 2;
            Ok(())
        }
    }

    let flow = Flow::new("test")
        .step(Step::new("init").process(|data: &mut i32| {
            *data = 5;
            Ok(())
        }))
        .step(DoubleProcessor)
        .step(Step::new("fail").process(|_data: &mut i32| Err(XOneError::Other("fail".into()))));

    let mut data = 0;
    let result = flow.execute(&mut data);

    assert!(!result.success());
    assert!(result.rolled());
    // init 设为 5，double 变 10，回滚 double 变 5，回滚 init 无 rollback 为 Ok
    assert_eq!(data, 5);
}

// --- 多个弱依赖失败 ---

#[test]
fn test_multiple_weak_failures() {
    let flow = Flow::new("test")
        .step(Step::weak("w1").process(|_data: &mut ()| Err(XOneError::Other("w1 fail".into()))))
        .step(Step::weak("w2").process(|_data: &mut ()| Err(XOneError::Other("w2 fail".into()))))
        .step(Step::new("s1").process(|_data: &mut ()| Ok(())));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(result.success());
    assert_eq!(result.skipped_errors().len(), 2);
    assert_eq!(result.skipped_errors()[0].processor_name(), "w1");
    assert_eq!(result.skipped_errors()[1].processor_name(), "w2");
}

// --- 首步失败 ---

#[test]
fn test_first_step_failure_no_rollback_needed() {
    let flow = Flow::new("test")
        .step(Step::new("s1").process(|_data: &mut ()| Err(XOneError::Other("first fail".into()))));

    let mut data = ();
    let result = flow.execute(&mut data);

    assert!(!result.success());
    assert!(result.rolled());
    assert!(!result.has_rollback_errors());
}

// --- enable_monitor 开关 ---

#[test]
fn test_monitor_disabled_by_default() {
    // 默认不启用监控，不会 panic 也不会触发任何回调
    let flow = Flow::new("test").step(Step::new("s1").process(|_data: &mut ()| Ok(())));

    let mut data = ();
    let result = flow.execute(&mut data);
    assert!(result.success());
}

#[test]
fn test_enable_monitor_with_default_monitor() {
    // 启用监控但不设自定义 monitor，使用 DefaultMonitor（不会 panic）
    let flow = Flow::new("test")
        .enable_monitor()
        .step(Step::new("s1").process(|_data: &mut ()| Ok(())));

    let mut data = ();
    let result = flow.execute(&mut data);
    assert!(result.success());
}

#[test]
fn test_setting_monitor_auto_enables() {
    // 设置自定义 monitor 自动启用
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = TestMonitor {
        events: events.clone(),
    };

    let flow = Flow::new("test")
        .monitor(monitor)
        .step(Step::new("s1").process(|_data: &mut ()| Ok(())));

    let mut data = ();
    flow.execute(&mut data);

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 2); // process:s1:ok + flow:ok
}
