// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the networks are not loaded by default.

#![cfg(feature = "network")]

use sysinfo::Networks;

#[test]
#[cfg(not(target_os = "ios"))]
fn test_networks() {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        return;
    }
    let mut networks = Networks::new();
    assert_eq!(networks.list().len(), 0);
    networks.refresh(false);
    assert_ne!(networks.list().len(), 0);
}

#[test]
#[cfg(not(target_os = "ios"))]
fn test_mac_addr() {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        return;
    }
    let mut networks = Networks::new();
    networks.refresh(false);
    assert_ne!(networks.list().len(), 0);
    networks
        .iter()
        .any(|(_, n)| !n.mac_address().is_unspecified());
}

#[test]
#[cfg(target_os = "ios")]
fn test_networks_remain_empty_on_ios() {
    let mut networks = Networks::new();
    assert!(networks.list().is_empty());
    networks.refresh(false);
    assert!(networks.list().is_empty());
    networks.refresh(true);
    assert!(networks.list().is_empty());
    assert!(Networks::new_with_refreshed_list().list().is_empty());
}
