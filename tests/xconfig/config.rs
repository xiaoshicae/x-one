use x_one::xconfig::config::*;

    #[test]
    fn test_server_config_default_version() {
        let c = ServerConfig::default();
        assert_eq!(c.version, "v0.0.1");
    }

    #[test]
    fn test_auxm_config_default_values() {
        let c = AuxmConfig::default();
        assert_eq!(c.host, "0.0.0.0");
        assert_eq!(c.port, 8000);
        assert!(!c.use_http2);
    }

    #[test]
    fn test_auxm_swagger_config_default_schemes() {
        let c = AuxmSwaggerConfig::default();
        assert_eq!(c.schemes, vec!["https", "http"]);
    }
