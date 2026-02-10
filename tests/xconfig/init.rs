use x_one::xconfig::init::*;

#[test]
fn test_merge_profiles_config() {
    let base_yaml = r#"
Server:
  Name: test-app
  Version: v1.0.0
  Profiles:
    Active: dev
XAxum:
  Host: "0.0.0.0"
  Port: 8000
CustomKey: base_value
"#;
    let env_yaml = r#"
XAxum:
  Port: 9090
CustomKey: env_value
"#;
    let base: serde_yaml::Value = serde_yaml::from_str(base_yaml).unwrap();
    let env: serde_yaml::Value = serde_yaml::from_str(env_yaml).unwrap();

    let merged = merge_profiles_config(base, env);

    // Server.Name 应保留基础值
    assert_eq!(
        merged
            .get("Server")
            .unwrap()
            .get("Name")
            .unwrap()
            .as_str()
            .unwrap(),
        "test-app"
    );

    // XAxum.Port 应被环境覆盖
    assert_eq!(
        merged
            .get("XAxum")
            .unwrap()
            .get("Port")
            .unwrap()
            .as_u64()
            .unwrap(),
        9090
    );

    // CustomKey 应被环境覆盖
    assert_eq!(
        merged.get("CustomKey").unwrap().as_str().unwrap(),
        "env_value"
    );
}

#[test]
fn test_merge_profiles_config_base_missing_server_inserts_env_server() {
    let base_yaml = r#"
XAxum:
  Host: "0.0.0.0"
  Port: 8000
"#;
    let env_yaml = r#"
Server:
  Name: env-app
  Version: v2.0.0
  Profiles:
    Active: dev
"#;
    let base: serde_yaml::Value = serde_yaml::from_str(base_yaml).unwrap();
    let env: serde_yaml::Value = serde_yaml::from_str(env_yaml).unwrap();

    let merged = merge_profiles_config(base, env);

    // base 无 Server 时，env 的 Server 应被正确合并
    let server = merged
        .get("Server")
        .expect("Server should exist after merge");
    assert_eq!(server.get("Name").unwrap().as_str().unwrap(), "env-app");
    assert_eq!(server.get("Version").unwrap().as_str().unwrap(), "v2.0.0");
    // Profiles 应被跳过
    assert!(server.get("Profiles").is_none());
}

#[test]
fn test_merge_profiles_config_server_二级_key_覆盖() {
    let base_yaml = r#"
Server:
  Name: base-app
  Version: v1.0.0
"#;
    let env_yaml = r#"
Server:
  Name: env-app
"#;
    let base: serde_yaml::Value = serde_yaml::from_str(base_yaml).unwrap();
    let env: serde_yaml::Value = serde_yaml::from_str(env_yaml).unwrap();

    let merged = merge_profiles_config(base, env);

    let server = merged.get("Server").unwrap();
    // Name 应被 env 覆盖
    assert_eq!(server.get("Name").unwrap().as_str().unwrap(), "env-app");
    // Version 应保留 base 值
    assert_eq!(server.get("Version").unwrap().as_str().unwrap(), "v1.0.0");
}

#[test]
fn test_load_local_config() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.yml");
    std::fs::write(
        &file_path,
        "Server:
  Name: test
",
    )
    .unwrap();

    let config = load_local_config(file_path.to_str().unwrap()).unwrap();
    assert_eq!(
        config
            .get("Server")
            .unwrap()
            .get("Name")
            .unwrap()
            .as_str()
            .unwrap(),
        "test"
    );
}

#[test]
fn test_load_local_config_not_found() {
    let result = load_local_config("/nonexistent/path.yml");
    assert!(result.is_err());
}

#[test]
fn test_init_xconfig_no_config_file() {
    // 当前目录没有 application.yml
    let result = init_xconfig();
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}
