use std::time::Duration;
use reqwest::blocking::Client;
use regex::Regex;
use std::sync::LazyLock;
use reqwest::Error;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use crate::smp::crypto::{build_secret, rsa_encrypt, SecretFormat};
use crate::smp::info::{local_ip_to, local_mac};
use crate::smp::parser::{extract_visible_text, parse_hidden_fields};
use crate::smp::config::SmpConfig;

/// 创建HTTP连接
pub fn make_client() -> Client {
    Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36 Edg/156.0.0.0")
        .redirect(reqwest::redirect::Policy::none())// 302不自动跟随
        .build()
        .expect("无法创建HTTP连接")
}

/// 获取302重定向URL
pub fn get_intercept_url(client: &Client) -> Result<Option<String>, Error> {
    const PROBE_URL:&str = "http://connect.rom.miui.com/generate_204";
    static HREF_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"location\.href\s*=\s*['"]([^'"]+)['"]"#).unwrap());

    let resp = client.get(PROBE_URL).send()?;

    if resp.status().as_u16() == 204 {return Ok(None)}

    if let Some(loc) = resp.headers().get("Location") {
        if let Ok(s) = loc.to_str() {
            if !s.is_empty() {
                return Ok(Some(s.to_string()));
            }
        }
    }

    let body = resp.text().unwrap_or_default();
    let unescaped = html_escape::decode_html_entities(&body);
    let href = HREF_RE.captures(&unescaped)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string());

    Ok(href)
}

/// 当前网络状态枚举
#[derive(Debug)]
pub enum AuthStatus {
    /// 在线
    Online,
    /// 被拦截，附带拦截时的跳转 URL
    Offline { intercept_url: String },
    /// 发生错误
    Failed(Error),
}

/// 获取当前网络状态
pub fn do_status(client: &Client) -> AuthStatus {
    match get_intercept_url(client) {
        Ok(s) => {
            if s.is_none() {return AuthStatus::Online}
            AuthStatus::Offline { intercept_url: s.unwrap() }
        }
        Err(r) => {
            AuthStatus::Failed(r.into())
        }
    }
}

/// 登录结果
#[derive(Debug)]
pub enum LoginResult {
    /// 探测时已经在线，无需登录
    AlreadyOnline,
    /// 登录成功，附带成功页字段
    Success {
        online_fields: HashMap<String, String>,
        visible: String,
        title: String,
    },
    /// 登录被拒绝
    Failed {
        title: String,
        visible: String,
    },
    /// 网络层失败
    NetworkError(String),
    /// 登录页里没解析到RSA模数
    MissingModule,
}

/// SMP 登录参数。
///
/// RSA 加密由 `do_login` 内部完成
pub struct LoginParams<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub secret_format: SecretFormat,
}

