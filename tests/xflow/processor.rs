use x_one::xflow::Dependency;

#[test]
fn test_dependency_display_strong() {
    assert_eq!(Dependency::Strong.to_string(), "strong");
}

#[test]
fn test_dependency_display_weak() {
    assert_eq!(Dependency::Weak.to_string(), "weak");
}

#[test]
fn test_dependency_eq() {
    assert_eq!(Dependency::Strong, Dependency::Strong);
    assert_eq!(Dependency::Weak, Dependency::Weak);
    assert_ne!(Dependency::Strong, Dependency::Weak);
}
