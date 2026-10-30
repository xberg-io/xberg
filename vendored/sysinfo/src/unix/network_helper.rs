// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use std::os::raw::c_char;
use std::ptr::null_mut;
use std::str::from_utf8_unchecked;
use std::{io, mem};

use crate::{IpNetwork, MacAddr};

/// This iterator yields an interface name and address.
pub(crate) struct InterfaceAddress {
    /// Pointer to the first element in linked list.
    buf: *mut libc::ifaddrs,
}

impl Drop for InterfaceAddress {
    fn drop(&mut self) {
        // Safety: this struct cannot be built with a NULL `buf` field.
        unsafe {
            libc::freeifaddrs(self.buf);
        }
    }
}

impl InterfaceAddress {
    pub(crate) fn new() -> Option<Self> {
        let mut ifap = null_mut();
        if unsafe { retry_eintr!(libc::getifaddrs(&mut ifap)) } == 0 && !ifap.is_null() {
            Some(Self { buf: ifap })
        } else {
            sysinfo_debug!("`getifaddrs` failed");
            None
        }
    }

    pub(crate) fn iter(&self) -> InterfaceAddressIterator<'_> {
        InterfaceAddressIterator {
            ifap: self.buf,
            helper: InterfaceAddressHelper { ifap: self.buf },
            _phantom: PhantomData,
        }
    }
}

pub(crate) struct InterfaceAddressIterator<'a> {
    ifap: *mut libc::ifaddrs,
    helper: InterfaceAddressHelper,
    _phantom: PhantomData<&'a ()>,
}

pub(crate) struct InterfaceAddressHelper {
    ifap: *mut libc::ifaddrs,
}

impl InterfaceAddressHelper {
    pub(crate) fn name(&self) -> Option<String> {
        // Safety: We assume that addr is valid for the lifetime of this body, and is not mutated.
        let addr_ref: &libc::ifaddrs = unsafe { &*self.ifap };

        let c_str = addr_ref.ifa_name as *const c_char;

        // Safety: ifa_name is a null terminated interface name
        let bytes = unsafe { CStr::from_ptr(c_str).to_bytes() };

        // Safety: Interfaces on unix must be valid UTF-8
        let name = unsafe { from_utf8_unchecked(bytes).to_owned() };
        // Interfaces names may be formatted as <interface name>:<sub-interface index>
        if name.contains(':') {
            name.split(':').next().map(|v| v.to_string())
        } else {
            Some(name)
        }
    }

    pub(crate) fn ip(&self) -> Option<IpAddr> {
        // Safety: We assume that addr is valid for the lifetime of this body, and is not mutated.
        let addr_ref: &libc::ifaddrs = unsafe { &*self.ifap };

        sockaddr_to_network_addr(addr_ref.ifa_addr as *const libc::sockaddr)
    }

    pub(crate) fn prefix(&self) -> u8 {
        // Safety: We assume that addr is valid for the lifetime of this body, and is not mutated.
        let addr_ref: &libc::ifaddrs = unsafe { &*self.ifap };
        let netmask = sockaddr_to_network_addr(addr_ref.ifa_netmask as *const libc::sockaddr);
        netmask
            .and_then(|netmask| ip_mask_to_prefix(netmask).ok())
            .unwrap_or(0)
    }

    pub(crate) fn mac_addr(&self) -> Option<MacAddr> {
        // Safety: We assume that addr is valid for the lifetime of this body, and is not mutated.
        let addr_ref: &libc::ifaddrs = unsafe { &*self.ifap };
        unsafe { parse_interface_address(addr_ref) }
    }
}

impl<'a> Iterator for InterfaceAddressIterator<'a> {
    type Item = &'a InterfaceAddressHelper;

