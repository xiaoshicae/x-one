use x_one::xconfig::profiles::*;
    use serial_test::serial;

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn test_to_profiles_active_config_location() {
        let result = to_profiles_active_config_location("./conf/application.yml", "dev").unwrap();
        assert_eq!(result, "./conf/application-dev.yml");
    }

    #[test]
    fn test_to_profiles_active_config_location_yaml() {
        let result =
            to_profiles_active_config_location("./config/application.yaml", "prod").unwrap();
        assert_eq!(result, "./config/application-prod.yaml");
    }

    #[test]
    fn test_to_profiles_active_config_location_no_ext() {
        let result = to_profiles_active_config_location("application", "dev");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_profiles_active_from_config() {
        let yaml = r#"
Server:
  Name: test
  Profiles:
    Active: dev
"#;
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let result = get_profiles_active_from_config(&config);
        assert_eq!(result, Some("dev".to_string()));
    }

    #[test]
    fn test_get_profiles_active_from_config_empty() {
        let yaml = r#"
Server:
  Name: test
"#;
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let result = get_profiles_active_from_config(&config);
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_get_profiles_active_from_env() {
        set_env(PROFILES_ACTIVE_ENV_KEY, "staging");
        let result = get_profiles_active_from_env();
        assert_eq!(result, Some("staging".to_string()));
        remove_env(PROFILES_ACTIVE_ENV_KEY);
    }

    #[test]
    #[serial]
    fn test_detect_profiles_active_none() {
        remove_env(PROFILES_ACTIVE_ENV_KEY);
        let yaml = "Server:
  Name: test
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let result = detect_profiles_active(&config);
        assert!(result.is_none());
    }
