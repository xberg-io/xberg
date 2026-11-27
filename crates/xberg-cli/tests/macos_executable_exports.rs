//! The macOS `xberg` executable exports no symbols (`build.rs` links it with
//! `-no_exported_symbols`), so dyld does no weak-definition coalescing for it at start.
#![cfg(target_os = "macos")]

use std::path::Path;

const MH_MAGIC_64: u32 = 0xfeed_facf;
const MH_WEAK_DEFINES: u32 = 0x8000;
const LC_DYLD_EXPORTS_TRIE: u32 = 0x8000_0033;
const LC_DYLD_INFO_ONLY: u32 = 0x8000_0022;
const MACH_HEADER_64_SIZE: usize = 32;

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    let field: [u8; 4] = bytes[offset..offset + 4].try_into().expect("four bytes");
    u32::from_le_bytes(field)
}

/// Header flags and the byte size of the export trie of a thin 64-bit little-endian Mach-O.
fn flags_and_export_trie_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path).expect("read the xberg executable");
    assert_eq!(
        read_u32(&bytes, 0),
        MH_MAGIC_64,
        "{} is not a thin 64-bit Mach-O",
        path.display()
    );
    let command_count = read_u32(&bytes, 16) as usize;
    let flags = read_u32(&bytes, 24);
    let mut offset = MACH_HEADER_64_SIZE;
    let mut export_trie_size = 0;
    for _ in 0..command_count {
        let command = read_u32(&bytes, offset);
        let size = read_u32(&bytes, offset + 4) as usize;
        match command {
            LC_DYLD_EXPORTS_TRIE => export_trie_size += read_u32(&bytes, offset + 12),
            LC_DYLD_INFO_ONLY => export_trie_size += read_u32(&bytes, offset + 44),
            _ => {}
        }
        offset += size;
    }
    (flags, export_trie_size)
}

#[test]
fn macos_executable_has_no_weak_definitions_to_coalesce() {
    let (flags, _) = flags_and_export_trie_size(Path::new(env!("CARGO_BIN_EXE_xberg")));
    assert_eq!(
        flags & MH_WEAK_DEFINES,
        0,
        "xberg exports weak definitions; flags = {flags:#x}"
    );
}

#[test]
fn macos_executable_exports_no_symbols() {
    let (_, export_trie_size) = flags_and_export_trie_size(Path::new(env!("CARGO_BIN_EXE_xberg")));
    // An empty trie is a single terminal node, padded to pointer alignment. ~keep
    assert!(
        export_trie_size <= 8,
        "xberg exports symbols: export trie is {export_trie_size} bytes"
    );
}
