use x_one::xutil::cmd::*;

    #[test]
    fn test_get_config_from_args_space_format() {
        let args = vec!["--config".to_string(), "app.yml".to_string()];
        let result = get_config_from_args_with("config", &args);
        assert_eq!(result.unwrap(), "app.yml");
    }

    #[test]
    fn test_get_config_from_args_equals_format() {
        let args = vec!["--config=app.yml".to_string()];
        let result = get_config_from_args_with("config", &args);
        assert_eq!(result.unwrap(), "app.yml");
    }

    #[test]
    fn test_get_config_from_args_not_found() {
        let args = vec!["--other".to_string(), "val".to_string()];
        let result = get_config_from_args_with("config", &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "arg not found");
    }

    #[test]
    fn test_get_config_from_args_empty_args() {
        let args: Vec<String> = vec![];
        let result = get_config_from_args_with("config", &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_config_from_args_invalid_key() {
        let args = vec!["--config".to_string()];
        let result = get_config_from_args_with("123invalid", &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("key must match regexp"));
    }

    #[test]
    fn test_get_config_from_args_key_without_value() {
        let args = vec!["--config".to_string()];
        let result = get_config_from_args_with("config", &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "arg not found, arg not set");
    }

    #[test]
    fn test_get_config_from_args_dot_key() {
        let args = vec![
            "--server.config.location".to_string(),
            "/etc/app.yml".to_string(),
        ];
        let result = get_config_from_args_with("server.config.location", &args);
        assert_eq!(result.unwrap(), "/etc/app.yml");
    }
