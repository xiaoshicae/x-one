use x_one::xpipeline::{EndFrame, ErrorFrame, Frame, MetadataFrame, StartFrame};

#[test]
fn test_start_frame_type() {
    let f = StartFrame::new("context");
    assert_eq!(f.frame_type(), "start");
}

#[test]
fn test_end_frame_type() {
    let f = EndFrame;
    assert_eq!(f.frame_type(), "end");
}

#[test]
fn test_error_frame_type() {
    let f = ErrorFrame {
        err: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test")),
        message: "test error".to_string(),
    };
    assert_eq!(f.frame_type(), "error");
}

#[test]
fn test_metadata_frame_type() {
    let f = MetadataFrame::new("key", 42);
    assert_eq!(f.frame_type(), "metadata");
    assert_eq!(f.key, "key");
}
