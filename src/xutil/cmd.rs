//! 命令行参数解析工具

/// 从启动命令行参数中获取指定 key 的值
///
/// 仅识别 `--` 双破折号前缀，支持两种格式：
/// - 空格分隔：`--key value`
/// - 等号分隔：`--key=value`
///
/// key 不合法或找不到对应参数时返回 `None`。
pub fn get_config_from_args(key: &str) -> Option<String> {
    let args = get_os_args();
    find_arg_value(key, &args)
}

// ---- 以下为私有实现 ----

/// 在参数列表中查找指定 key 的值
fn find_arg_value(key: &str, args: &[String]) -> Option<String> {
    if !is_valid_arg_key(key) {
        return None;
    }

    let eq_suffix = format!("{key}=");

    for (i, arg) in args.iter().enumerate() {
        let Some(after_dash) = arg.strip_prefix("--") else {
            continue;
        };

        // 空格分隔：--config value
        if after_dash == key {
            return args.get(i + 1).cloned();
        }

        // 等号分隔：--config=value
        if let Some(value) = after_dash.strip_prefix(&eq_suffix) {
            return Some(value.to_string());
        }
    }

    None
}

/// 获取进程启动参数（跳过第一个可执行文件路径）
fn get_os_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

/// 校验参数 key 是否合法：首字符为字母或下划线，后续为字母、数字、下划线、点或横线
fn is_valid_arg_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}
