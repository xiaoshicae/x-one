use x_one::XOneError;
use x_one::xflow::{Dependency, Processor, Step};

#[test]
fn test_new_creates_strong_dependency() {
    let step = Step::<()>::new("init");
    assert_eq!(step.name(), "init");
    assert_eq!(step.dependency(), Dependency::Strong);
}

#[test]
fn test_weak_creates_weak_dependency() {
    let step = Step::<()>::weak("optional");
    assert_eq!(step.name(), "optional");
    assert_eq!(step.dependency(), Dependency::Weak);
}

#[test]
fn test_process_closure_executes() {
    let step = Step::new("add").process(|data: &mut i32| {
        *data += 10;
        Ok(())
    });

    let mut data = 5;
    Processor::process(&step, &mut data).unwrap();
    assert_eq!(data, 15);
}

#[test]
fn test_rollback_closure_executes() {
    let step = Step::new("save")
        .process(|data: &mut String| {
            *data = "saved".into();
            Ok(())
        })
        .rollback(|data: &mut String| {
            *data = "rolled back".into();
            Ok(())
        });

    let mut data = String::new();
    Processor::process(&step, &mut data).unwrap();
    assert_eq!(data, "saved");

    Processor::rollback(&step, &mut data).unwrap();
    assert_eq!(data, "rolled back");
}

#[test]
fn test_default_process_returns_ok() {
    let step = Step::<()>::new("noop");
    let mut data = ();
    assert!(Processor::process(&step, &mut data).is_ok());
}

#[test]
fn test_default_rollback_returns_ok() {
    let step = Step::<()>::new("noop");
    let mut data = ();
    assert!(Processor::rollback(&step, &mut data).is_ok());
}

#[test]
fn test_process_closure_can_return_error() {
    let step =
        Step::new("fail").process(|_data: &mut ()| Err(XOneError::Other("bad input".into())));

    let mut data = ();
    let err = Processor::process(&step, &mut data).unwrap_err();
    assert!(err.to_string().contains("bad input"));
}
