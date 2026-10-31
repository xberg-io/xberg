// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

use libc::c_char;

use super::ffi;
use super::utils::c_buf_to_utf8_str;
use crate::{Disk, DiskKind, DiskRefreshKind, DiskUsage};

#[derive(Debug)]
pub(crate) struct DiskInner {
    name: OsString,
    c_mount_point: Vec<c_char>,
    dev_id: Vec<c_char>,
    mount_point: PathBuf,
    total_space: u64,
    available_space: u64,
    pub(crate) file_system: OsString,
    is_removable: bool,
    is_read_only: bool,
    read_bytes: u64,
    old_read_bytes: u64,
    written_bytes: u64,
    old_written_bytes: u64,
    updated: bool,
}

#[cfg(test)]
impl Default for DiskInner {
    fn default() -> Self {
        Self {
            name: OsString::new(),
            c_mount_point: Vec::new(),
            dev_id: Vec::new(),
            mount_point: PathBuf::new(),
            total_space: 0,
            available_space: 0,
            file_system: OsString::new(),
            is_removable: false,
            is_read_only: false,
            read_bytes: 0,
            old_read_bytes: 0,
            written_bytes: 0,
            old_written_bytes: 0,
            updated: false,
        }
    }
}

impl DiskInner {
    pub(crate) fn kind(&self) -> DiskKind {
        // Currently don't know how to retrieve this information on NetBSD.
        DiskKind::Unknown(-1)
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(crate) fn file_system(&self) -> &OsStr {
        &self.file_system
    }

    pub(crate) fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    pub(crate) fn total_space(&self) -> u64 {
        self.total_space
    }

    pub(crate) fn available_space(&self) -> u64 {
        self.available_space
    }

    pub(crate) fn is_removable(&self) -> bool {
        self.is_removable
    }

    pub(crate) fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    pub(crate) fn refresh_specifics(&mut self, refresh_kind: DiskRefreshKind) -> bool {
        refresh_disk(self, refresh_kind)
    }

    pub(crate) fn usage(&self) -> DiskUsage {
        DiskUsage {
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
        }
    }

    fn update_old(&mut self) {
        self.old_read_bytes = self.read_bytes;
        self.old_written_bytes = self.written_bytes;
    }
}

impl crate::DisksInner {
    pub(crate) fn new() -> Self {
        Self {
            disks: Vec::with_capacity(2),
        }
    }

    pub(crate) fn refresh_specifics(
        &mut self,
        remove_not_listed_disks: bool,
        refresh_kind: DiskRefreshKind,
    ) {
        unsafe { get_all_list(&mut self.disks, remove_not_listed_disks, refresh_kind) }
    }

