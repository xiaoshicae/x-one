use x_one::xhttp::*;

#[test]
fn test_register_hook_idempotent() {
    register_hook();
    register_hook();
}

#[test]
fn test_client_accessible() {
    let c = c();
    let _ = c;
}

#[test]
fn test_get_request_builder() {
    let _builder = get("http://example.com");
}

#[test]
fn test_post_request_builder() {
    let _builder = post("http://example.com");
}

#[test]
fn test_put_request_builder() {
    let _builder = put("http://example.com");
}

#[test]
fn test_delete_request_builder() {
    let _builder = delete("http://example.com");
}

#[test]
fn test_patch_request_builder() {
    let _builder = patch("http://example.com");
}
