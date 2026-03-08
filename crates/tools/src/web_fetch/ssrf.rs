//! SSRF protection - blocks requests to private/internal IP ranges
//!
//! Prevents Server-Side Request Forgery by validating URLs before fetching.
//! Blocks loopback, private, link-local, and cloud metadata IP ranges,
//! as well as dangerous URL schemes like file:// and gopher://.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use url::Url;

/// Blocked URL schemes that should never be fetched
const BLOCKED_SCHEMES: &[&str] = &["file", "gopher", "ftp", "data", "dict", "ldap"];

/// Known hostnames that resolve to loopback/private addresses
const BLOCKED_HOSTNAMES: &[&str] = &["localhost", "localhost.localdomain"];

/// Check if a URL is safe to fetch (not targeting internal/private resources).
///
/// Returns `Ok(())` if safe, or `Err(reason)` if the URL should be blocked.
pub(crate) fn validate_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

    // Block dangerous URL schemes
    let scheme = parsed.scheme();
    if BLOCKED_SCHEMES.contains(&scheme) {
        return Err(format!(
            "Blocked URL scheme '{}://'. Only http:// and https:// are allowed.",
            scheme
        ));
    }

    // Only allow http and https
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "Unsupported URL scheme '{}://'. Only http:// and https:// are allowed.",
            scheme
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;

    // Block known private hostnames
    let host_lower = host.to_lowercase();
    if BLOCKED_HOSTNAMES.contains(&host_lower.as_str()) {
        return Err(format!(
            "Blocked request to '{}': resolves to a private/loopback address.",
            host
        ));
    }

    // Try to parse host directly as an IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err(format!(
                "Blocked request to private/internal IP address: {}",
                ip
            ));
        }
        return Ok(());
    }

    // For hostnames, resolve via DNS and check all resolved IPs
    let port = parsed.port().unwrap_or(match scheme {
        "https" => 443,
        _ => 80,
    });

    let socket_addr = format!("{}:{}", host, port);
    match socket_addr.to_socket_addrs() {
        Ok(addrs) => {
            let resolved: Vec<_> = addrs.collect();
            if resolved.is_empty() {
                return Err(format!("DNS resolution failed for host: {}", host));
            }
            for addr in &resolved {
                if is_private_ip(&addr.ip()) {
                    return Err(format!(
                        "Blocked request to '{}': resolves to private/internal IP address {}",
                        host,
                        addr.ip()
                    ));
                }
            }
            Ok(())
        }
        Err(_) => {
            // DNS resolution failed -- allow the request to proceed.
            // The HTTP client will fail with a more descriptive error.
            Ok(())
        }
    }
}

/// Check if an IP address is in a private/internal range that should be blocked.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
        IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
    }
}

/// Check if an IPv4 address is private/internal.
///
/// Blocked ranges:
/// - 127.0.0.0/8    (loopback)
/// - 10.0.0.0/8     (private, RFC 1918)
/// - 172.16.0.0/12  (private, RFC 1918)
/// - 192.168.0.0/16 (private, RFC 1918)
/// - 169.254.0.0/16 (link-local, includes cloud metadata at 169.254.169.254)
/// - 0.0.0.0/8      (current network)
fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    matches!(
        octets,
        [127, ..] |           // 127.0.0.0/8 loopback
        [10, ..] |            // 10.0.0.0/8 private
        [172, 16..=31, ..] |  // 172.16.0.0/12 private
        [192, 168, ..] |      // 192.168.0.0/16 private
        [169, 254, ..] |      // 169.254.0.0/16 link-local / cloud metadata
        [0, ..]               // 0.0.0.0/8 current network
    )
}

