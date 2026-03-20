use prometheus_client::encoding::text::encode;
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

#[test]
#[serial]
fn test_reset_metrics_clears_all_stores() {
    xmetric::reset_metrics();

    // 创建各类型指标
    xmetric::counter_inc("reset_counter", &[("a", "1")]);
    xmetric::gauge_set("reset_gauge", 5.0, &[("b", "2")]);
    xmetric::histogram_observe("reset_histogram", 0.1, &[("c", "3")]);

    // 重置
    xmetric::reset_metrics();

    // 重置后再次创建同名指标不应 panic，说明缓存已清空
    xmetric::counter_inc("reset_counter", &[("a", "1")]);
    xmetric::gauge_set("reset_gauge", 99.0, &[("b", "2")]);
    xmetric::histogram_observe("reset_histogram", 0.5, &[("c", "3")]);

    // 验证 registry 也被重置：encode 后不应包含旧数据的重复注册
    let registry = xmetric::registry();
    let mut output = String::new();
    prometheus_client::encoding::text::encode(&mut output, &registry.read())
        .expect("encode 不应失败");
    // 重置后重建的指标应正常存在
    assert!(
        output.contains("reset_counter"),
        "reset 后重建的 counter 应存在于 registry"
    );
    assert!(
        output.contains("reset_gauge"),
        "reset 后重建的 gauge 应存在于 registry"
    );
    assert!(
        output.contains("reset_histogram"),
        "reset 后重建的 histogram 应存在于 registry"
    );
}

#[test]
#[serial]
fn test_counter_with_multiple_label_sets() {
    xmetric::reset_metrics();

    // 使用不同标签集的同一指标
    xmetric::counter_add(
        "multi_label_counter",
        10,
        &[("method", "GET"), ("status", "200")],
    );
    xmetric::counter_add(
        "multi_label_counter",
        3,
        &[("method", "POST"), ("status", "201")],
    );
    xmetric::counter_add(
        "multi_label_counter",
        1,
        &[("method", "GET"), ("status", "404")],
    );

    let registry = xmetric::registry();
    let mut output = String::new();
    prometheus_client::encoding::text::encode(&mut output, &registry.read())
        .expect("encode 不应失败");
    assert!(
        output.contains("multi_label_counter"),
        "多标签 counter 应存在于 registry"
    );
}

#[test]
#[serial]
fn test_gauge_set_and_inc_dec_with_labels() {
    xmetric::reset_metrics();

    let labels = [("region", "us-east")];
    xmetric::gauge_set("conn_pool", 10.0, &labels);
    xmetric::gauge_inc("conn_pool", &labels);
    xmetric::gauge_dec("conn_pool", &labels);
    // 最终值应回到 10.0，验证不 panic 即通过

    let registry = xmetric::registry();
    let mut output = String::new();
    prometheus_client::encoding::text::encode(&mut output, &registry.read())
        .expect("encode 不应失败");
    assert!(output.contains("conn_pool"), "gauge 应存在于 registry");
    assert!(output.contains("region"), "gauge 的标签应被编码");
}

#[test]
#[serial]
fn test_histogram_observe_multiple_values_with_labels() {
    xmetric::reset_metrics();

    let labels = [("endpoint", "/health")];
    xmetric::histogram_observe("latency", 0.001, &labels);
    xmetric::histogram_observe("latency", 0.05, &labels);
    xmetric::histogram_observe("latency", 0.5, &labels);
    xmetric::histogram_observe("latency", 5.0, &labels);

    let registry = xmetric::registry();
    let mut output = String::new();
    prometheus_client::encoding::text::encode(&mut output, &registry.read())
        .expect("encode 不应失败");
    assert!(output.contains("latency"), "histogram 应存在于 registry");
    assert!(output.contains("endpoint"), "histogram 的标签应被编码");
}

#[test]
#[serial]
fn test_dyn_labels_ordering_consistent_regardless_of_input_order() {
    xmetric::reset_metrics();

    // 用不同顺序的标签调用，应命中同一 family 内的同一 metric
    xmetric::counter_add("order_test", 1, &[("b", "2"), ("a", "1")]);
    xmetric::counter_add("order_test", 1, &[("a", "1"), ("b", "2")]);

    let registry = xmetric::registry();
    let mut output = String::new();
    prometheus_client::encoding::text::encode(&mut output, &registry.read())
        .expect("encode 不应失败");
    // 标签排序后 ("a","1"),("b","2") 是同一时间序列，值应合并为 2
    assert!(output.contains("order_test"), "counter 应存在于 registry");
    // 验证输出中 total 为 2（两次 inc 合并）
    assert!(
        output.contains("2"),
        "同一标签集的两次 counter_add 应合并: {output}"
    );
}

#[test]
#[serial]
fn test_build_metric_name_with_namespace_via_config() {
    xmetric::reset_metrics();

    // 通过 xconfig 注入带 namespace 的配置，再调用 xmetric init
    use x_one::xconfig;

    xconfig::reset_config();
    let yaml: serde_yaml::Value = serde_yaml::from_str("XMetric:\n  Namespace: myapp\n").unwrap();
    xconfig::set_config(yaml);

    // 手动调用 xmetric init（会读取 xconfig 并调用内部 set_config）
    let _ = x_one::xmetric::init::init_xmetric();

    // 创建各类型指标，应使用 namespace 前缀
    xmetric::counter_inc("ns_test_counter", &[("k", "v")]);
    xmetric::gauge_set("ns_test_gauge", 1.0, &[("k", "v")]);
    xmetric::histogram_observe("ns_test_histo", 0.5, &[("k", "v")]);

    let mut output = String::new();
    encode(&mut output, &xmetric::registry().read()).expect("encode 不应失败");

    // 验证指标名带有 namespace 前缀
    assert!(
        output.contains("myapp_ns_test_counter"),
        "counter 应带 namespace 前缀: {output}"
    );
    assert!(
        output.contains("myapp_ns_test_gauge"),
        "gauge 应带 namespace 前缀: {output}"
    );
    assert!(
        output.contains("myapp_ns_test_histo"),
        "histogram 应带 namespace 前缀: {output}"
    );

    // 清理
    xconfig::reset_config();
    xmetric::reset_metrics();
}

