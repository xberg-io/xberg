// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, hash_map};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::network::refresh_networks_addresses;
use crate::{InterfaceOperationalState, IpNetwork, MacAddr, NetworkData};

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $name;
    }};
    ($ty_:expr, $name:ident, $old:ident, $path:expr) => {{
        let _tmp = $path;
        $ty_.$old = $ty_.$name;
        $ty_.$name = _tmp;
    }};
}

fn read<P: AsRef<Path>>(parent: P, path: &str, data: &mut [u8]) -> u64 {
    if let Ok(mut f) = File::open(parent.as_ref().join(path))
        && let Ok(size) = f.read(data)
    {
        let mut i = 0;
        let mut ret = 0;

        while i < size && i < data.len() && data[i] >= b'0' && data[i] <= b'9' {
            ret *= 10;
            ret += (data[i] - b'0') as u64;
            i += 1;
        }
        return ret;
    }
    0
}

// `read_str` clears and refills the Vec, so its length becomes the length of
// the string just read. For example, reading "up\n" leaves the Vec length at 3.
// Keep this buffer separate from numeric read buffers, otherwise counters could
// be truncated to a few bytes.
#[allow(clippy::ptr_arg)]
fn read_str<'data, P: AsRef<Path>>(parent: P, path: &str, data: &'data mut Vec<u8>) -> &'data [u8] {
    data.clear();
    if let Ok(mut f) = File::open(parent.as_ref().join(path))
        && let Ok(size) = f.read_to_end(data)
    {
        &mut data[..size]
    } else {
        b""
    }
}

impl InterfaceOperationalState {
    pub(crate) fn from_data(data: &[u8]) -> Self {
        // see /sys/class/net/<iface>/operstate section at https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-net
        match data {
            b"unknown" => InterfaceOperationalState::Unknown,
            b"notpresent" => InterfaceOperationalState::NotPresent,
            b"down" => InterfaceOperationalState::Down,
            b"lowerlayerdown" => InterfaceOperationalState::LowerLayerDown,
            b"testing" => InterfaceOperationalState::Testing,
            b"dormant" => InterfaceOperationalState::Dormant,
            b"up" => InterfaceOperationalState::Up,
            _ => InterfaceOperationalState::Other,
        }
    }
}

fn refresh_networks_list_from_sysfs(
    interfaces: &mut HashMap<String, NetworkData>,
    remove_not_listed_interfaces: bool,
    sysfs_net: &Path,
) {
    if let Ok(dir) = std::fs::read_dir(sysfs_net) {
        // ilog10 gives number of digits minus one; the extra +1 is for the newline character.
        let mut num_buf = [0u8; u64::MAX.ilog(10) as usize + 2];
        let mut str_buf = Vec::with_capacity(32);

        for stats in interfaces.values_mut() {
            stats.inner.updated = false;
        }

        for entry in dir.flatten() {
            let parent = &entry.path().join("statistics");
            let entry_path = &entry.path();
            let entry = match entry.file_name().into_string() {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let rx_bytes = read(parent, "rx_bytes", &mut num_buf);
            let tx_bytes = read(parent, "tx_bytes", &mut num_buf);
            let rx_packets = read(parent, "rx_packets", &mut num_buf);
            let tx_packets = read(parent, "tx_packets", &mut num_buf);
            let rx_errors = read(parent, "rx_errors", &mut num_buf);
            let tx_errors = read(parent, "tx_errors", &mut num_buf);
            // let rx_compressed = read(parent, "rx_compressed", &mut num_buf);
            // let tx_compressed = read(parent, "tx_compressed", &mut num_buf);
            let mtu = read(entry_path, "mtu", &mut num_buf);

            let operational_state = InterfaceOperationalState::from_data(
                read_str(entry_path, "operstate", &mut str_buf).trim_ascii(),
            );

            match interfaces.entry(entry) {
                hash_map::Entry::Occupied(mut e) => {
                    let interface = e.get_mut();
                    let interface = &mut interface.inner;

                    old_and_new!(interface, rx_bytes, old_rx_bytes);
                    old_and_new!(interface, tx_bytes, old_tx_bytes);
                    old_and_new!(interface, rx_packets, old_rx_packets);
                    old_and_new!(interface, tx_packets, old_tx_packets);
                    old_and_new!(interface, rx_errors, old_rx_errors);
                    old_and_new!(interface, tx_errors, old_tx_errors);
                    // old_and_new!(e, rx_compressed, old_rx_compressed);
                    // old_and_new!(e, tx_compressed, old_tx_compressed);
                    interface.mtu = mtu;
                    interface.operational_state = operational_state;
                    interface.updated = true;
                }
                hash_map::Entry::Vacant(e) => {
                    e.insert(NetworkData {
                        inner: NetworkDataInner {
                            rx_bytes,
                            old_rx_bytes: rx_bytes,
                            tx_bytes,
                            old_tx_bytes: tx_bytes,
                            rx_packets,
                            old_rx_packets: rx_packets,
                            tx_packets,
                            old_tx_packets: tx_packets,
                            rx_errors,
                            old_rx_errors: rx_errors,
                            tx_errors,
                            old_tx_errors: tx_errors,
                            mac_addr: MacAddr::UNSPECIFIED,
                            ip_networks: vec![],
                            // rx_compressed,
                            // old_rx_compressed: rx_compressed,
                            // tx_compressed,
                            // old_tx_compressed: tx_compressed,
                            operational_state,
                            mtu,
                            updated: true,
                        },
                    });
                }
            };
        }
    }
    // We do this here because `refresh_networks_list_remove_interface` test is checking that
    // this is working as expected.
    if remove_not_listed_interfaces {
        // Remove interfaces which are gone.
        interfaces.retain(|_, i| {
            if !i.inner.updated {
                return false;
            }
            i.inner.updated = false;
            true
        });
    }
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
        refresh_networks_list_from_sysfs(
            &mut self.interfaces,
            remove_not_listed_interfaces,
            Path::new("/sys/class/net/"),
        );
        refresh_networks_addresses(&mut self.interfaces);
    }
}

