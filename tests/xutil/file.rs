use x_one::xutil::file::*;

    #[test]
    fn test_file_exist_exists() {
        assert!(file_exist("Cargo.toml"));
    }

    #[test]
    fn test_file_exist_not_exists() {
        assert!(!file_exist("nonexistent_file.go"));
    }

    #[test]
    fn test_file_exist_is_dir() {
        assert!(!file_exist("src"));
    }

    #[test]
    fn test_dir_exist_exists() {
        assert!(dir_exist("src"));
    }

    #[test]
    fn test_dir_exist_not_exists() {
        assert!(!dir_exist("nonexistent_dir"));
    }

    #[test]
    fn test_dir_exist_is_file() {
        assert!(!dir_exist("Cargo.toml"));
    }
