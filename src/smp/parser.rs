use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;


/// 从页面 HTML 里抓 name/value 对。覆盖 \<input> 和 JS 赋值白名单。
pub fn parse_hidden_fields(html: &str) -> HashMap<String, String> {
    static INPUT_TAG_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)<input\b[^>]*>").unwrap());
    static NAME_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)name\s*=\s*["']([^"']+)["']"#).unwrap());
    static VALUE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)value\s*=\s*["']([^"']*)["']"#).unwrap());
    static JS_FIELD_RES: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
        const KEYS: &[&str] = &[
            "exponent", "module", "nasIp", "port", "vid", "mac", "userIp",
            "sccpServerPort", "sccpEncrytKey", "sccpEncrytIV", "sccpClientPort",
            "sccpServerIp", "sccpTimestamp", "encryptIp",
            "userIndex", "loginUniqueCodeForOnline", "nickName", "onlineTime",
            "isFirstLogin", "urlBeforeLogin", "urlBeforeLogon",
        ];
        KEYS.iter()
            .map(|&key| {
                let pat = format!(
                    r#"(?:var\s+)?{}\s*=\s*["']([^"']*)["']"#,
                    regex::escape(key)
                );
                (key, Regex::new(&pat).unwrap())
            })
            .collect()
    });

    let text = html_escape::decode_html_entities(html);
    let mut fields: HashMap<String, String> = HashMap::new();

    // 1) <input name="X" value="Y"> ← 修正：真的解析 input 标签了
    for tag in INPUT_TAG_RE.find_iter(&text).map(|m| m.as_str()) {
        let Some(name_cap) = NAME_RE.captures(tag) else { continue };
        let name = name_cap.get(1).unwrap().as_str().to_string();
        let value = VALUE_RE
            .captures(tag)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        fields.insert(name, value);
    }

    // 2) JS 赋值：只取白名单（只在 <input> 里没抓到时才补）
    for (key, re) in JS_FIELD_RES.iter() {
        if fields.contains_key(*key) {
            continue;
        }
        if let Some(cap) = re.captures(&text) {
            fields.insert(key.to_string(), cap.get(1).unwrap().as_str().to_string());
        }
    }

    fields
}

/// 去掉 script/style/标签，返回页面上肉眼可见的文字。
pub fn extract_visible_text(html: &str) -> String {
    // (?is) = 大小写不敏感 + `.` 匹配换行
    static SCRIPT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
    static STYLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());
    static TAG_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
    static WS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\s+").unwrap());

    let t = SCRIPT_RE.replace_all(html, " ");
    let t = STYLE_RE.replace_all(&t, " ");
    let t = TAG_RE.replace_all(&t, " ");
    let t = html_escape::decode_html_entities(&t);
    WS_RE.replace_all(&t, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_sample() -> String {
        std::fs::read_to_string("smp_login_page.html")
            .expect("请先运行 python smp_login.py login --dump-page 生成 smp_login_page.html")
    }

    #[test]
    fn parse_from_dump() {
        let html = load_sample();
        let f = parse_hidden_fields(&html);

        // 关键字段必须存在（具体值因会话而异，不在这里断言）
        for key in ["module", "exponent", "mac", "userIp", "nasIp", "port", "vid"] {
            assert!(f.contains_key(key), "字段 {key} 未解析到，检查 HTML 结构");
            println!("{key} = {:?}", f.get(key));
        }

        // 打印全部字段，人工和 Python 输出对比
        let mut keys: Vec<_> = f.keys().collect();
        keys.sort();
        for k in keys {
            let v = &f[k];
            let preview = if v.len() > 60 { &v[..60] } else { v.as_str() };
            println!("{k} = {preview}{}", if v.len() > 60 { "..." } else { "" });
        }
    }

    #[test]
    fn visible_text_contains_something() {
        let html = load_sample();
        let text = extract_visible_text(&html);
        println!("visible = {text:?}");
        assert!(!text.is_empty(), "可见文字为空，正则可能有问题");
    }
}