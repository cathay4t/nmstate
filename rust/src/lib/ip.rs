use std::net::IpAddr;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{DnsClientState, ErrorKind, Interfaces, NmstateError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct InterfaceIpv4 {
    #[serde(default, deserialize_with = "crate::deserializer::bool_or_string")]
    pub enabled: bool,
    #[serde(skip)]
    pub(crate) prop_list: Vec<&'static str>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub dhcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "address")]
    pub addresses: Option<Vec<InterfaceIpAddr>>,
    #[serde(skip)]
    pub(crate) dns: Option<DnsClientState>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-dns",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub auto_dns: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-gateway",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub auto_gateway: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-routes",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub auto_routes: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-route-table-id",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub auto_table_id: Option<u32>,
}

impl InterfaceIpv4 {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_auto(&self) -> bool {
        self.enabled && self.dhcp == Some(true)
    }

    pub(crate) fn update(&mut self, other: &Self) {
        if other.prop_list.contains(&"enabled") {
            self.enabled = other.enabled;
        }

        if other.prop_list.contains(&"dhcp") {
            self.dhcp = other.dhcp;
        }

        if other.prop_list.contains(&"addresses") {
            self.addresses = other.addresses.clone();
        }
        if other.prop_list.contains(&"dns") {
            self.dns = other.dns.clone();
        }
        if other.prop_list.contains(&"auto_dns") {
            self.auto_dns = other.auto_dns;
        }
        if other.prop_list.contains(&"auto_gateway") {
            self.auto_gateway = other.auto_gateway;
        }
        if other.prop_list.contains(&"auto_routes") {
            self.auto_routes = other.auto_routes;
        }
        if other.prop_list.contains(&"auto_table_id") {
            self.auto_table_id = other.auto_table_id;
        }

        for other_prop_name in &other.prop_list {
            if !self.prop_list.contains(other_prop_name) {
                self.prop_list.push(other_prop_name);
            }
        }
        self.cleanup()
    }

    // * Disable DHCP and remove address if enabled: false
    // * Set DHCP options to None if DHCP is false
    fn cleanup(&mut self) {
        if !self.enabled {
            self.dhcp = None;
            self.addresses = None;
        }

        if self.dhcp != Some(true) {
            self.auto_dns = None;
            self.auto_gateway = None;
            self.auto_routes = None;
            self.auto_table_id = None;
        }
    }

    // Clean up before sending to plugin for applying
    // * Set auto_dns, auto_gateway and auto_routes to true if DHCP enabled and
    //   those options is None
    // * Remove static IP address when DHCP enabled.
    pub(crate) fn pre_edit_cleanup(&mut self) {
        if self.is_auto() {
            if self.auto_dns.is_none() {
                self.auto_dns = Some(true);
            }
            if self.auto_routes.is_none() {
                self.auto_routes = Some(true);
            }
            if self.auto_gateway.is_none() {
                self.auto_gateway = Some(true);
            }
            if !self.addresses.as_deref().unwrap_or_default().is_empty() {
                log::warn!(
                    "Static addresses {:?} are ignored when dynamic \
                    IP is enabled",
                    self.addresses.as_deref().unwrap_or_default()
                );
                self.addresses = None;
            }
        }
        self.cleanup();
    }

    // Clean up before verification
    // * Sort IP address
    // * Ignore DHCP options if DHCP disabled
    // * Ignore address if DHCP enabled
    // * Set DHCP as off if enabled and dhcp is None
    pub(crate) fn pre_verify_cleanup(&mut self) {
        self.cleanup();
        if self.dhcp == Some(true) {
            self.addresses = None;
        }
        if let Some(addrs) = self.addresses.as_mut() {
            addrs.sort_unstable_by(|a, b| {
                (&a.ip, a.prefix_length).cmp(&(&b.ip, b.prefix_length))
            })
        };
        if self.dhcp != Some(true) {
            self.dhcp = Some(false);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InterfaceIpv6 {
    #[serde(default, deserialize_with = "crate::deserializer::bool_or_string")]
    pub enabled: bool,
    #[serde(skip)]
    pub(crate) prop_list: Vec<&'static str>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub dhcp: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub autoconf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "address")]
    pub addresses: Option<Vec<InterfaceIpAddr>>,
    #[serde(skip)]
    pub(crate) dns: Option<DnsClientState>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename = "auto-dns",
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub auto_dns: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename = "auto-gateway",
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub auto_gateway: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-routes",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub auto_routes: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-route-table-id",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub auto_table_id: Option<u32>,
}

impl InterfaceIpv6 {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_auto(&self) -> bool {
        self.enabled && (self.dhcp == Some(true) || self.autoconf == Some(true))
    }

    // * Disable DHCP and remove address if enabled: false
    // * Set DHCP options to None if DHCP is false
    fn cleanup(&mut self) {
        if !self.enabled {
            self.dhcp = None;
            self.autoconf = None;
            self.addresses = None;
        }

        if !self.is_auto() {
            self.auto_dns = None;
            self.auto_gateway = None;
            self.auto_routes = None;
            self.auto_table_id = None;
        }
    }

    pub(crate) fn update(&mut self, other: &Self) {
        if other.prop_list.contains(&"enabled") {
            self.enabled = other.enabled;
        }
        if other.prop_list.contains(&"dhcp") {
            self.dhcp = other.dhcp;
        }
        if other.prop_list.contains(&"autoconf") {
            self.autoconf = other.autoconf;
        }
        if other.prop_list.contains(&"addresses") {
            self.addresses = other.addresses.clone();
        }
        if other.prop_list.contains(&"auto_dns") {
            self.auto_dns = other.auto_dns;
        }
        if other.prop_list.contains(&"auto_gateway") {
            self.auto_gateway = other.auto_gateway;
        }
        if other.prop_list.contains(&"auto_routes") {
            self.auto_routes = other.auto_routes;
        }
        if other.prop_list.contains(&"auto_table_id") {
            self.auto_table_id = other.auto_table_id;
        }
        if other.prop_list.contains(&"dns") {
            self.dns = other.dns.clone();
        }
        for other_prop_name in &other.prop_list {
            if !self.prop_list.contains(other_prop_name) {
                self.prop_list.push(other_prop_name);
            }
        }
        self.cleanup()
    }

    // Clean up before verification
    // * Remove link-local address
    // * Ignore DHCP options if DHCP disabled
    // * Ignore IP address when DHCP/autoconf enabled.
    // * Set DHCP None to Some(false)
    pub(crate) fn pre_verify_cleanup(&mut self) {
        self.cleanup();
        if self.is_auto() {
            self.addresses = None;
        }
        if let Some(addrs) = self.addresses.as_mut() {
            addrs.retain(|addr| {
                !is_ipv6_unicast_link_local(
                    &addr.ip.to_string(),
                    addr.prefix_length,
                )
            })
        };
        if let Some(addrs) = self.addresses.as_mut() {
            addrs.sort_unstable_by(|a, b| {
                (&a.ip, a.prefix_length).cmp(&(&b.ip, b.prefix_length))
            })
        };
        if self.dhcp != Some(true) {
            self.dhcp = Some(false);
        }
        if self.autoconf != Some(true) {
            self.autoconf = Some(false);
        }
    }

    // Clean up before Apply
    // * Remove link-local address
    // * Set auto_dns, auto_gateway and auto_routes to true if DHCP/autoconf
    //   enabled and those options is None
    // * Remove static IP address when DHCP/autoconf enabled.
    pub(crate) fn pre_edit_cleanup(&mut self) {
        if let Some(addrs) = self.addresses.as_mut() {
            addrs.retain(|addr| {
                if is_ipv6_unicast_link_local(
                    &addr.ip.to_string(),
                    addr.prefix_length,
                ) {
                    log::warn!(
                        "Ignoring IPv6 link local address {}/{}",
                        &addr.ip,
                        addr.prefix_length
                    );
                    false
                } else {
                    true
                }
            })
        };
        if self.is_auto() {
            if self.auto_dns.is_none() {
                self.auto_dns = Some(true);
            }
            if self.auto_routes.is_none() {
                self.auto_routes = Some(true);
            }
            if self.auto_gateway.is_none() {
                self.auto_gateway = Some(true);
            }
            if !self.addresses.as_deref().unwrap_or_default().is_empty() {
                log::warn!(
                    "Static addresses {:?} are ignored when dynamic \
                    IP is enabled",
                    self.addresses.as_deref().unwrap_or_default()
                );
                self.addresses = None;
            }
        }
        self.cleanup();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct InterfaceIpAddr {
    pub ip: IpAddr,
    #[serde(deserialize_with = "crate::deserializer::u8_or_string")]
    pub prefix_length: u8,
}

impl Default for InterfaceIpAddr {
    fn default() -> Self {
        Self {
            ip: IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            prefix_length: 128,
        }
    }
}

pub(crate) fn is_ipv6_addr(addr: &str) -> bool {
    addr.contains(':')
}

// TODO: Rust offical has std::net::Ipv6Addr::is_unicast_link_local() in
// experimental.
fn is_ipv6_unicast_link_local(ip: &str, prefix: u8) -> bool {
    // The unicast link local address range is fe80::/10.
    is_ipv6_addr(ip)
        && ip.len() >= 3
        && ["fe8", "fe9", "fea", "feb"].contains(&&ip[..3])
        && prefix >= 10
}

impl std::convert::TryFrom<&str> for InterfaceIpAddr {
    type Error = NmstateError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut addr: Vec<&str> = value.split('/').collect();
        addr.resize(2, "");
        let ip = IpAddr::from_str(addr[0]).map_err(|e| {
            let e = NmstateError::new(
                ErrorKind::InvalidArgument,
                format!("Invalid IP address {}: {}", addr[0], e),
            );
            log::error!("{}", e);
            e
        })?;

        let prefix_length = if addr[1].is_empty() {
            if ip.is_ipv6() {
                128
            } else {
                32
            }
        } else {
            addr[1].parse::<u8>().map_err(|parse_error| {
                let e = NmstateError::new(
                    ErrorKind::InvalidArgument,
                    format!("Invalid IP address {}: {}", value, parse_error),
                );
                log::error!("{}", e);
                e
            })?
        };
        Ok(Self { ip, prefix_length })
    }
}

impl std::convert::From<&InterfaceIpAddr> for String {
    fn from(v: &InterfaceIpAddr) -> String {
        format!("{}/{}", &v.ip, v.prefix_length)
    }
}

pub(crate) fn include_current_ip_address_if_dhcp_on_to_off(
    chg_net_state: &mut Interfaces,
    current: &Interfaces,
) {
    for (iface_name, iface) in chg_net_state.kernel_ifaces.iter_mut() {
        let cur_iface = if let Some(c) = current.kernel_ifaces.get(iface_name) {
            c
        } else {
            continue;
        };
        if let Some(cur_ip_conf) = cur_iface.base_iface().ipv4.as_ref() {
            if cur_ip_conf.is_auto() && cur_ip_conf.addresses.is_some() {
                if let Some(ip_conf) = iface.base_iface_mut().ipv4.as_mut() {
                    if ip_conf.enabled
                        && !ip_conf.is_auto()
                        && ip_conf.addresses.is_none()
                    {
                        ip_conf.addresses = cur_ip_conf.addresses.clone();
                    }
                }
            }
        }
        if let Some(cur_ip_conf) = cur_iface.base_iface().ipv6.as_ref() {
            if cur_ip_conf.is_auto() && cur_ip_conf.addresses.is_some() {
                if let Some(ip_conf) = iface.base_iface_mut().ipv6.as_mut() {
                    if ip_conf.enabled
                        && !ip_conf.is_auto()
                        && ip_conf.addresses.is_none()
                    {
                        ip_conf.addresses = cur_ip_conf.addresses.clone();
                    }
                }
            }
        }
    }
}
