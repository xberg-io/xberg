// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    Cpu, CpuRefreshKind, LoadAvg, MemoryRefreshKind, Pid, Process, ProcessRefreshKind,
    ProcessesToUpdate,
};

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use crate::sys::cpu::{CpusWrapper, physical_core_count};
use crate::sys::utils::{self, c_buf_to_utf8_string, get_sys_value, get_sys_value_by_name};

use super::ffi;
use crate::unix::bsd::common::{boot_time, init_mib};

use libc::c_int;

pub(crate) struct SystemInner {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    mem_used: u64,
    swap_total: u64,
    swap_used: u64,
    system_info: SystemInfo,
    cpus: CpusWrapper,
}

impl SystemInner {
    pub(crate) fn new() -> Self {
        Self {
            process_list: HashMap::with_capacity(200),
            mem_total: 0,
            mem_free: 0,
            mem_used: 0,
            swap_total: 0,
            swap_used: 0,
            system_info: SystemInfo::new(),
            cpus: CpusWrapper::new(),
        }
    }

    pub(crate) fn refresh_memory_specifics(&mut self, refresh_kind: MemoryRefreshKind) {
        if !refresh_kind.ram() && !refresh_kind.swap() {
            return;
        }

        let mib = [libc::CTL_VM, ffi::VM_UVMEXP2];
        let mut info = MaybeUninit::<ffi::uvmexp_sysctl>::uninit();

        let info = unsafe {
            if !get_sys_value(&mib, &mut info) {
                sysinfo_debug!(
                    "failed to get memory information: failed to query uvmexp information"
                );
                return;
            }
            info.assume_init()
        };

        if refresh_kind.ram() {
            self.mem_total = info.npages as u64 * self.system_info.page_size;
            self.mem_used = (info.active as u64 + info.wired as u64) * self.system_info.page_size;
            let cached_memory =
                (info.filepages + info.execpages) as u64 * self.system_info.page_size;
            self.mem_free = self.mem_total.saturating_sub(self.mem_used + cached_memory);
        }
        if refresh_kind.swap() {
            self.swap_total = info.swpages as u64 * self.system_info.page_size;
            self.swap_used = info.swpginuse as u64 * self.system_info.page_size;
        }
    }

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        None
    }

    pub(crate) fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        self.cpus.refresh(refresh_kind)
    }

    pub(crate) fn refresh_cpu_list(&mut self, refresh_kind: CpuRefreshKind) {
        self.cpus = CpusWrapper::new();
        self.cpus.refresh(refresh_kind);
    }

    pub(crate) fn refresh_processes_specifics(
        &mut self,
        processes_to_update: ProcessesToUpdate<'_>,
        refresh_kind: ProcessRefreshKind,
    ) -> usize {
        unsafe { self.refresh_procs(processes_to_update, refresh_kind) }
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    pub(crate) fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    pub(crate) fn processes_mut(&mut self) -> &mut HashMap<Pid, Process> {
        &mut self.process_list
    }

    pub(crate) fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    pub(crate) fn global_cpu_usage(&self) -> f32 {
        self.cpus.global_cpu_usage
    }

    pub(crate) fn cpus(&self) -> &[Cpu] {
        &self.cpus.cpus
    }

    pub(crate) fn total_memory(&self) -> u64 {
        self.mem_total
    }

    pub(crate) fn free_memory(&self) -> u64 {
        self.mem_free
    }

    pub(crate) fn available_memory(&self) -> u64 {
        self.mem_free
    }

    pub(crate) fn used_memory(&self) -> u64 {
        self.mem_used
    }

    pub(crate) fn total_swap(&self) -> u64 {
        self.swap_total
    }

    pub(crate) fn free_swap(&self) -> u64 {
        self.swap_total - self.swap_used
    }

    pub(crate) fn used_swap(&self) -> u64 {
        self.swap_used
    }

    pub(crate) fn uptime() -> u64 {
        unsafe {
            let csec = libc::time(std::ptr::null_mut());

            libc::difftime(csec, Self::boot_time() as _) as u64
        }
    }

    pub(crate) fn boot_time() -> u64 {
        boot_time()
    }

    pub(crate) fn load_average() -> LoadAvg {
        let mut loads = vec![0f64; 3];
        unsafe {
            libc::getloadavg(loads.as_mut_ptr(), 3);
            LoadAvg {
                one: loads[0],
                five: loads[1],
                fifteen: loads[2],
            }
        }
    }

    pub(crate) fn name() -> Option<String> {
        let mut os_type: [c_int; 2] = [0; 2];
        unsafe {
            init_mib(b"kern.ostype\0", &mut os_type);
            get_system_info(&os_type, Some("NetBSD"))
        }
    }

    pub(crate) fn os_version() -> Option<String> {
        let mut os_release: [c_int; 2] = [0; 2];
        unsafe {
            init_mib(b"kern.osrelease\0", &mut os_release);
            // It returns something like "13.0-RELEASE". We want to keep everything until the "-".
            get_system_info(&os_release, None)
                .and_then(|s| s.split('-').next().map(|s| s.to_owned()))
        }
    }

    pub(crate) fn long_os_version() -> Option<String> {
        let mut os_release: [c_int; 2] = [0; 2];
        unsafe {
            init_mib(b"kern.version\0", &mut os_release);
            get_system_info(&os_release, None)
        }
    }

    pub(crate) fn host_name() -> Option<String> {
        let mut hostname: [c_int; 2] = [0; 2];
        unsafe {
            init_mib(b"kern.hostname\0", &mut hostname);
            get_system_info(&hostname, None)
        }
    }

    pub(crate) fn kernel_version() -> Option<String> {
        unsafe {
            let mut kern_version: libc::c_int = 0;
            if get_sys_value_by_name(b"kern.osrevision\0", &mut kern_version) {
                Some(kern_version.to_string())
            } else {
                None
            }
        }
    }

    pub(crate) fn distribution_id() -> String {
        std::env::consts::OS.to_owned()
    }

    pub(crate) fn distribution_id_like() -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn kernel_name() -> Option<&'static str> {
        Some("NetBSD")
    }

    pub(crate) fn cpu_arch() -> Option<String> {
        let mut arch_str: [u8; 32] = [0; 32];
        let mib = [ffi::CTL_HW as _, ffi::HW_MACHINE as _];

        unsafe {
            if get_sys_value(&mib, &mut arch_str) {
                CStr::from_bytes_until_nul(&arch_str)
                    .ok()
                    .and_then(|res| match res.to_str() {
                        Ok(arch) => Some(arch.to_string()),
                        Err(_) => None,
                    })
            } else {
                None
            }
        }
    }

    pub(crate) fn physical_core_count() -> Option<usize> {
        physical_core_count()
    }

    pub(crate) fn open_files_limit() -> Option<usize> {
        let mut value = 0u32;
        unsafe {
            if get_sys_value_by_name(b"kern.maxfilesper\0", &mut value) {
                Some(value as _)
            } else {
                None
            }
        }
    }
}

