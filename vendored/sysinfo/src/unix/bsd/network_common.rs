// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{InterfaceOperationalState, IpNetwork, MacAddr};

pub(crate) struct NetworkDataInner {
    /// Total number of bytes received over interface.
    pub(crate) ifi_ibytes: u64,
    pub(crate) old_ifi_ibytes: u64,
    /// Total number of bytes transmitted over interface.
    pub(crate) ifi_obytes: u64,
    pub(crate) old_ifi_obytes: u64,
    /// Total number of packets received.
    pub(crate) ifi_ipackets: u64,
    pub(crate) old_ifi_ipackets: u64,
    /// Total number of packets transmitted.
    pub(crate) ifi_opackets: u64,
    pub(crate) old_ifi_opackets: u64,
    /// Shows the total number of packets received with error. This includes
    /// too-long-frames errors, ring-buffer overflow errors, CRC errors,
    /// frame alignment errors, fifo overruns, and missed packets.
    pub(crate) ifi_ierrors: u64,
    pub(crate) old_ifi_ierrors: u64,
    /// similar to `ifi_ierrors`
    pub(crate) ifi_oerrors: u64,
    pub(crate) old_ifi_oerrors: u64,
    /// Whether or not the above data has been updated during refresh
    pub(crate) updated: bool,
    /// MAC address
    pub(crate) mac_addr: MacAddr,
    /// IP networks
    pub(crate) ip_networks: Vec<IpNetwork>,
    /// Interface Maximum Transfer Unit (MTU)
    pub(crate) mtu: u64,
    pub(crate) operational_state: InterfaceOperationalState,
}

impl NetworkDataInner {
    pub(crate) fn received(&self) -> u64 {
        self.ifi_ibytes.saturating_sub(self.old_ifi_ibytes)
    }

    pub(crate) fn total_received(&self) -> u64 {
        self.ifi_ibytes
    }

    pub(crate) fn transmitted(&self) -> u64 {
        self.ifi_obytes.saturating_sub(self.old_ifi_obytes)
    }

    pub(crate) fn total_transmitted(&self) -> u64 {
        self.ifi_obytes
    }

    pub(crate) fn packets_received(&self) -> u64 {
        self.ifi_ipackets.saturating_sub(self.old_ifi_ipackets)
    }

    pub(crate) fn total_packets_received(&self) -> u64 {
        self.ifi_ipackets
    }

    pub(crate) fn packets_transmitted(&self) -> u64 {
        self.ifi_opackets.saturating_sub(self.old_ifi_opackets)
    }

    pub(crate) fn total_packets_transmitted(&self) -> u64 {
        self.ifi_opackets
    }

    pub(crate) fn errors_on_received(&self) -> u64 {
        self.ifi_ierrors.saturating_sub(self.old_ifi_ierrors)
    }

    pub(crate) fn total_errors_on_received(&self) -> u64 {
        self.ifi_ierrors
    }

    pub(crate) fn errors_on_transmitted(&self) -> u64 {
        self.ifi_oerrors.saturating_sub(self.old_ifi_oerrors)
    }

    pub(crate) fn total_errors_on_transmitted(&self) -> u64 {
        self.ifi_oerrors
    }

    pub(crate) fn mac_address(&self) -> MacAddr {
        self.mac_addr
    }

    pub(crate) fn ip_networks(&self) -> &[IpNetwork] {
        &self.ip_networks
    }

    pub(crate) fn mtu(&self) -> u64 {
        self.mtu
    }

    pub(crate) fn operational_state(&self) -> InterfaceOperationalState {
        self.operational_state
    }
}

impl InterfaceOperationalState {
    pub(crate) fn from_flag(flags: core::ffi::c_int, link_state: core::ffi::c_int) -> Self {
        // https://github.com/freebsd/freebsd-src/blob/7e7d4e711ff94d114c93fd522d4125aa9bd9f5cd/contrib/bsnmp/snmp_mibII/mibII_interfaces.c#L283-L298

        if (flags & libc::IFF_RUNNING) != 0 {
            if link_state != libc::LINK_STATE_UP {
                InterfaceOperationalState::Dormant
            } else {
                InterfaceOperationalState::Up
            }
        } else {
            InterfaceOperationalState::Down
        }
    }
}