pub(crate) struct NetworkDataInner {
    /// Total number of bytes received over interface.
    rx_bytes: u64,
    old_rx_bytes: u64,
    /// Total number of bytes transmitted over interface.
    tx_bytes: u64,
    old_tx_bytes: u64,
    /// Total number of packets received.
    rx_packets: u64,
    old_rx_packets: u64,
    /// Total number of packets transmitted.
    tx_packets: u64,
    old_tx_packets: u64,
    /// Shows the total number of packets received with error. This includes
    /// too-long-frames errors, ring-buffer overflow errors, CRC errors,
    /// frame alignment errors, fifo overruns, and missed packets.
    rx_errors: u64,
    old_rx_errors: u64,
    /// similar to `rx_errors`
    tx_errors: u64,
    old_tx_errors: u64,
    /// MAC address
    pub(crate) mac_addr: MacAddr,
    pub(crate) ip_networks: Vec<IpNetwork>,
    /// Interface Maximum Transfer Unit (MTU)
    mtu: u64,
    operational_state: InterfaceOperationalState,
    // /// Indicates the number of compressed packets received by this
    // /// network device. This value might only be relevant for interfaces
    // /// that support packet compression (e.g: PPP).
    // rx_compressed: usize,
    // old_rx_compressed: usize,
    // /// Indicates the number of transmitted compressed packets. Note
    // /// this might only be relevant for devices that support
    // /// compression (e.g: PPP).
    // tx_compressed: usize,
    // old_tx_compressed: usize,
    /// Whether or not the above data has been updated during refresh
    updated: bool,
}

impl NetworkDataInner {
    pub(crate) fn received(&self) -> u64 {
        self.rx_bytes.saturating_sub(self.old_rx_bytes)
    }

    pub(crate) fn total_received(&self) -> u64 {
        self.rx_bytes
    }

    pub(crate) fn transmitted(&self) -> u64 {
        self.tx_bytes.saturating_sub(self.old_tx_bytes)
    }

    pub(crate) fn total_transmitted(&self) -> u64 {
        self.tx_bytes
    }

    pub(crate) fn packets_received(&self) -> u64 {
        self.rx_packets.saturating_sub(self.old_rx_packets)
    }

    pub(crate) fn total_packets_received(&self) -> u64 {
        self.rx_packets
    }

    pub(crate) fn packets_transmitted(&self) -> u64 {
        self.tx_packets.saturating_sub(self.old_tx_packets)
    }

    pub(crate) fn total_packets_transmitted(&self) -> u64 {
        self.tx_packets
    }

