use axum::http::StatusCode;
use axum::{extract::State, http::Request, middleware::Next, response::Response};
use ipnet::IpNet;
use std::{net::IpAddr, sync::Arc};

#[derive(Clone)]
pub struct AccessControl {
    allowed_proxies: Arc<Vec<IpNet>>,
    allowed_ips: Arc<Vec<IpNet>>,
}

impl AccessControl {
    pub fn new(allowed_ips: Vec<String>, allowed_proxies: Vec<String>) -> Self {
        Self {
            allowed_proxies: Arc::new(Self::compile_nets(allowed_proxies)),
            allowed_ips: Arc::new(Self::compile_nets(allowed_ips)),
        }
    }

    fn compile_nets(entries: Vec<String>) -> Vec<IpNet> {
        entries
            .into_iter()
            .filter_map(|s| {
                let s = s.trim().to_string();
                if s.is_empty() {
                    return None;
                }
                if let Ok(net) = s.parse::<IpNet>() {
                    return Some(net);
                }
                if let Ok(ip) = s.parse::<IpAddr>() {
                    let net = match ip {
                        IpAddr::V4(v4) => {
                            IpNet::V4(ipnet::Ipv4Net::new(v4, 32).expect("valid /32"))
                        }
                        IpAddr::V6(v6) => {
                            IpNet::V6(ipnet::Ipv6Net::new(v6, 128).expect("valid /128"))
                        }
                    };
                    return Some(net);
                }
                tracing::warn!("access_control: ignoring invalid IP/CIDR entry: {}", s);
                None
            })
            .collect()
    }

    fn ip_in_nets(ip: IpAddr, nets: &[IpNet]) -> bool {
        if nets.iter().any(|n| n.contains(&ip)) {
            return true;
        }
        if let IpAddr::V6(v6) = ip
            && let Some(v4) = v6.to_ipv4_mapped()
        {
            return nets.iter().any(|n| n.contains(&IpAddr::V4(v4)));
        }
        false
    }

    /// Normalizes IPv4-mapped IPv6 to plain IPv4.
    fn normalize_ip(ip: IpAddr) -> IpAddr {
        match ip {
            IpAddr::V6(v6) => {
                if let Some(v4) = v6.to_ipv4_mapped() {
                    IpAddr::V4(v4)
                } else {
                    IpAddr::V6(v6)
                }
            }
            v4 => v4,
        }
    }

    pub fn is_allowed(&self, remote: IpAddr, xff: Option<&str>) -> (bool, &'static str) {
        let remote = Self::normalize_ip(remote);

        if self.allowed_proxies.is_empty() {
            if self.allowed_ips.is_empty() {
                return (
                    true,
                    "no proxies configured and no allowed_ips -> allow all",
                );
            }
            if Self::ip_in_nets(remote, &self.allowed_ips) {
                return (true, "remote addr in allowed_ips");
            }
            return (false, "remote addr not in allowed_ips");
        }

        if !Self::ip_in_nets(remote, &self.allowed_proxies) {
            return (false, "request did not come from trusted proxy");
        }

        let client_ip: Option<IpAddr> = xff
            .and_then(|v| v.split(',').next())
            .and_then(|s| s.trim().parse().ok())
            .map(Self::normalize_ip);

        if self.allowed_ips.is_empty() {
            return (
                true,
                "Trusted proxy -> No allowed_ips configured -> allow all",
            );
        }

        if let Some(ip) = client_ip {
            if Self::ip_in_nets(ip, &self.allowed_ips) {
                return (true, "client ip from header in allowed_ips");
            }
            return (false, "client ip not allowed");
        }

        (false, "client ip not allowed")
    }
}

pub async fn access_control(
    State(ac): State<Arc<AccessControl>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let remote_addr = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0.ip());

    let xff = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok());

    match remote_addr {
        Some(remote) => {
            let (allowed, reason) = ac.is_allowed(remote, xff);
            tracing::debug!(
                "access_control: remote={} xff={:?} allowed={} reason={}",
                remote,
                xff,
                allowed,
                reason
            );
            if allowed {
                next.run(req).await
            } else {
                tracing::warn!(
                    "access_control: DENY remote={} xff={:?} reason={}",
                    remote,
                    xff,
                    reason
                );
                Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(axum::body::Body::from("Forbidden"))
                    .unwrap()
            }
        }
        None => {
            tracing::warn!("access_control: DENY — no remote address available");
            Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(axum::body::Body::from("Forbidden"))
                .unwrap()
        }
    }
}
