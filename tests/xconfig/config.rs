use x_one::xconfig::config::*;

#[test]
fn test_server_config_default_version() {
    let c = ServerConfig::default();
    assert_eq!(c.version, "v0.0.1");
}