pub fn do_login(client: &Client, params: LoginParams) -> LoginResult {
    // 探测拦截
    let intercept_url = match get_intercept_url(client) {
        Ok(None) => return LoginResult::AlreadyOnline,
        Ok(Some(url)) => url,
        Err(e) => return LoginResult::NetworkError(format!("302重定向URL获取失败: {e}")),
    };
    let portal_ip = Url::parse(&intercept_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "192.168.8.206".parse().unwrap());

    // GET 登录页，解析隐藏字段
    let page_resp = match client.get(&intercept_url).send() {
        Ok(r) => r,
        Err(e) => return LoginResult::NetworkError(format!("GET 登录页失败: {e}")),
    };
    let page_bytes = match page_resp.bytes() {
        Ok(b) => b,
        Err(e) => return LoginResult::NetworkError(format!("读取登录页失败: {e}"))
    };
    let page_text = String::from_utf8_lossy(&page_bytes).into_owned();
    let fields = parse_hidden_fields(&page_text);

    // 从拦截 URL 取 encryptIp
    let encrypt_ip = Url::parse(&intercept_url)
        .ok()
        .and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "userip")
                .map(|(_, v)| v.to_string())
        })
        .or_else(|| fields.get("encryptIp").cloned())
        .unwrap_or_default();

    // 补齐字段
    let mac = fields.get("mac").cloned().unwrap_or_else(local_mac);
    let user_ip = fields.get("userIp").cloned()
        .unwrap_or_else(|| local_ip_to(&portal_ip));
    let nas_ip = fields.get("nasIp").cloned().unwrap_or_default();
    let port = fields.get("port").cloned().unwrap_or_else(|| "7".into());
    let vid = fields.get("vid").cloned().unwrap_or_else(|| "0".into());
    let exponent = fields.get("exponent").cloned()
        .unwrap_or_else(|| "10001".into());
    let module = match fields.get("module") {
        Some(m) if !m.is_empty() => m.clone(),
        _ => return LoginResult::MissingModule,
    };

    // 加密密码
    let secret = build_secret(params.password, params.secret_format, &mac, &user_ip);
    let enc_pwd = rsa_encrypt(&secret, &exponent, &module);

    // POST body
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body: Vec<(&str, String)> = vec![
        ("kind", "preLogin".into()),
        ("isEnableNickNameAuth", "".into()),
        ("mac", mac),
        ("port", port),
        ("urlBeforeLogon", "http://www.msftconnecttest.com/redirect".into()),
        ("vid", vid),
        ("ssid", "".into()),
        ("apmac", "".into()),
        ("userIp", user_ip.clone()),
        ("t", "".into()),
        ("enableEsaAuthen", "".into()),
        ("nasIp", nas_ip),
        ("enableNormalUserLogin", "true".into()),
        ("normalUserNotifyMd5Cookie", "".into()),
        ("findPswUrl", format!("./passwordprotectionservlet?kind=toFindPasswordPage&userIp={user_ip}")),
        ("lastUserId", fields.get("lastUserId").cloned().unwrap_or_default()),
        ("readPasswordFromCookie", "".into()),
        ("passwordInCookie", "".into()),
        ("userIdInCookie", "".into()),
        ("sccpServerPort", fields.get("sccpServerPort").cloned().unwrap_or_else(|| "53".into())),
        ("sccpEncrytKey", fields.get("sccpEncrytKey").cloned().unwrap_or_default()),
        ("sccpEncrytIV", fields.get("sccpEncrytIV").cloned().unwrap_or_default()),
        ("sccpClientPort", fields.get("sccpClientPort").cloned().unwrap_or_else(|| "138".into())),
        ("sccpServerIp", fields.get("sccpServerIp").cloned().unwrap_or_else(|| portal_ip.clone())),
        ("sccpTimestamp", now.to_string()),
        ("commonDisclaimerEnable", "false".into()),
        ("disclaimerTitle", "".into()),
        ("disclaimerLoginBtn", "".into()),
        ("loginActionName", "./commonauth".into()),
        ("loginUrl", "commonauth".into()),
        ("isSubmit", "".into()),
        ("exponent", exponent),
        ("module", module),
        ("isFromCookie", "false".into()),
        ("encryptIp", encrypt_ip),
        ("userId", params.username.to_string()),
        ("password", enc_pwd),
        ("enableCookie", "".into()),
    ];

    // POST
    let resp = match client
        .post(format!("http://{portal_ip}/smp/commonauth"))
        .header("Referer", &intercept_url)
        .header("Origin", format!("http://{portal_ip}"))
        .form(&body)
        .send()
        {
            Ok(r) => r,
            Err(e) => return LoginResult::NetworkError(format!("POST 登录失败: {e}")),
        };
    let resp_bytes = match resp.bytes() {
        Ok(b) => b,
        Err(e) => return LoginResult::NetworkError(format!("读取响应失败: {e}")),
    };
    let resp_text = String::from_utf8_lossy(&resp_bytes);
    let visible = extract_visible_text(&resp_text);

    // 提取 title
    let title = {
        static TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?is)<title>(.*?)</title>").unwrap()
        });
        TITLE_RE
            .captures(&resp_text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default()
    };

    // 判定
    let ok = title.contains("成功")
        || title.to_lowercase().contains("success")
        || visible.contains("在线");

    const BAD_KEYWORDS: &[&str] = &[
        "失败", "错误", "invalid", "fail", "密码错", "用户不存在", "已欠费", "绑定",
    ];

    let has_bad = BAD_KEYWORDS.iter().any(|k| {
        let k_lower = k.to_lowercase();
        visible.to_lowercase().contains(&k_lower) || title.to_lowercase().contains(&k_lower)
    });

    if ok && !has_bad {
        let online_fields = parse_hidden_fields(&resp_text);
        LoginResult::Success { online_fields, visible, title }
    } else {
        LoginResult::Failed { title, visible }
    }
}

/// 登出结果。
#[derive(Debug)]
pub enum LogoutResult {
    /// 已离线
    AlreadyOffline,
    /// 登出成功
    Success,
    /// 登出失败
    Failed { title: String, visible: String },
    /// 没有可用的 online_fields
    NoOnlineInfo,
    /// 网络错误
    NetworkError(String),
}

