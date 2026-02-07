use crate::xutil;
use std::sync::OnceLock;

/// 全局 HTTP 客户端
pub(crate) static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 获取全局 HTTP 客户端
pub fn c() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        xutil::warn_if_enable_debug(
            "XHttp client accessed before init, using default configuration",
        );
        reqwest::Client::new()
    })
}

// 便捷方法，直接使用全局 client 发起请求

pub fn get(url: &str) -> reqwest::RequestBuilder {
    c().get(url)
}

pub fn post(url: &str) -> reqwest::RequestBuilder {
    c().post(url)
}

pub fn put(url: &str) -> reqwest::RequestBuilder {
    c().put(url)
}

pub fn patch(url: &str) -> reqwest::RequestBuilder {
    c().patch(url)
}

pub fn delete(url: &str) -> reqwest::RequestBuilder {
    c().delete(url)
}

pub fn head(url: &str) -> reqwest::RequestBuilder {
    c().head(url)
}