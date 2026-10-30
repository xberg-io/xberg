// Take a look at the license at the top of the repository in the LICENSE file.

use crate::unix::utils::realpath;
use crate::{DiskUsage, Gid, Pid, Process, ProcessRefreshKind, ProcessStatus, Uid};

use std::ffi::OsString;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

use super::ffi;
use super::utils::{WrapMap, c_buf_to_os_string, from_cstr_array, get_sys_value_osstr};

pub(crate) struct ProcessInner {
    pub(crate) name: OsString,
    pub(crate) cmd: Vec<OsString>,
    pub(crate) exe: Option<PathBuf>,
    pub(crate) pid: Pid,
    pub(crate) parent: Option<Pid>,
    pub(crate) environ: Vec<OsString>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) root: Option<PathBuf>,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    pub(crate) updated: bool,
    pub(crate) cpu_usage: f32,
    pub(crate) start_time: u64,
    pub(crate) run_time: u64,
    pub(crate) status: ProcessStatus,
    pub(crate) user_id: Uid,
    pub(crate) effective_user_id: Uid,
    pub(crate) group_id: Gid,
    pub(crate) effective_group_id: Gid,
    pub(crate) read_bytes: u64,
    pub(crate) old_read_bytes: u64,
    pub(crate) written_bytes: u64,
    pub(crate) old_written_bytes: u64,
    pub(crate) accumulated_cpu_time: u64,
    pub(crate) exists: bool,
}

impl ProcessInner {
    // Other methods are defined in `../system.rs`.

    pub(crate) fn disk_usage(&self) -> DiskUsage {
        DiskUsage {
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
        }
    }

    pub(crate) fn open_files(&self) -> Option<usize> {
        let open_files_dir = format!("/proc/{}/fd", self.pid);
        match read_dir(&open_files_dir) {
            Ok(entries) => Some(entries.count() as _),
            Err(_error) => {
                sysinfo_debug!("Failed to get open files in `{open_files_dir}`: {_error:?}");
                None
            }
        }
    }

    pub(crate) fn open_files_limit(&self) -> Option<usize> {
        crate::System::open_files_limit()
    }
}

#[inline]
fn get_accumulated_cpu_time(kproc: &libc::kinfo_proc2) -> u64 {
    // from htop source code
    100 * (kproc.p_rtime_sec as u64 + ((kproc.p_rtime_usec as u64 + 500_000) / 1_000_000))
}

fn get_active_status(kd: NonNull<ffi::kvm_t>, kproc: &libc::kinfo_proc2) -> Option<ProcessStatus> {
    let mut nlwps = 0;
    unsafe {
        let klwps = ffi::kvm_getlwps(
            kd.as_ptr(),
            kproc.p_pid,
            kproc.p_paddr,
            std::mem::size_of::<libc::kinfo_lwp>(),
            &mut nlwps,
        );
        if klwps.is_null() || nlwps < 1 {
            return None;
        }
        let klwps: &[libc::kinfo_lwp] = std::slice::from_raw_parts(klwps, nlwps as _);
        for entry in klwps {
            match entry.l_stat {
                ffi::LSONPROC => return Some(ProcessStatus::Run),
                ffi::LSRUN => return Some(ProcessStatus::Suspended),
                ffi::LSSLEEP => return Some(ProcessStatus::Sleep),
                ffi::LSSTOP => return Some(ProcessStatus::Stop),
                _ => {}
            }
        }
    }
    None
}

