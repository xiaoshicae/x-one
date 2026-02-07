use x_one::xhook::options::*;

    #[test]
    fn test_default_options() {
        let opts = HookOptions::default();
        assert_eq!(opts.order, 100);
        assert!(opts.must_invoke_success);
    }

    // default values covered by test_default_options
