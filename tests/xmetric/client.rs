use serial_test::serial;
use x_one::xmetric;

#[test]
#[serial]
fn test_counter_inc() {
    xmetric::reset_metrics();
    // 不 panic 即为通过
    xmetric::counter_inc("test_counter", &[("method", "GET")]);
    xmetric::counter_inc("test_counter", &[("method", "GET")]);
    xmetric::counter_inc("test_counter", &[("method", "POST")]);
}

#[test]
#[serial]
fn test_counter_add() {
    xmetric::reset_metrics();
    xmetric::counter_add("test_add", 5, &[("key", "val")]);
    xmetric::counter_add("test_add", 3, &[("key", "val")]);
}

#[test]
#[serial]
fn test_gauge_operations() {
    xmetric::reset_metrics();
    xmetric::gauge_set("test_gauge", 100.0, &[]);
    xmetric::gauge_inc("test_gauge", &[]);
    xmetric::gauge_dec("test_gauge", &[]);
}

#[test]
#[serial]
fn test_histogram_observe() {
    xmetric::reset_metrics();
    xmetric::histogram_observe("test_histogram", 0.5, &[("path", "/api")]);
    xmetric::histogram_observe("test_histogram", 1.5, &[("path", "/api")]);
}

#[test]
#[serial]
fn test_registry_not_null() {
    xmetric::reset_metrics();
    let registry = xmetric::registry();
    // registry 应该是可用的
    let _guard = registry.read();
}