    pub(crate) fn errors_on_received(&self) -> u64 {
        self.rx_errors.saturating_sub(self.old_rx_errors)
    }

    pub(crate) fn total_errors_on_received(&self) -> u64 {
        self.rx_errors
    }

    pub(crate) fn errors_on_transmitted(&self) -> u64 {
        self.tx_errors.saturating_sub(self.old_tx_errors)
    }

    pub(crate) fn total_errors_on_transmitted(&self) -> u64 {
        self.tx_errors
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

#[cfg(test)]
mod test {
    use super::refresh_networks_list_from_sysfs;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn refresh_networks_list_add_interface() {
        let sys_net_dir = tempfile::tempdir().expect("failed to create temporary directory");

        fs::create_dir(sys_net_dir.path().join("itf1")).expect("failed to create subdirectory");

        let mut interfaces = HashMap::new();

        refresh_networks_list_from_sysfs(&mut interfaces, false, sys_net_dir.path());
        assert_eq!(interfaces.keys().collect::<Vec<_>>(), ["itf1"]);

        fs::create_dir(sys_net_dir.path().join("itf2")).expect("failed to create subdirectory");

        refresh_networks_list_from_sysfs(&mut interfaces, false, sys_net_dir.path());
        let mut itf_names: Vec<String> = interfaces.keys().map(|n| n.to_owned()).collect();
        itf_names.sort();
        assert_eq!(itf_names, ["itf1", "itf2"]);
    }

    #[test]
    fn refresh_networks_list_remove_interface() {
        let sys_net_dir = tempfile::tempdir().expect("failed to create temporary directory");

        let itf1_dir = sys_net_dir.path().join("itf1");
        let itf2_dir = sys_net_dir.path().join("itf2");
        fs::create_dir(&itf1_dir).expect("failed to create subdirectory");
        fs::create_dir(itf2_dir).expect("failed to create subdirectory");

        let mut interfaces = HashMap::new();

        refresh_networks_list_from_sysfs(&mut interfaces, false, sys_net_dir.path());
        let mut itf_names: Vec<String> = interfaces.keys().map(|n| n.to_owned()).collect();
        itf_names.sort();
        assert_eq!(itf_names, ["itf1", "itf2"]);

        fs::remove_dir(&itf1_dir).expect("failed to remove subdirectory");

        refresh_networks_list_from_sysfs(&mut interfaces, true, sys_net_dir.path());
        assert_eq!(interfaces.keys().collect::<Vec<_>>(), ["itf2"]);
    }

    #[test]
    fn refresh_networks_list_with_multiple_interfaces() {
        let dir = tempfile::tempdir().expect("failed to create temporary directory");

        for (name, rx) in [
            ("if_a", "100"),
            ("if_b", "1234567890123"),
            ("if_c", "9876543210987"),
            ("if_d", "18446744073709551615"),
        ] {
            let stats = dir.path().join(name).join("statistics");

            fs::create_dir_all(&stats).expect("failed to create statistics dir");

            for f in &[
                "rx_bytes",
                "tx_bytes",
                "rx_packets",
                "tx_packets",
                "rx_errors",
                "tx_errors",
            ] {
                fs::write(stats.join(f), rx).expect("failed to write stats");
            }

            fs::write(dir.path().join(name).join("mtu"), "1500").expect("failed to write mtu");

            fs::write(dir.path().join(name).join("operstate"), "up\n")
                .expect("failed to write operstate");
        }

        let mut interfaces = HashMap::new();

        refresh_networks_list_from_sysfs(&mut interfaces, false, dir.path());
        refresh_networks_list_from_sysfs(&mut interfaces, false, dir.path());

        assert_eq!(interfaces.get("if_a").unwrap().inner.rx_bytes, 100);
        assert_eq!(
            interfaces.get("if_b").unwrap().inner.rx_bytes,
            1_234_567_890_123
        );
        assert_eq!(
            interfaces.get("if_c").unwrap().inner.rx_bytes,
            9_876_543_210_987
        );
        assert_eq!(interfaces.get("if_d").unwrap().inner.rx_bytes, u64::MAX);

        for name in ["if_a", "if_b", "if_c", "if_d"] {
            let interface = interfaces.get(name).unwrap();
            assert_eq!(interface.inner.mtu, 1500, "{name}: mtu");
        }
    }
}