    pub(crate) fn list(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }
}

/// Returns `(total_space, available_space, is_read_only)`.
unsafe fn get_statvfs(
    c_mount_point: &[c_char],
    vfs: &mut libc::statvfs,
) -> Option<(u64, u64, bool)> {
    if unsafe { libc::statvfs(c_mount_point.as_ptr() as *const _, vfs as *mut _) < 0 } {
        sysinfo_debug!("statvfs failed");
        None
    } else {
        let block_size: u64 = vfs.f_frsize as _;
        Some((
            vfs.f_blocks.saturating_mul(block_size),
            vfs.f_favail.saturating_mul(block_size),
            (vfs.f_flag & libc::ST_RDONLY) != 0,
        ))
    }
}

fn refresh_disk(disk: &mut DiskInner, refresh_kind: DiskRefreshKind) -> bool {
    if refresh_kind.storage() {
        unsafe {
            let mut vfs: libc::statvfs = std::mem::zeroed();
            if let Some((total_space, available_space, is_read_only)) =
                get_statvfs(&disk.c_mount_point, &mut vfs)
            {
                disk.total_space = total_space;
                disk.available_space = available_space;
                disk.is_read_only = is_read_only;
            }
        }
    }

    if refresh_kind.io_usage() {
        unsafe {
            refresh_disk_io(&mut [disk]);
        }
    }

    true
}

trait GetValues {
    fn update_old(&mut self);
    fn get_read(&mut self) -> &mut u64;
    fn get_written(&mut self) -> &mut u64;
    fn dev_id(&self) -> &[c_char];
}

impl GetValues for crate::Disk {
    fn update_old(&mut self) {
        self.inner.update_old()
    }
    fn get_read(&mut self) -> &mut u64 {
        self.inner.get_read()
    }
    fn get_written(&mut self) -> &mut u64 {
        self.inner.get_written()
    }
    fn dev_id(&self) -> &[c_char] {
        self.inner.dev_id()
    }
}

impl GetValues for &mut DiskInner {
    fn update_old(&mut self) {
        self.old_read_bytes = self.read_bytes;
        self.old_written_bytes = self.written_bytes;
    }
    fn get_read(&mut self) -> &mut u64 {
        &mut self.read_bytes
    }
    fn get_written(&mut self) -> &mut u64 {
        &mut self.written_bytes
    }
    fn dev_id(&self) -> &[c_char] {
        self.dev_id.as_slice()
    }
}
impl GetValues for DiskInner {
    fn update_old(&mut self) {
        self.old_read_bytes = self.read_bytes;
        self.old_written_bytes = self.written_bytes;
    }
    fn get_read(&mut self) -> &mut u64 {
        &mut self.read_bytes
    }
    fn get_written(&mut self) -> &mut u64 {
        &mut self.written_bytes
    }
    fn dev_id(&self) -> &[c_char] {
        self.dev_id.as_slice()
    }
}

fn same_name(dev_id: &[c_char], drive_name: &[c_char]) -> bool {
    // NetBSD iostats returns device names without partition letters (e.g., "ld0")
    // but mount points use full device names with partitions (e.g., "ld0a").
    // We need to match by checking if dev_id starts with drive_name.

    for (c1, c2) in dev_id.iter().zip(drive_name.iter()) {
        // Stop comparison at null terminator in drive_name
        if *c2 == 0 {
            return true;
        }
        if c1 != c2 {
            return false;
        }
    }
    true
}

unsafe fn refresh_disk_io<T: GetValues>(disks: &mut [T]) {
    const IO_SYSCTL_SIZE: usize = std::mem::size_of::<ffi::io_sysctl>();
    let mib = &[ffi::CTL_HW, ffi::HW_IOSTATS, IO_SYSCTL_SIZE as libc::c_int];
    let mut size: libc::size_t = 0;

    unsafe {
        if !super::utils::get_sysctl_raw(mib, null_mut(), &mut size) {
            sysinfo_debug!("failed to get iostats info");
            return;
        }
        let mut nb_drives = size as usize / IO_SYSCTL_SIZE;
        if nb_drives < 1 {
            sysinfo_debug!("no drive");
            return;
        }
        let mut drives: Vec<ffi::io_sysctl> = Vec::with_capacity(nb_drives);
        if !super::utils::get_sysctl_raw(mib, drives.as_mut_ptr() as *mut _, &mut size) {
            sysinfo_debug!("failed to get iostats drives");
            return;
        }
        nb_drives = size as usize / IO_SYSCTL_SIZE;
        drives.set_len(nb_drives);

        for drive in drives {
            if let Some(disk) = disks
                .iter_mut()
                .find(|disk| same_name(disk.dev_id(), &drive.name))
            {
                disk.update_old();
                *disk.get_read() = drive.rbytes;
                *disk.get_written() = drive.wbytes;
            }
        }
    }
}

fn get_dev_id(path: &[c_char]) -> Vec<c_char> {
    let mut start = None;
    let mut end = None;
    for (pos, c) in path.iter().enumerate().rev() {
        if *c == b'/' as _ {
            start = Some(pos + 1);
            break;
        } else if end.is_none() && *c != 0 {
            end = Some(pos);
        }
    }
    let start = start.unwrap_or(0);
    match end {
        Some(end) => path[start..=end].to_vec(),
        None => path[start..].to_vec(),
    }
}

pub unsafe fn get_all_list(
    container: &mut Vec<Disk>,
    remove_not_listed_disks: bool,
    refresh_kind: DiskRefreshKind,
) {
    let mut mnt_buf: *mut libc::statvfs = null_mut();

    let count = unsafe { ffi::getmntinfo(&mut mnt_buf, libc::MNT_WAIT) };
    if count < 1 {
        return;
    }

    let mnt_buf: &[libc::statvfs] = unsafe { std::slice::from_raw_parts(mnt_buf as _, count as _) };

    for fs_info in mnt_buf {
        // Ignored disk.
        if (fs_info.f_flag & libc::MNT_IGNORE as u64) != 0 {
            continue;
        }
        // Non-local disk.
        if (fs_info.f_flag & libc::MNT_LOCAL as u64) == 0 {
            continue;
        }
        if fs_info.f_mntfromname[0] == 0 || fs_info.f_mntonname[0] == 0 {
            // If we have missing information, no need to look any further...
            continue;
        }

        let fs_type: Vec<u8> = {
            let len = fs_info
                .f_fstypename
                .iter()
                .position(|x| *x == 0)
                .unwrap_or(fs_info.f_fstypename.len());
            fs_info.f_fstypename[..len]
                .iter()
                .map(|c| *c as u8)
                .collect()
        };
        match &fs_type[..] {
            b"autofs" | b"devfs" | b"linprocfs" | b"procfs" | b"fdesckfs" | b"tmpfs"
            | b"linsysfs" | b"kernfs" | b"ptyfs" => {
                sysinfo_debug!(
                    "Memory filesystem `{:?}`, ignoring it.",
                    c_buf_to_utf8_str(&fs_info.f_fstypename),
                );
                continue;
            }
            _ => {}
        }

        let mount_point = match c_buf_to_utf8_str(&fs_info.f_mntonname) {
            Some(m) => m,
            None => {
                sysinfo_debug!("Cannot get disk mount point, ignoring it.");
                continue;
            }
        };

        if mount_point == "/boot/efi" {
            continue;
        }

        // Extract device name from f_mntfromname (e.g., "/dev/ld0a")
        let name = match c_buf_to_utf8_str(&fs_info.f_mntfromname) {
            Some(device_name) => OsString::from(device_name),
            None => {
                sysinfo_debug!("Cannot get disk device name, ignoring it.");
                continue;
            }
        };

        if let Some(disk) = container.iter_mut().find(|d| {
            d.inner.name == name
                && d.inner
                    .file_system
                    .as_encoded_bytes()
                    .iter()
                    .zip(fs_type.iter())
                    .all(|(a, b)| a == b)
        }) {
            // I/O usage is updated for all disks at once at the end.
            refresh_disk(&mut disk.inner, refresh_kind.without_io_usage());
            disk.inner.updated = true;
        } else {
            // USB keys and CDs are removable.
            let is_removable = if refresh_kind.storage() {
                [b"USB", b"usb"].iter().any(|b| *b == &fs_type[..])
                    || fs_type.starts_with(b"/dev/cd")
            } else {
                false
            };

            let mut disk = DiskInner {
                name,
                c_mount_point: fs_info.f_mntonname.to_vec(),
                dev_id: get_dev_id(&fs_info.f_mntfromname),
                mount_point: PathBuf::from(mount_point),
                total_space: 0,
                available_space: 0,
                file_system: OsString::from_vec(fs_type),
                is_removable,
                is_read_only: false,
                read_bytes: 0,
                old_read_bytes: 0,
                written_bytes: 0,
                old_written_bytes: 0,
                updated: true,
            };
            // I/O usage is updated for all disks at once at the end.
            refresh_disk(&mut disk, refresh_kind.without_io_usage());
            container.push(Disk { inner: disk });
        }
    }

    if remove_not_listed_disks {
        container.retain_mut(|disk| {
            if !disk.inner.updated {
                return false;
            }
            disk.inner.updated = false;
            true
        });
    } else {
        for c in container.iter_mut() {
            c.inner.updated = false;
        }
    }

    if refresh_kind.io_usage() {
        unsafe {
            refresh_disk_io(container);
        }
    }
}