impl SystemInner {
    unsafe fn refresh_procs(
        &mut self,
        processes_to_update: ProcessesToUpdate<'_>,
        refresh_kind: ProcessRefreshKind,
    ) -> usize {
        let (op, arg) = match processes_to_update {
            ProcessesToUpdate::Some(&[]) => return 0,
            ProcessesToUpdate::Some(&[pid]) => (libc::KERN_PROC_PID, pid.as_u32() as c_int),
            _ => (libc::KERN_PROC_ALL, 0),
        };
        let Some(kd) = self.system_info.kd else {
            return 0;
        };

        let mut count = 0;
        let kvm_procs = unsafe {
            ffi::kvm_getproc2(
                kd.as_ptr(),
                op,
                arg,
                std::mem::size_of::<libc::kinfo_proc2>(),
                &mut count,
            )
        };
        if count < 1 {
            sysinfo_debug!("kvm_getproc2 returned nothing...");
            return 0;
        }

        #[inline(always)]
        fn real_filter(e: &libc::kinfo_proc2, filter: &[Pid]) -> bool {
            filter.contains(&Pid(e.p_pid))
        }

        #[inline(always)]
        fn empty_filter(_e: &libc::kinfo_proc2, _filter: &[Pid]) -> bool {
            true
        }

        #[allow(clippy::type_complexity)]
        let (filter, filter_callback): (
            &[Pid],
            &(dyn Fn(&libc::kinfo_proc2, &[Pid]) -> bool + Sync + Send),
        ) = match processes_to_update {
            ProcessesToUpdate::All => (&[], &empty_filter),
            ProcessesToUpdate::Some(pids) => {
                if pids.is_empty() {
                    return 0;
                }
                (pids, &real_filter)
            }
        };

        let nb_updated = AtomicUsize::new(0);

        let new_processes = {
            #[cfg(feature = "multithread")]
            use rayon::iter::{ParallelIterator, ParallelIterator as IterTrait};
            #[cfg(not(feature = "multithread"))]
            use std::iter::Iterator as IterTrait;

            unsafe {
                let kvm_procs: &mut [utils::KInfoProc] =
                    std::slice::from_raw_parts_mut(kvm_procs as _, count as _);

                let system_info = &self.system_info;
                let now = get_now();
                let proc_list = utils::WrapMap(UnsafeCell::new(&mut self.process_list));

                IterTrait::filter_map(crate::utils::into_iter(kvm_procs), |kproc| {
                    if !filter_callback(kproc, filter) {
                        return None;
                    }
                    let ret = super::process::get_process_data(
                        kproc,
                        &proc_list,
                        now,
                        refresh_kind,
                        system_info,
                    )
                    .ok()?;
                    nb_updated.fetch_add(1, Ordering::Relaxed);
                    ret
                })
                .collect::<Vec<_>>()
            }
        };

        for process in new_processes {
            self.process_list.insert(process.inner.pid, process);
        }
        nb_updated.into_inner()
    }
}

