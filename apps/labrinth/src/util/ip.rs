use std::net::{AddrParseError, IpAddr, Ipv6Addr};

pub fn convert_to_ip_v6(src: &str) -> Result<Ipv6Addr, AddrParseError> {
    let ip_addr: IpAddr = src.parse()?;

    Ok(match ip_addr {
        IpAddr::V4(x) => x.to_ipv6_mapped(),
        IpAddr::V6(x) => x,
    })
}

pub fn strip_ip(ip: Ipv6Addr) -> u64 {
    if let Some(ip) = ip.to_ipv4_mapped() {
        let octets = ip.octets();
        u64::from_be_bytes([
            octets[0], octets[1], octets[2], octets[3], 0, 0, 0, 0,
        ])
    } else {
        let octets = ip.octets();
        u64::from_be_bytes([
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5],
            octets[6], octets[7],
        ])
    }
}

/// 把 IP 转为激励系统去重身份字符串：
/// IPv4 取完整 32 位；IPv6 取前 64 位（一条 ISP 线路 = 一个身份）。
pub fn ip_to_identity_64(ip: Ipv6Addr) -> String {
    if let Some(v4) = ip.to_ipv4_mapped() {
        let o = v4.octets();
        format!("ip4_{}.{}.{}.{}", o[0], o[1], o[2], o[3])
    } else {
        let s = ip.segments();
        format!("ip6_{:x}:{:x}:{:x}:{:x}", s[0], s[1], s[2], s[3])
    }
}
