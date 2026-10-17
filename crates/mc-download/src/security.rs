use crate::errors::redirect_error;
use mc_core::{AppError, ErrorCode};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use url::Url;

pub(crate) fn validate_url(url: &Url, allowed_hosts: &HashSet<String>) -> Result<(), AppError> {
    let host = url
        .host_str()
        .ok_or_else(|| redirect_error("URL has no host"))?;
    let valid = url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && url.port_or_known_default() == Some(443)
        && !host.parse::<IpAddr>().is_ok()
        && allowed_hosts.contains(&host.to_ascii_lowercase());
    if !valid {
        return Err(redirect_error("URL is outside the trusted HTTPS origins"));
    }
    Ok(())
}

pub(crate) async fn resolve_public_address(
    url: &Url,
    allowed_hosts: &HashSet<String>,
) -> Result<SocketAddr, AppError> {
    validate_url(url, allowed_hosts)?;
    let host = url.host_str().expect("validated URL host");
    let port = url.port_or_known_default().expect("validated URL port");
    let addresses = tokio::net::lookup_host((host, port)).await.map_err(|_| {
        AppError::new(
            ErrorCode::SourceUnavailable,
            "Could not resolve artifact source.",
        )
    })?;
    let mut public_addresses = Vec::new();
    let mut found = false;
    for address in addresses {
        found = true;
        if !is_public_ip(address.ip()) {
            return Err(redirect_error("DNS returned a non-public address"));
        }
        public_addresses.push(address);
    }
    public_addresses.into_iter().next().ok_or_else(|| {
        if found {
            redirect_error("DNS returned no public address")
        } else {
            AppError::new(
                ErrorCode::SourceUnavailable,
                "Artifact source has no address.",
            )
        }
    })
}

pub(crate) fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_broadcast()
                || ip.is_unspecified()
                || octets[0] == 0
                || octets[0] >= 240
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            let is_v4_mapped = segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff;
            let mapped_is_public = if is_v4_mapped {
                let v4 = std::net::Ipv4Addr::new(
                    (segments[6] >> 8) as u8,
                    segments[6] as u8,
                    (segments[7] >> 8) as u8,
                    segments[7] as u8,
                );
                is_public_ip(IpAddr::V4(v4))
            } else {
                true
            };
            mapped_is_public
                && !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !ip.is_multicast()
                && (segments[0] & 0xe000) == 0x2000
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
        }
    }
}