fn update_proc_info(
    kproc: &libc::kinfo_proc2,
    system_info: &super::system::SystemInfo,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    proc_: &mut ProcessInner,
) {
    proc_.run_time = now.saturating_sub(proc_.start_time);

    if refresh_kind.cwd().needs_update(|| proc_.cwd.is_none()) {
        unsafe {
            proc_.cwd = get_path(proc_.pid, ffi::KERN_PROC_CWD);
        }
    }

    if refresh_kind.exe().needs_update(|| proc_.exe.is_none()) {
        unsafe {
            proc_.exe = get_path(proc_.pid, ffi::KERN_PROC_PATHNAME);
        }
    }

    if refresh_kind.disk_usage() {
        proc_.old_read_bytes = proc_.read_bytes;
        proc_.read_bytes = kproc.p_uru_inblock.saturating_mul(system_info.block_size);
        proc_.old_written_bytes = proc_.written_bytes;
        proc_.written_bytes = kproc.p_uru_oublock.saturating_mul(system_info.block_size);
    }

    if refresh_kind.cpu() {
        proc_.accumulated_cpu_time = get_accumulated_cpu_time(kproc);
    }

    let cmd_needs_update = refresh_kind.cmd().needs_update(|| proc_.cmd.is_empty());
    if proc_.name.is_empty() || cmd_needs_update {
        let cmd = match system_info.kd {
            Some(kd) => unsafe { from_cstr_array(ffi::kvm_getargv2(kd.as_ptr(), kproc, 0) as _) },
            None => Vec::new(),
        };

        if !cmd.is_empty() {
            // First, we try to retrieve the name from the command line.
            let p = Path::new(&cmd[0]);
            if let Some(name) = p.file_name() {
                name.clone_into(&mut proc_.name);
            }

            if cmd_needs_update {
                proc_.cmd = cmd;
            }
        }
        if proc_.name.is_empty() {
            // The name can be cut short because the `ki_comm` field size is limited,
            // which is why we prefer to get the name from the command line as much as
            // possible.
            proc_.name = c_buf_to_os_string(&kproc.p_comm);
        }
    }

    // We now get the values needed for both new and existing process.
    if refresh_kind.cpu() {
        proc_.cpu_usage = (100 * kproc.p_pctcpu) as f32 / system_info.fscale;
    }

    // Processes can be reparented apparently?
    proc_.parent = if kproc.p_ppid != 0 {
        Some(Pid(kproc.p_ppid))
    } else {
        None
    };

    if let Some(kd) = system_info.kd {
        if refresh_kind
            .environ()
            .needs_update(|| proc_.environ.is_empty())
        {
            proc_.environ =
                unsafe { from_cstr_array(ffi::kvm_getenvv2(kd.as_ptr(), kproc, 0) as _) };
        }

        proc_.status = match kproc.p_realstat {
            ffi::SIDL => ProcessStatus::Idle,
            ffi::SSTOP => ProcessStatus::Stop,
            ffi::SZOMB => ProcessStatus::Zombie,
            ffi::SDEAD => ProcessStatus::Tracing,
            ffi::SACTIVE => get_active_status(kd, kproc)
                .unwrap_or(ProcessStatus::Unknown(kproc.p_realstat as _)),
            _ => ProcessStatus::Unknown(kproc.p_realstat as _),
        };
    }

    if refresh_kind.memory() {
        proc_.virtual_memory = kproc.p_vm_vsize as _;
        proc_.memory = (kproc.p_vm_rssize as u64).saturating_mul(system_info.page_size as _);
    }

    if refresh_kind.root().needs_update(|| proc_.root.is_none()) {
        proc_.root = realpath(format!("/proc/{}/root", proc_.pid));
    }

    proc_.updated = true;
}

pub(crate) unsafe fn get_process_data(
    kproc: &libc::kinfo_proc2,
    wrap: &WrapMap,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    system_info: &super::system::SystemInfo,
) -> Result<Option<Process>, ()> {
    if kproc.p_flag & (ffi::P_SYSTEM as i32) != 0 {
        // We filter out the kernel threads.
        return Err(());
    }

    // FIXME: This is to get the "real" run time (in micro-seconds).
    // let run_time = (kproc.ki_runtime + 5_000) / 10_000;

    let start_time = kproc.p_ustart_sec as u64;

    if let Some(proc_) = unsafe { (*wrap.0.get()).get_mut(&Pid(kproc.p_pid)) } {
        let proc_ = &mut proc_.inner;
        proc_.updated = true;
        // If the `start_time` we just got is different from the one stored, it means it's not the
        // same process.
        if proc_.start_time == start_time {
            update_proc_info(kproc, system_info, now, refresh_kind, proc_);

            return Ok(None);
        }
    }

    // This is a new process, we need to get more information!

    let mut inner = ProcessInner {
        pid: Pid(kproc.p_pid),
        parent: None,
        user_id: Uid(kproc.p_ruid),
        effective_user_id: Uid(kproc.p_uid),
        group_id: Gid(kproc.p_rgid),
        effective_group_id: Gid(kproc.p_gid),
        start_time,
        run_time: 0,
        cpu_usage: 0.,
        virtual_memory: 0,
        memory: 0,
        cwd: None,
        exe: None,
        name: OsString::new(),
        cmd: Vec::new(),
        root: None,
        environ: Vec::new(),
        status: ProcessStatus::Unknown(0),
        read_bytes: 0,
        old_read_bytes: 0,
        written_bytes: 0,
        old_written_bytes: 0,
        accumulated_cpu_time: 0,
        updated: true,
        exists: true,
    };

    update_proc_info(kproc, system_info, now, refresh_kind, &mut inner);

    Ok(Some(Process { inner }))
}

pub(crate) unsafe fn get_path(pid: crate::Pid, kind: libc::c_int) -> Option<PathBuf> {
    let mut buffer = [0; libc::MAXPATHLEN as usize + 1];

    unsafe {
        get_sys_value_osstr(
            &[libc::CTL_KERN, libc::KERN_PROC_ARGS, pid.0, kind],
            &mut buffer,
        )
        .map(PathBuf::from)
    }
}
