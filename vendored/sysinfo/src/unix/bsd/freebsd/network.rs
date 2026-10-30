// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, hash_map};
use std::mem::MaybeUninit;

use super::utils;
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
        refresh_networks_addresses(&mut self.interfaces);
    }

    unsafe fn refresh_interfaces(&mut self, refresh_all: bool) {
        let mut nb_interfaces: libc::c_int = 0;
        if unsafe {
            !utils::get_sys_value(
                &[
                    libc::CTL_NET,
                    libc::PF_LINK,
                    libc::NETLINK_GENERIC,
                    libc::IFMIB_SYSTEM,
                    libc::IFMIB_IFCOUNT,
                ],
                &mut nb_interfaces,
            )
        } {
            return;
        }
        if refresh_all {
            // We don't need to update this value if we're not updating all interfaces.
            for interface in self.interfaces.values_mut() {
                interface.inner.updated = false;
            }
        }
        let mut data: libc::ifmibdata = unsafe { MaybeUninit::zeroed().assume_init() };
        for row in 1..=nb_interfaces {
            let mib = [
                libc::CTL_NET,
                libc::PF_LINK,
                libc::NETLINK_GENERIC,
                libc::IFMIB_IFDATA,
                row,
                libc::IFDATA_GENERAL,
            ];

            if unsafe { !utils::get_sys_value(&mib, &mut data) } {
                continue;
            }
            if let Some(name) = utils::c_buf_to_utf8_string(&data.ifmd_name) {
                let flags = data.ifmd_flags;
                let data = &data.ifmd_data;
                let mtu = data.ifi_mtu as u64;
                let operational_state =
                    InterfaceOperationalState::from_flag(flags, data.ifi_link_state.into());

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
