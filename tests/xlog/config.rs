use x_one::xlog::config::*;

    #[test]
    fn test_default_config_values() {
        let c = XLogConfig::default();
        assert_eq!(c.name, "app");
        assert_eq!(c.level, LogLevel::Info);
        assert_eq!(c.path, "./log");
        assert_eq!(c.max_age, "7d");
        assert_eq!(c.rotate_time, "1d");
        assert_eq!(c.timezone, "Asia/Shanghai");
        assert!(!c.console);
        assert!(!c.console_format_is_raw);
    }

    #[test]
    fn test_config_with_values() {
        let c = XLogConfig {
            level: LogLevel::Debug,
            name: "myapp".to_string(),
            path: "/var/log".to_string(),
            console: true,
            ..Default::default()
        };
        assert_eq!(c.level, LogLevel::Debug);
        assert_eq!(c.name, "myapp");
        assert_eq!(c.path, "/var/log");
        assert!(c.console);
    }