pub fn do_logout(client: &Client, config: &SmpConfig) -> LogoutResult {
    // 已离线？
    match get_intercept_url(client) {
        Ok(Some(_)) => return LogoutResult::AlreadyOffline,
        Ok(None) => {}
        Err(e) => return LogoutResult::NetworkError(format!("探测失败: {e}")),
    }

    // 取在线字段：优先配置，其次 GET 门户在线页
    let portal_ip = "192.168.8.206";
    let mut online_fields = config.online_fields.clone().unwrap_or_default();

    if !online_fields.contains_key("userIndex") {
        for path in ["/smp/commonauth", "/"] {
            let url = format!("http://{portal_ip}{path}");
            if let Ok(resp) = client.get(&url).send() {
                if let Ok(bytes) = resp.bytes() {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let f = parse_hidden_fields(&text);
                    if f.contains_key("userIndex") {
                        online_fields = f;
                        break;
                    }
                }
            }
        }
    }

    if !online_fields.contains_key("userIndex") {
        return LogoutResult::NoOnlineInfo;
    }

    // 构造登出 body
    let get = |k: &str,s: &str| online_fields.get(k).cloned().unwrap_or_else(|| s.to_string());
    let user_ip = if !get("userIp","").is_empty() { get("userIp","") } else { local_ip_to(portal_ip) };
    let mac = if !get("mac","").is_empty() { get("mac","") } else { local_mac() };
    let user_id = if !get("userId","").is_empty() { get("userId","") } else { config.username.clone() };
    let url_before = {
        let a = get("urlBeforeLogon", "");
        if !a.is_empty() { a } else {
            let b = get("urlBeforeLogin", "");
            if !b.is_empty() { b } else { "http://www.msftconnecttest.com/redirect".to_string() }
        }
    };
    let body: Vec<(&str, String)> = vec![
        ("kind", "logout".into()),
        ("loginUniqueCodeForOnline", get("loginUniqueCodeForOnline", "")),
        ("userId", user_id),
        ("nickName", get("nickName", "")),
        ("mac", mac),
        ("port", get("port", "7")),
        ("urlBeforeLogon", url_before.clone()),
        ("vid", get("vid", "0")),
        ("ssid", get("ssid", "")),
        ("apmac", get("apmac", "")),
        ("userIp", user_ip.clone()),
        ("t", get("t", "")),
        ("nasIp", get("nasIp", "")),
        ("secZoneName", "".into()),
        ("isMessageExist", "".into()),
        ("isPatchExist", "".into()),
        ("ssoUrl", "".into()),
        ("urlBeforeLogin", url_before.clone()),
        ("loginPublishUrl", "".into()),
        ("enableCookie", "true".into()),
        ("logoutCode", "1".into()),
        ("onlineTime",get("onlineTime", "0")),
        ("isRedirectModify", "true".into()),
        ("userIndex", get("userIndex", "")),
        ("pswProtectionUrl", format!(
            "./passwordprotectionservlet?kind=toPswProtectionConfigFromWeb\
             &urlBeforeLogin={url_before}&isFromSuccessPage=true&userIp={user_ip}"
        )),
        ("isFirstLogin", get("isFirstLogin", "false")),
        ("broadcastUrl", "".into()),
        ("onlineInfoShowEnable", "false".into()),
    ];

    // POST
    let post_url = format!("http://{portal_ip}/smp/commonauth");
    let resp = match client
        .post(&post_url)
        .header("Referer", &post_url)
        .header("Origin", format!("http://{portal_ip}"))
        .form(&body)
        .send()
    {
        Ok(r) => r,
        Err(e) => return LogoutResult::NetworkError(format!("POST 登出失败: {e}")),
    };

    let bytes = match resp.bytes() {
        Ok(b) => b,
        Err(e) => return LogoutResult::NetworkError(format!("读取响应失败: {e}")),
    };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let visible = extract_visible_text(&text);

    // 复探：登出成功后应重新被拦截
    std::thread::sleep(Duration::from_secs(1));
    match get_intercept_url(client) {
        Ok(Some(_)) => LogoutResult::Success,
        Ok(None) => LogoutResult::Failed { title: String::new(), visible },
        Err(e) => LogoutResult::NetworkError(format!("复探失败: {e}")),
    }
}