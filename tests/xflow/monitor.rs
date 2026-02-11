use std::sync::{Arc, Mutex};
use std::time::Duration;

use x_one::XOneError;
use x_one::xflow::{DefaultMonitor, Dependency, ExecuteResult, Flow, Monitor, Step};

#[test]
fn test_default_monitor_does_not_panic() {
    let monitor = DefaultMonitor;
    monitor.on_process_done(
        "flow",
        "step1",
        Dependency::Strong,
        Duration::from_millis(1),
        None,
    );
    monitor.on_process_done(
        "flow",
        "step2",
        Dependency::Weak,
        Duration::from_millis(2),
        Some(&XOneError::Other("timeout".into())),
    );
    monitor.on_rollback_done("flow", "step1", Duration::from_millis(1), None);
    monitor.on_rollback_done(
        "flow",
        "step1",
        Duration::from_millis(1),
        Some(&XOneError::Other("rollback err".into())),
    );

    let mut data = ();
    let result = Flow::<()>::new("ok").execute(&mut data);
    monitor.on_flow_done("flow", Duration::from_millis(5), &result);
}

/// 自定义监控器，记录回调事件
struct RecordingMonitor {
    events: Arc<Mutex<Vec<String>>>,
}

impl Monitor for RecordingMonitor {
    fn on_process_done(
        &self,
        flow_name: &str,
        processor_name: &str,
        _dependency: Dependency,
        _duration: Duration,
        err: Option<&XOneError>,
    ) {
        let status = if err.is_some() { "fail" } else { "ok" };
        self.events
            .lock()
            .unwrap()
            .push(format!("{flow_name}:{processor_name}:process:{status}"));
    }

    fn on_rollback_done(
        &self,
        flow_name: &str,
        processor_name: &str,
        _duration: Duration,
        err: Option<&XOneError>,
    ) {
        let status = if err.is_some() { "fail" } else { "ok" };
        self.events
            .lock()
            .unwrap()
            .push(format!("{flow_name}:{processor_name}:rollback:{status}"));
    }

    fn on_flow_done(&self, flow_name: &str, _duration: Duration, result: &ExecuteResult) {
        let status = if result.success() { "ok" } else { "fail" };
        self.events
            .lock()
            .unwrap()
            .push(format!("{flow_name}:flow:{status}"));
    }
}

#[test]
fn test_custom_monitor_receives_callbacks() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = RecordingMonitor {
        events: events.clone(),
    };

    let flow = Flow::new("order")
        .monitor(monitor)
        .step(Step::new("validate").process(|_data: &mut ()| Ok(())))
        .step(Step::new("save").process(|_data: &mut ()| Ok(())));

    let mut data = ();
    let result = flow.execute(&mut data);
    assert!(result.success());

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], "order:validate:process:ok");
    assert_eq!(events[1], "order:save:process:ok");
    assert_eq!(events[2], "order:flow:ok");
}
