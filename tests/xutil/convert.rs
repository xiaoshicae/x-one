use std::time::Duration;
use x_one::xutil::convert::*;

    #[test]
    fn test_to_duration_empty() {
        assert_eq!(to_duration("").unwrap(), Duration::ZERO);
    }

    #[test]
    fn test_to_duration_seconds() {
        assert_eq!(to_duration("1s").unwrap(), Duration::from_secs(1));
    }

    #[test]
    fn test_to_duration_milliseconds() {
        assert_eq!(to_duration("100ms").unwrap(), Duration::from_millis(100));
    }

    #[test]
    fn test_to_duration_minutes() {
        assert_eq!(to_duration("5m").unwrap(), Duration::from_secs(300));
    }

    #[test]
    fn test_to_duration_hours() {
        assert_eq!(to_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn test_to_duration_day() {
        assert_eq!(to_duration("1d").unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn test_to_duration_day_with_hours() {
        assert_eq!(
            to_duration("2d12h").unwrap(),
            Duration::from_secs(2 * 86400 + 12 * 3600)
        );
    }

    #[test]
    fn test_to_duration_complex() {
        assert_eq!(to_duration("1h30m").unwrap(), Duration::from_secs(5400));
    }

    #[test]
    fn test_to_duration_invalid_day_fallback() {
        // "abcd12h" -> 天数解析失败，回退解析 "12h"
        assert_eq!(to_duration("abcd12h").unwrap(), Duration::from_secs(12 * 3600));
    }

    #[test]
    fn test_to_duration_7d() {
        assert_eq!(to_duration("7d").unwrap(), Duration::from_secs(7 * 86400));
    }
