use x_one::xutil::cmd::*;

#[test]
fn test_get_config_from_args_space_format() {
    let args = vec!["--config".to_string(), "app.yml".to_string()];
    assert_eq!(get_config_from_args_with("config", &args), Some("app.yml".to_string()));
}

#[test]
fn test_get_config_from_args_equals_format() {
    let args = vec!["--config=app.yml".to_string()];
    assert_eq!(get_config_from_args_with("config", &args), Some("app.yml".to_string()));
}

#[test]
fn test_get_config_from_args_not_found() {
    let args = vec!["--other".to_string(), "val".to_string()];
    assert_eq!(get_config_from_args_with("config", &args), None);
}

#[test]
fn test_get_config_from_args_empty_args() {
    let args: Vec<String> = vec![];
    assert_eq!(get_config_from_args_with("config", &args), None);
}

#[test]
fn test_get_config_from_args_invalid_key() {
    let args = vec!["--config".to_string()];
    assert_eq!(get_config_from_args_with("123invalid", &args), None);
}

#[test]
fn test_get_config_from_args_key_without_value() {
    let args = vec!["--config".to_string()];
    assert_eq!(get_config_from_args_with("config", &args), None);
}

#[test]
fn test_get_config_from_args_dot_key() {
    let args = vec![
        "--server.config.location".to_string(),
        "/etc/app.yml".to_string(),
    ];
    assert_eq!(
        get_config_from_args_with("server.config.location", &args),
        Some("/etc/app.yml".to_string()),
    );
}

#[test]
fn test_get_config_from_args_single_dash_ignored() {
    let args = vec!["-config".to_string(), "val".to_string()];
    assert_eq!(get_config_from_args_with("config", &args), None);
}

#[test]
fn test_get_config_from_args_triple_dash_ignored() {
    let args = vec!["---config".to_string(), "val".to_string()];
    assert_eq!(get_config_from_args_with("config", &args), None);
}