    fn next(&mut self) -> Option<Self::Item> {
        // Advance the pointer until a MAC address is found
        if self.ifap.is_null() {
            return None;
        }
        unsafe {
            // Safety: `ifap` is already checked as non-null in the above `if` condition.
            let ifap = self.ifap;
            self.ifap = (*ifap).ifa_next;

            self.helper.ifap = ifap;
            // Safety: The returned reference will always match `Self`'s so should not be an issue.
            Some(std::mem::transmute::<&InterfaceAddressHelper, Self::Item>(
                &self.helper,
            ))
        }
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios"
))]
impl From<&libc::sockaddr_dl> for MacAddr {
    fn from(value: &libc::sockaddr_dl) -> Self {
        let sdl_data = value.sdl_data;
        // interface name length, NO trailing 0
        let sdl_nlen = value.sdl_nlen as usize;
        // make sure that it is never out of bound
        if sdl_nlen + 5 < 12 {
            MacAddr([
                sdl_data[sdl_nlen] as u8,
                sdl_data[sdl_nlen + 1] as u8,
                sdl_data[sdl_nlen + 2] as u8,
                sdl_data[sdl_nlen + 3] as u8,
                sdl_data[sdl_nlen + 4] as u8,
                sdl_data[sdl_nlen + 5] as u8,
            ])
        } else {
            MacAddr::UNSPECIFIED
        }
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios"
))]
unsafe fn parse_interface_address(ifap: &libc::ifaddrs) -> Option<MacAddr> {
    let sock_addr = ifap.ifa_addr;
    if sock_addr.is_null() {
        return None;
    }
    unsafe {
        match (*sock_addr).sa_family as libc::c_int {
            libc::AF_LINK => {
                let addr = sock_addr as *const libc::sockaddr_dl;
                Some(MacAddr::from(&*addr))
            }
            _ => None,
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn parse_interface_address(ifap: &libc::ifaddrs) -> Option<MacAddr> {
    use libc::sockaddr_ll;

    let sock_addr = ifap.ifa_addr;
    if sock_addr.is_null() {
        return None;
    }
    unsafe {
        match (*sock_addr).sa_family as libc::c_int {
            libc::AF_PACKET => {
                let addr = sock_addr as *const sockaddr_ll;
                // Take the first 6 bytes
                let [addr @ .., _, _] = (*addr).sll_addr;
                Some(MacAddr(addr))
            }
            _ => None,
        }
    }
}

pub(crate) unsafe fn refresh_network_interfaces(
    interfaces: &mut HashMap<String, crate::NetworkData>,
) {
    let Some(interface_iter) = InterfaceAddress::new() else {
        return;
    };
    let mut ifaces: HashMap<String, HashSet<IpNetwork>> = HashMap::new();

    for entry in interface_iter.iter() {
        if let Some(interface_name) = entry.name()
            // We only do work if the existing interfaces contain this one.
            && let Some(interface) = interfaces.get_mut(&interface_name)
        {
            if let Some(mac_addr) = entry.mac_addr() {
                interface.inner.mac_addr = mac_addr;
            }
            if let Some(ip) = entry.ip() {
                let prefix = entry.prefix();
                ifaces
                    .entry(interface_name)
                    .and_modify(|values| {
                        values.insert(IpNetwork { addr: ip, prefix });
                    })
                    .or_insert(HashSet::from([IpNetwork { addr: ip, prefix }]));
            }
        }
    }

    for (interface_name, ip_networks) in ifaces {
        if let Some(interface) = interfaces.get_mut(&interface_name) {
            interface.inner.ip_networks = ip_networks.into_iter().collect::<Vec<_>>();
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn sockaddr_to_network_addr(sa: *const libc::sockaddr) -> Option<IpAddr> {
    unsafe {
        if sa.is_null() || (*sa).sa_family as libc::c_int == libc::AF_PACKET {
            None
        } else {
            let addr = sockaddr_to_addr(
                &(sa as *const libc::sockaddr_storage).read_unaligned(),
                mem::size_of::<libc::sockaddr_storage>(),
            );

            match addr {
                Ok(SocketAddr::V4(sa)) => Some(IpAddr::V4(*sa.ip())),
                Ok(SocketAddr::V6(sa)) => Some(IpAddr::V6(*sa.ip())),
                _ => None,
            }
        }
    }
}
#[cfg(not(any(target_os = "illumos", target_os = "solaris")))]
pub type InAddrType = libc::c_uint;
#[cfg(any(target_os = "illumos", target_os = "solaris"))]
pub type InAddrType = libc::c_ulonglong;

fn sockaddr_to_addr(storage: &libc::sockaddr_storage, len: usize) -> io::Result<SocketAddr> {
    match storage.ss_family as libc::c_int {
        libc::AF_INET => {
            assert!(len >= mem::size_of::<libc::sockaddr_in>());
            let storage: &libc::sockaddr_in = unsafe { mem::transmute(storage) };
            let ip = (storage.sin_addr.s_addr as InAddrType).to_be();
            let a = (ip >> 24) as u8;
            let b = (ip >> 16) as u8;
            let c = (ip >> 8) as u8;
            let d = ip as u8;
            let sockaddrv4 = SocketAddrV4::new(Ipv4Addr::new(a, b, c, d), storage.sin_port.to_be());
            Ok(SocketAddr::V4(sockaddrv4))
        }
        libc::AF_INET6 => {
            assert!(len >= mem::size_of::<libc::sockaddr_in6>());
            let storage: &libc::sockaddr_in6 = unsafe { mem::transmute(storage) };
            let arr: [u16; 8] = unsafe { mem::transmute(storage.sin6_addr.s6_addr) };
            let ip = Ipv6Addr::new(
                arr[0].to_be(),
                arr[1].to_be(),
                arr[2].to_be(),
                arr[3].to_be(),
                arr[4].to_be(),
                arr[5].to_be(),
                arr[6].to_be(),
                arr[7].to_be(),
            );
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip,
                storage.sin6_port.to_be(),
                u32::from_be(storage.sin6_flowinfo),
                storage.sin6_scope_id,
            )))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected IPv4 or IPv6 socket",
        )),
    }
}

#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "macos",
    target_os = "ios"
))]
fn sockaddr_to_network_addr(sa: *const libc::sockaddr) -> Option<IpAddr> {
    unsafe {
        if sa.is_null() || (*sa).sa_family as libc::c_int == 18 {
            None
        } else {
            let addr = sockaddr_to_addr(
                &(sa as *const libc::sockaddr_storage).read_unaligned(),
                mem::size_of::<libc::sockaddr_storage>(),
            );

            match addr {
                Ok(SocketAddr::V4(sa)) => Some(IpAddr::V4(*sa.ip())),
                Ok(SocketAddr::V6(sa)) => Some(IpAddr::V6(*sa.ip())),
                _ => None,
            }
        }
    }
}

