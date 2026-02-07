use x_one::xconfig::*;

    #[test]
    fn test_get_string_no_config() {
        reset_config();
        assert_eq!(get_string("Server.Name"), "");
    }

    #[test]
    fn test_get_server_name_default() {
        reset_config();
        assert_eq!(get_server_name(), DEFAULT_SERVER_NAME);
    }

    #[test]
    fn test_get_server_name_with_config() {
        reset_config();
        let yaml = "Server:
  Name: my-app
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        set_config(config);
        assert_eq!(get_server_name(), "my-app");
        reset_config();
    }

    #[test]
    fn test_get_server_version_default() {
        reset_config();
        assert_eq!(get_server_version(), DEFAULT_SERVER_VERSION);
    }

    #[test]
    fn test_get_int() {
        reset_config();
        let yaml = "Server:
  Gin:
    Port: 9090
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        set_config(config);
        assert_eq!(get_int("Server.Gin.Port"), 9090);
        reset_config();
    }

    #[test]
    fn test_get_bool() {
        reset_config();
        let yaml = "Server:
  Gin:
    UseHttp2: true
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        set_config(config);
        assert!(get_bool("Server.Gin.UseHttp2"));
        reset_config();
    }

    #[test]
    fn test_contain_key() {
        reset_config();
        let yaml = "Server:
  Name: test
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        set_config(config);
        assert!(contain_key("Server.Name"));
        assert!(!contain_key("Server.NonExistent"));
        reset_config();
    }

    #[test]
    fn test_get_string_slice() {
        reset_config();
        let yaml = "items:
  - a
  - b
  - c
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        set_config(config);
        assert_eq!(get_string_slice("items"), vec!["a", "b", "c"]);
        reset_config();
    }

    #[test]
    fn test_parse_config() {
        reset_config();
        let yaml = "Server:
  Gin:
    Host: '127.0.0.1'
    Port: 3000
";
        let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        set_config(config);
        let gin: GinConfig = parse_config("Server.Gin").unwrap();
        assert_eq!(gin.host, "127.0.0.1");
        assert_eq!(gin.port, 3000);
        reset_config();
    }

    #[test]
    fn test_get_gin_config_default() {
        reset_config();
        let gin = get_gin_config();
        assert_eq!(gin.host, "0.0.0.0");
        assert_eq!(gin.port, 8000);
    }

    #[test]
    fn test_get_gin_swagger_config_default() {
        reset_config();
        let swagger = get_gin_swagger_config();
        assert_eq!(swagger.schemes, vec!["https", "http"]);
    }
