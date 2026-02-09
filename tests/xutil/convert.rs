use std::time::Duration;
use x_one::xutil::convert::*;

#[test]
fn test_to_duration_empty() {
    assert_eq!(to_duration(""), Some(Duration::ZERO));
}

#[test]
fn test_to_duration_seconds() {
    assert_eq!(to_duration("1s"), Some(Duration::from_secs(1)));
}

#[test]
fn test_to_duration_milliseconds() {
    assert_eq!(to_duration("100ms"), Some(Duration::from_millis(100)));
}

#[test]
fn test_to_duration_minutes() {
    assert_eq!(to_duration("5m"), Some(Duration::from_secs(300)));
}

#[test]
fn test_to_duration_hours() {
    assert_eq!(to_duration("2h"), Some(Duration::from_secs(7200)));
}

#[test]
fn test_to_duration_day() {
    assert_eq!(to_duration("1d"), Some(Duration::from_secs(86400)));
}

#[test]
fn test_to_duration_day_with_hours() {
    assert_eq!(
        to_duration("2d12h"),
        Some(Duration::from_secs(2 * 86400 + 12 * 3600))
    );
}

#[test]
fn test_to_duration_complex() {
    assert_eq!(to_duration("1h30m"), Some(Duration::from_secs(5400)));
}

#[test]
fn test_to_duration_invalid_returns_none() {
    assert_eq!(to_duration("abcd12h"), None);
}

#[test]
fn test_to_duration_7d() {
    assert_eq!(to_duration("7d"), Some(Duration::from_secs(7 * 86400)));
}