pub(crate) fn ip_mask_to_prefix(mask: IpAddr) -> Result<u8, &'static str> {
    match mask {
        IpAddr::V4(mask) => ipv4_mask_to_prefix(mask),
        IpAddr::V6(mask) => ipv6_mask_to_prefix(mask),
    }
}

pub(crate) fn ipv4_mask_to_prefix(mask: Ipv4Addr) -> Result<u8, &'static str> {
    let mask = u32::from(mask);

    let prefix = (!mask).leading_zeros() as u8;
    if (u64::from(mask) << prefix) & 0xffff_ffff != 0 {
        Err("invalid ipv4 prefix")
    } else {
        Ok(prefix)
    }
}

pub(crate) fn ipv6_mask_to_prefix(mask: Ipv6Addr) -> Result<u8, &'static str> {
    let mask = mask.segments();
    let mut mask_iter = mask.iter();

    // Count the number of set bits from the start of the address
    let mut prefix = 0;
    for &segment in &mut mask_iter {
        if segment == 0xffff {
            prefix += 16;
        } else if segment == 0 {
            // Prefix finishes on a segment boundary
            break;
        } else {
            let prefix_bits = (!segment).leading_zeros() as u8;
            // Check that the remainder of the bits are all unset
            if segment << prefix_bits != 0 {
                return Err("invalid ipv6 prefix");
            }
            prefix += prefix_bits;
            break;
        }
    }

    // Now check all the remaining bits are unset
    for &segment in mask_iter {
        if segment != 0 {
            return Err("invalid ipv6 prefix");
        }
    }

    Ok(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_mask() {
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        let prefix = ipv4_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 24);
    }

    #[test]
    fn ipv4_mask_another() {
        let mask = Ipv4Addr::new(255, 255, 255, 128);
        let prefix = ipv4_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 25);
    }

    #[test]
    fn v4_mask_to_prefix_invalid() {
        let mask = Ipv4Addr::new(255, 128, 255, 0);
        assert!(ipv4_mask_to_prefix(mask).is_err());
    }

    #[test]
    fn ipv6_mask() {
        let mask = Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0, 0, 0, 0, 0);
        let prefix = ipv6_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 48);
    }

    #[test]
    fn ipv6_mask_invalid() {
        let mask = Ipv6Addr::new(0, 0xffff, 0xffff, 0, 0, 0, 0, 0);
        assert!(ipv6_mask_to_prefix(mask).is_err());
    }

    #[test]
    fn ip_mask_enum_ipv4() {
        let mask = IpAddr::from(Ipv4Addr::new(255, 255, 255, 0));
        let prefix = ip_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 24);
    }

    #[test]
    fn ip_mask_enum_ipv4_invalid() {
        let mask = IpAddr::from(Ipv4Addr::new(255, 0, 255, 0));
        assert!(ip_mask_to_prefix(mask).is_err());
    }

    #[test]
    fn ip_mask_enum_ipv6() {
        let mask = IpAddr::from(Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0, 0, 0, 0, 0));
        let prefix = ip_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 48);
    }

    #[test]
    fn ip_mask_enum_ipv6_invalid() {
        let mask = IpAddr::from(Ipv6Addr::new(0xffff, 0xffff, 0xff00, 0xffff, 0, 0, 0, 0));
        assert!(ip_mask_to_prefix(mask).is_err());
    }
}
