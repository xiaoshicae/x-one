use x_one::xutil::net::*;

    #[test]
    fn test_get_local_ip() {
        let ip = get_local_ip().unwrap();
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_extract_real_ip_specific_addr() {
        let ip = extract_real_ip("192.168.1.1").unwrap();
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_real_ip_with_port() {
        let ip = extract_real_ip("192.168.1.1:8080").unwrap();
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_real_ip_zero_addr() {
        let ip = extract_real_ip("0.0.0.0").unwrap();
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_extract_real_ip_ipv6_zero() {
        let ip = extract_real_ip("[::]").unwrap();
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_extract_real_ip_invalid() {
        let result = extract_real_ip("invalid-ip");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_real_ip_ipv6_loopback() {
        let ip = extract_real_ip("[::1]").unwrap();
        assert_eq!(ip, "::1");
    }

    #[test]
    fn test_validate_ip_valid() {
        let ip = validate_ip("192.168.1.1", "192.168.1.1").unwrap();
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_validate_ip_invalid() {
        let result = validate_ip("invalid", "invalid");
        assert!(result.is_err());
    }