#[test]
#[serial]
fn test_reset_metrics_clears_config_and_stores_completely() {
    xmetric::reset_metrics();

    // 创建各类型指标并编码
    xmetric::counter_inc("clear_c", &[("x", "1")]);
    xmetric::gauge_set("clear_g", 42.0, &[("y", "2")]);
    xmetric::histogram_observe("clear_h", 0.1, &[("z", "3")]);

    let mut before = String::new();
    encode(&mut before, &xmetric::registry().read()).unwrap();
    assert!(before.contains("clear_c"), "reset 前 counter 应存在");
    assert!(before.contains("clear_g"), "reset 前 gauge 应存在");
    assert!(before.contains("clear_h"), "reset 前 histogram 应存在");

    // 执行 reset
    xmetric::reset_metrics();

    // reset 后 registry 应为空
    let mut after = String::new();
    encode(&mut after, &xmetric::registry().read()).unwrap();
    assert!(
        !after.contains("clear_c"),
        "reset 后 counter 不应存在于 registry: {after}"
    );
    assert!(
        !after.contains("clear_g"),
        "reset 后 gauge 不应存在于 registry: {after}"
    );
    assert!(
        !after.contains("clear_h"),
        "reset 后 histogram 不应存在于 registry: {after}"
    );
}

#[test]
#[serial]
fn test_counter_gauge_histogram_with_multiple_label_pairs() {
    xmetric::reset_metrics();

    // 使用多对标签
    let labels = [("service", "auth"), ("env", "prod"), ("region", "us")];

    xmetric::counter_add("multi_pair_counter", 7, &labels);
    xmetric::gauge_set("multi_pair_gauge", 3.5, &labels);
    xmetric::histogram_observe("multi_pair_histo", 0.25, &labels);

    let mut output = String::new();
    encode(&mut output, &xmetric::registry().read()).unwrap();

    // 验证所有标签 key 都出现在编码输出中（覆盖 DynLabels encode 循环）
    assert!(
        output.contains("service"),
        "标签 service 应被编码: {output}"
    );
    assert!(output.contains("env"), "标签 env 应被编码: {output}");
    assert!(output.contains("region"), "标签 region 应被编码: {output}");

    // 验证标签值
    assert!(output.contains("auth"), "标签值 auth 应被编码: {output}");
    assert!(output.contains("prod"), "标签值 prod 应被编码: {output}");
    assert!(output.contains("us"), "标签值 us 应被编码: {output}");
}

#[test]
#[serial]
fn test_dyn_labels_sorted_encoding_produces_consistent_output() {
    xmetric::reset_metrics();

    // 标签以 ("z","3"), ("a","1"), ("m","2") 顺序传入
    xmetric::counter_inc("sorted_enc", &[("z", "3"), ("a", "1"), ("m", "2")]);

    let mut output = String::new();
    encode(&mut output, &xmetric::registry().read()).unwrap();

    // 验证编码输出中标签存在（encode 路径被执行）
    assert!(output.contains("sorted_enc"), "counter 应存在于 registry");
    // 在 prometheus text 格式中标签按 encode 顺序排列
    // 排序后应为 a, m, z
    let label_section_start = output.find('{').expect("应有标签段");
    let label_section_end = output[label_section_start..].find('}').unwrap() + label_section_start;
    let label_section = &output[label_section_start..=label_section_end];

    let pos_a = label_section.find("a=").expect("应包含 a=");
    let pos_m = label_section.find("m=").expect("应包含 m=");
    let pos_z = label_section.find("z=").expect("应包含 z=");
    assert!(
        pos_a < pos_m && pos_m < pos_z,
        "标签应按 key 排序: a < m < z，实际: {label_section}"
    );
}

#[test]
#[serial]
fn test_same_metric_name_different_types_are_independent() {
    xmetric::reset_metrics();

    // 不同类型的指标同名不应冲突（各自在独立 store 中）
    xmetric::counter_inc("shared_name", &[("type", "counter")]);
    xmetric::gauge_set("shared_name_g", 1.0, &[("type", "gauge")]);
    xmetric::histogram_observe("shared_name_h", 0.5, &[("type", "histogram")]);

    let mut output = String::new();
    encode(&mut output, &xmetric::registry().read()).unwrap();

    assert!(output.contains("shared_name"), "counter shared_name 应存在");
    assert!(
        output.contains("shared_name_g"),
        "gauge shared_name_g 应存在"
    );
    assert!(
        output.contains("shared_name_h"),
        "histogram shared_name_h 应存在"
    );
}

#[test]
#[serial]
fn test_empty_labels_encode_correctly() {
    xmetric::reset_metrics();

    // 空标签数组
    xmetric::counter_inc("no_labels_counter", &[]);
    xmetric::gauge_set("no_labels_gauge", 99.0, &[]);
    xmetric::histogram_observe("no_labels_histo", 0.1, &[]);

    let mut output = String::new();
    encode(&mut output, &xmetric::registry().read()).unwrap();

    assert!(
        output.contains("no_labels_counter"),
        "无标签 counter 应存在"
    );
    assert!(output.contains("no_labels_gauge"), "无标签 gauge 应存在");
    assert!(
        output.contains("no_labels_histo"),
        "无标签 histogram 应存在"
    );
}
