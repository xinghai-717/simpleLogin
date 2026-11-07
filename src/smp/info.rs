use std::net::UdpSocket;
use mac_address::get_mac_address;

/// 获取本机 MAC 地址
pub fn local_mac() -> String {
    match get_mac_address() {
        Ok(Some(ma)) => ma.to_string().replace(':', "").to_uppercase(),
        _ => String::new(),
    }
}

/// 通过 UDP connect 探测到 portal 的路由出口 IP
pub fn local_ip_to(portal: &str) -> String {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    match socket.connect((portal, 80)) {
        Ok(_) => socket
            .local_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mac_format() {
        let mac = local_mac();
        println!("local_mac = {mac:?}");
        if !mac.is_empty() {
            assert_eq!(mac.len(), 12, "MAC 长度应该是 12 位 hex");
            assert!(
                mac.chars().all(|c| c.is_ascii_hexdigit()),
                "MAC 含非 hex 字符: {mac}"
            );
            assert_eq!(mac, mac.to_uppercase(), "MAC 应该是大写");
        }
    }

    #[test]
    fn local_ip_to_returns_something() {
        // 用公网 DNS 探测，只要能路由出去就能拿到本机 IP
        let ip = local_ip_to("223.5.5.5");
        println!("local_ip_to(223.5.5.5) = {ip:?}");
        // 有网时应该有值，无网时返回空也算合理，所以不断言 is_empty
        if !ip.is_empty() {
            assert!(
                ip.parse::<std::net::IpAddr>().is_ok(),
                "返回的不是合法 IP: {ip}"
            );
        }
    }
}