/// Check if an IPv6 address is private/internal.
///
/// Blocked:
/// - ::1             (loopback)
/// - fc00::/7        (unique local, i.e. fc00::/8 and fd00::/8)
/// - fe80::/10       (link-local)
/// - ::ffff:0:0/96   (IPv4-mapped — delegate to IPv4 check)
fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    // Loopback
    if *ip == Ipv6Addr::LOCALHOST {
        return true;
    }

    // Check IPv4-mapped addresses (::ffff:x.x.x.x)
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return is_private_ipv4(&ipv4);
    }

    let segments = ip.segments();
    let first_byte = (segments[0] >> 8) as u8;

    // fc00::/7 — unique local addresses (first byte fc or fd)
    if first_byte == 0xfc || first_byte == 0xfd {
        return true;
    }

    // fe80::/10 — link-local
    if segments[0] & 0xffc0 == 0xfe80 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- URL scheme tests ---

    #[test]
    fn test_blocks_file_scheme() {
        let result = validate_url("file:///etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Blocked URL scheme"));
    }

    #[test]
    fn test_blocks_gopher_scheme() {
        let result = validate_url("gopher://evil.com/");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Blocked URL scheme"));
    }

    #[test]
    fn test_blocks_ftp_scheme() {
        let result = validate_url("ftp://internal-server/data");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Blocked URL scheme"));
    }

    #[test]
    fn test_blocks_data_scheme() {
        let result = validate_url("data:text/html,<h1>hello</h1>");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Blocked URL scheme"));
    }

    #[test]
    fn test_allows_https() {
        // This will try DNS resolution for example.com which should work
        let result = validate_url("https://example.com/");
        assert!(result.is_ok());
    }

    // --- Hostname tests ---

    #[test]
    fn test_blocks_localhost() {
        let result = validate_url("https://localhost/secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("private/loopback"));
    }

    #[test]
    fn test_blocks_localhost_case_insensitive() {
        let result = validate_url("https://LOCALHOST/secret");
        assert!(result.is_err());
    }

    // --- IPv4 private range tests ---

    #[test]
    fn test_blocks_loopback_127() {
        let result = validate_url("http://127.0.0.1/");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("private/internal IP"));
    }

    #[test]
    fn test_blocks_loopback_127_other() {
        let result = validate_url("http://127.0.0.2/");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_10_range() {
        let result = validate_url("http://10.0.0.1/");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("private/internal IP"));
    }

    #[test]
    fn test_blocks_172_16_range() {
        let result = validate_url("http://172.16.0.1/");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_172_31_range() {
        let result = validate_url("http://172.31.255.255/");
        assert!(result.is_err());
    }

    #[test]
    fn test_allows_172_outside_range() {
        // 172.32.0.1 is NOT in 172.16.0.0/12, so it should be allowed
        let result = validate_url("http://172.32.0.1/");
        // This is a public IP, should be allowed
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocks_192_168_range() {
        let result = validate_url("http://192.168.1.1/");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_link_local_169_254() {
        let result = validate_url("http://169.254.169.254/latest/meta-data/");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("private/internal IP"));
    }

    #[test]
    fn test_blocks_zero_network() {
        let result = validate_url("http://0.0.0.0/");
        assert!(result.is_err());
    }

    // --- IPv6 tests ---

    #[test]
    fn test_blocks_ipv6_loopback() {
        let result = validate_url("http://[::1]/");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("private/internal IP"));
    }

    #[test]
    fn test_blocks_ipv6_unique_local_fc() {
        let result = validate_url("http://[fc00::1]/");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_ipv6_unique_local_fd() {
        let result = validate_url("http://[fd12:3456::1]/");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_ipv6_link_local() {
        let result = validate_url("http://[fe80::1]/");
        assert!(result.is_err());
    }

    // --- is_private_ip unit tests ---

    #[test]
    fn test_is_private_ipv4_loopback() {
        assert!(is_private_ipv4(&Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(127, 255, 255, 255)));
    }

    #[test]
    fn test_is_private_ipv4_rfc1918() {
        assert!(is_private_ipv4(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(10, 255, 255, 255)));
        assert!(is_private_ipv4(&Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(172, 31, 255, 255)));
        assert!(is_private_ipv4(&Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(192, 168, 255, 255)));
    }

    #[test]
    fn test_is_private_ipv4_not_private() {
        assert!(!is_private_ipv4(&Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(172, 32, 0, 1)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(192, 169, 0, 1)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(1, 1, 1, 1)));
    }

    #[test]
    fn test_is_private_ipv4_link_local() {
        assert!(is_private_ipv4(&Ipv4Addr::new(169, 254, 169, 254)));
        assert!(is_private_ipv4(&Ipv4Addr::new(169, 254, 0, 1)));
    }

    #[test]
    fn test_is_private_ipv6_loopback() {
        assert!(is_private_ipv6(&Ipv6Addr::LOCALHOST));
    }

    #[test]
    fn test_is_private_ipv6_unique_local() {
        assert!(is_private_ipv6(&"fc00::1".parse().unwrap()));
        assert!(is_private_ipv6(&"fd00::1".parse().unwrap()));
    }

    #[test]
    fn test_is_private_ipv6_link_local() {
        assert!(is_private_ipv6(&"fe80::1".parse().unwrap()));
    }

    #[test]
    fn test_is_private_ipv6_not_private() {
        assert!(!is_private_ipv6(&"2001:db8::1".parse().unwrap()));
        assert!(!is_private_ipv6(
            &"2607:f8b0:4004:800::200e".parse().unwrap()
        ));
    }

    #[test]
    fn test_is_private_ipv6_mapped_ipv4() {
        // ::ffff:127.0.0.1 should be blocked
        let mapped: Ipv6Addr = "::ffff:127.0.0.1".parse().unwrap();
        assert!(is_private_ipv6(&mapped));

        // ::ffff:8.8.8.8 should be allowed
        let public_mapped: Ipv6Addr = "::ffff:8.8.8.8".parse().unwrap();
        assert!(!is_private_ipv6(&public_mapped));
    }

    #[test]
    fn test_invalid_url() {
        let result = validate_url("not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_with_port() {
        let result = validate_url("http://127.0.0.1:8080/admin");
        assert!(result.is_err());
    }

    #[test]
    fn test_cloud_metadata_aws() {
        // AWS metadata endpoint
        let result = validate_url("http://169.254.169.254/latest/meta-data/");
        assert!(result.is_err());
    }

    #[test]
    fn test_cloud_metadata_gcp() {
        // GCP metadata uses a hostname, but 169.254.169.254 is the IP
        let result = validate_url("http://169.254.169.254/computeMetadata/v1/");
        assert!(result.is_err());
    }
}