/// This struct is used to get system information more easily.
#[derive(Debug)]
pub(crate) struct SystemInfo {
    pub(crate) page_size: u64,
    pub(crate) kd: Option<NonNull<ffi::kvm_t>>,
    /// From NetBSD manual: "The kernel fixed-point scale factor". It's used when computing
    /// processes' CPU usage.
    pub(crate) fscale: f32,
    /// NetBSD does not expose the block size per process, only the total
    /// blocks in and out. We use the User-preferred display block size
    /// `getbsize(3)` to convert `struct rusage` block-op counts to bytes
    /// for per-process disk-usage reporting.
    pub(crate) block_size: u64,
}

// This is needed because `kd: *mut libc::kvm_t` isn't thread-safe.
unsafe impl Send for SystemInfo {}
unsafe impl Sync for SystemInfo {}

impl SystemInfo {
    fn new() -> Self {
        unsafe {
            let mut errbuf = MaybeUninit::<[libc::c_char; ffi::_POSIX2_LINE_MAX]>::uninit();
            let kd = NonNull::new(ffi::kvm_openfiles(
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                ffi::KVM_NO_FILES,
                errbuf.as_mut_ptr() as *mut _,
            ));
            if kd.is_none() {
                sysinfo_debug!("kvm_openfiles failed, cannot retrieve processes information");
            }

            let mut si = SystemInfo {
                page_size: 0,
                kd,
                fscale: 0.,
                block_size: 512,
            };
            let mut fscale: c_int = 0;
            if !get_sys_value(&[libc::CTL_KERN, libc::KERN_FSCALE], &mut fscale) || fscale < 0 {
                sysinfo_debug!("failed to get fscale, cannot retrieve CPU usage information");
            } else {
                si.fscale = fscale as f32;
            }

            let page_size = libc::sysconf(libc::_SC_PAGESIZE);
            if page_size < 0 {
                sysinfo_debug!("failed to get page size, cannot retrieve memory information");
            } else {
                si.page_size = page_size as _;
            }

            let mut block_size: libc::c_long = 0;
            // `getbsize` isn't thread-safe because it returns a `static char*`. However
            // we don't read from this `char*` so it's all fine.
            ffi::getbsize(std::ptr::null_mut(), &mut block_size);
            if block_size > 0 {
                si.block_size = block_size as u64;
            }

            si
        }
    }
}

impl Drop for SystemInfo {
    fn drop(&mut self) {
        unsafe {
            if let Some(kd) = self.kd {
                ffi::kvm_close(kd.as_ptr());
            }
        }
    }
}

fn get_system_info(mib: &[c_int], default: Option<&str>) -> Option<String> {
    let mut size = 0;

    unsafe {
        // Call first to get size
        libc::sysctl(
            mib.as_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        );

        // exit early if we did not update the size
        if size == 0 {
            default.map(|s| s.to_owned())
        } else {
            // set the buffer to the correct size
            let mut buf: Vec<libc::c_char> = vec![0; size as _];

            if libc::sysctl(
                mib.as_ptr(),
                mib.len() as _,
                buf.as_mut_ptr() as _,
                &mut size,
                std::ptr::null_mut(),
                0,
            ) == -1
            {
                // If command fails return default
                default.map(|s| s.to_owned())
            } else {
                c_buf_to_utf8_string(&buf)
            }
        }
    }
}

fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())
        .unwrap_or(0)
}
