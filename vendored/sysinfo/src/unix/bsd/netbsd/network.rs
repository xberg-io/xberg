// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, hash_map};

use crate::network::refresh_networks_addresses;
use crate::unix::bsd::NetworkDataInner;
use crate::{InterfaceOperationalState, MacAddr, NetworkData};

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident, $data:expr) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $data.$name;
    }};
}

pub(crate) struct NetworksInner {
    pub(crate) interfaces: HashMap<String, NetworkData>,
}

impl NetworksInner {
    pub(crate) fn new() -> Self {
        Self {
            interfaces: HashMap::new(),
        }
    }

    pub(crate) fn list(&self) -> &HashMap<String, NetworkData> {
        &self.interfaces
    }

    pub(crate) fn refresh(&mut self, remove_not_listed_interfaces: bool) {
        unsafe {
            self.refresh_interfaces(true);
        }
        if remove_not_listed_interfaces {
            // Remove interfaces which are gone.
            self.interfaces.retain(|_, i| {
                if !i.inner.updated {
                    return false;
                }
                i.inner.updated = false;
                true
            });
        }
        // FIXME: Try to find a way to call `getifaddrs` only once. Currently it's called in
        // `Self::refresh_interfaces` and in `refresh_networks_addresses`.
        refresh_networks_addresses(&mut self.interfaces);
    }

    unsafe fn refresh_interfaces(&mut self, refresh_all: bool) {
        unsafe {
            let Some(ifaddrs) = InterfaceAddressIterator::new() else {
                sysinfo_debug!("getifaddrs failed");
                return;
            };

            for ifa in ifaddrs {
                let ifa = &*ifa;
                if let Some(name) = std::ffi::CStr::from_ptr(ifa.ifa_name)
                    .to_str()
                    .ok()
                    .map(|s| s.to_string())
                {
                    let flags = ifa.ifa_flags;
                    let data: &libc::if_data = &*(ifa.ifa_data as *mut libc::if_data);
                    let mtu = data.ifi_mtu;
                    let operational_state = InterfaceOperationalState::from_flag(
                        flags as core::ffi::c_int,
                        data.ifi_link_state,
                    );
                    match self.interfaces.entry(name) {
                        hash_map::Entry::Occupied(mut e) => {
                            let interface = e.get_mut();
                            let interface = &mut interface.inner;

                            old_and_new!(interface, ifi_ibytes, old_ifi_ibytes, data);
                            old_and_new!(interface, ifi_obytes, old_ifi_obytes, data);
                            old_and_new!(interface, ifi_ipackets, old_ifi_ipackets, data);
                            old_and_new!(interface, ifi_opackets, old_ifi_opackets, data);
                            old_and_new!(interface, ifi_ierrors, old_ifi_ierrors, data);
                            old_and_new!(interface, ifi_oerrors, old_ifi_oerrors, data);
                            interface.mtu = mtu;
                            interface.operational_state = operational_state;
                            interface.updated = true;
                        }
                        hash_map::Entry::Vacant(e) => {
                            if !refresh_all {
                                // This is simply a refresh, we don't want to add new interfaces!
                                continue;
                            }
                            e.insert(NetworkData {
                                inner: NetworkDataInner {
                                    ifi_ibytes: data.ifi_ibytes,
                                    old_ifi_ibytes: 0,
                                    ifi_obytes: data.ifi_obytes,
                                    old_ifi_obytes: 0,
                                    ifi_ipackets: data.ifi_ipackets,
                                    old_ifi_ipackets: 0,
                                    ifi_opackets: data.ifi_opackets,
                                    old_ifi_opackets: 0,
                                    ifi_ierrors: data.ifi_ierrors,
                                    old_ifi_ierrors: 0,
                                    ifi_oerrors: data.ifi_oerrors,
                                    old_ifi_oerrors: 0,
                                    updated: true,
                                    mac_addr: MacAddr::UNSPECIFIED,
                                    ip_networks: vec![],
                                    mtu,
                                    operational_state,
                                },
                            });
                        }
                    }
                }
            }
        }
    }
}

struct InterfaceAddressIterator {
    /// Pointer to the current `ifaddrs` struct.
    ifap: *mut libc::ifaddrs,
    /// Pointer to the first element in linked list.
    buf: *mut libc::ifaddrs,
}

impl InterfaceAddressIterator {
    fn new() -> Option<Self> {
        let mut ifap = std::ptr::null_mut();
        if unsafe { retry_eintr!(libc::getifaddrs(&mut ifap)) } == 0 && !ifap.is_null() {
            Some(Self { ifap, buf: ifap })
        } else {
            None
        }
    }
}

impl Iterator for InterfaceAddressIterator {
    type Item = *mut libc::ifaddrs;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while !self.ifap.is_null() {
                // advance the pointer until a MAC address is found
                // Safety: `ifap` is already checked as non-null in the loop condition.
                let ifap = self.ifap;
                let r_ifap = &*ifap;
                self.ifap = r_ifap.ifa_next;

                if r_ifap.ifa_addr.is_null()
                    || (*r_ifap.ifa_addr).sa_family as libc::c_int != libc::AF_LINK
                    || r_ifap.ifa_flags & libc::IFF_LOOPBACK as libc::c_uint != 0
                {
                    continue;
                }
                return Some(ifap);
            }
            None
        }
    }
}

impl Drop for InterfaceAddressIterator {
    fn drop(&mut self) {
        unsafe {
            libc::freeifaddrs(self.buf);
        }
    }
}
