// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    Group,
    common::{Gid, Uid},
};
use libc::{c_int, getgrgid_r, getgrouplist};

pub(crate) struct UserInner {
    pub(crate) uid: Uid,
    pub(crate) gid: Gid,
    pub(crate) name: String,
    c_user: Vec<u8>,
}

impl UserInner {
    pub(crate) fn new(uid: Uid, gid: Gid, name: String) -> Self {
        let mut c_user = name.as_bytes().to_vec();
        c_user.push(0);
        Self {
            uid,
            gid,
            name,
            c_user,
        }
    }

    pub(crate) fn id(&self) -> &Uid {
        &self.uid
    }

    pub(crate) fn group_id(&self) -> Gid {
        self.gid
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn groups(&self) -> Vec<Group> {
        unsafe { get_user_groups(self.c_user.as_ptr() as *const _, self.gid.0 as _) }
    }
}

pub(crate) unsafe fn get_group_name(
    id: libc::gid_t,
    buffer: &mut Vec<libc::c_char>,
) -> Option<String> {
    let mut g = std::mem::MaybeUninit::<libc::group>::uninit();
    let mut tmp_ptr = std::ptr::null_mut();
    let mut last_errno = 0;

    unsafe {
        loop {
            if retry_eintr!(set_to_0 => last_errno => getgrgid_r(
                id as _,
                g.as_mut_ptr() as _,
                buffer.as_mut_ptr(),
                buffer.capacity() as _,
                &mut tmp_ptr as _
            )) != 0
            {
                // If there was not enough memory, we give it more.
                if last_errno == libc::ERANGE as libc::c_int {
                    // Needs to be updated for `Vec::reserve` to actually add additional capacity.
                    // In here it's "fine" since we never read from `buffer`.
                    buffer.set_len(buffer.capacity());
                    buffer.reserve(2048);
                    continue;
                }
                return None;
            }
            break;
        }
        let g = g.assume_init();
        super::utils::cstr_to_rust(g.gr_name)
    }
}

pub(crate) unsafe fn get_user_groups(
    name: *const libc::c_char,
    group_id: libc::gid_t,
) -> Vec<Group> {
    let mut buffer = Vec::with_capacity(2048);
    let mut groups = Vec::with_capacity(256);

    let mut nb_groups = groups.capacity() as c_int;
    let mut old_nb_groups = nb_groups;
    loop {
        unsafe {
            if getgrouplist(name, group_id as _, groups.as_mut_ptr(), &mut nb_groups) == -1 {
                // prevent infinite looping.
                if old_nb_groups >= nb_groups {
                    sysinfo_debug!(
                        "getgrouplist failed but buffer size requested was not larger than existing buffer"
                    );
                    return Vec::new();
                }
                groups.reserve(nb_groups as usize);
                // reserve could reserve more capacity than requestes, so this could
                // overflow c_int.
                nb_groups = groups.capacity().try_into().unwrap_or(c_int::MAX);
                old_nb_groups = nb_groups;
                continue;
            }
            groups.set_len(nb_groups as _);
            return groups
                .iter()
                .filter_map(|group_id| {
                    let name = get_group_name(*group_id as _, &mut buffer)?;
                    Some(Group {
                        inner: crate::GroupInner::new(Gid(*group_id as _), name),
                    })
                })
                .collect();
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) fn get_users(users: &mut Vec<crate::User>) {
    use std::io::{BufRead, BufReader};
    use std::str::FromStr;

    fn filter(shell: &str, uid: libc::uid_t) -> bool {
        uid < 65536 && !shell.ends_with("/false") && !shell.ends_with("/uucico")
    }

    users.clear();

    // We cannot use `getpwent`, `setpwent` and `endpwent` because they're not thread-safe. The
    // `getpwent_r` equivalent is not thread-safe either. So we have to retrieve the users list
    // ourselves...
    let Ok(file) = std::fs::File::open("/etc/passwd") else {
        sysinfo_debug!("failed to open `/etc/passwd`");
        return;
    };
    let mut users_map = std::collections::HashMap::with_capacity(10);

    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        let mut parts = line.split(':');
        let Some(name) = parts.next() else { continue };
        if users_map.contains_key(name) {
            continue;
        }
        parts.next(); // We skip the password field.
        let Some(uid) = parts.next().and_then(|v| libc::uid_t::from_str(v).ok()) else {
            continue;
        };
        let Some(gid) = parts.next().and_then(|v| libc::gid_t::from_str(v).ok()) else {
            continue;
        };
        parts.next(); // We skip the "comment" field.
        parts.next(); // We skip the directory field.
        let Some(shell) = parts.next() else { continue };

        if !filter(shell, uid) {
            continue;
        }
        users_map.insert(name.into(), (Uid(uid), Gid(gid)));
    }

    for (name, (uid, gid)) in users_map {
        users.push(crate::User {
            inner: UserInner::new(uid, gid, name),
        });
    }
}
