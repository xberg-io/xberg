// Take a look at the license at the top of the repository in the LICENSE file.

use crate::unix::users::UserInner;
use crate::{Gid, Uid, User};

use libc::c_void;
use objc2_core_foundation::{
    CFArray, CFRetained, CFString, CFStringBuiltInEncodings, kCFAllocatorDefault, kCFAllocatorNull,
};
use objc2_open_directory::{
    ODNodeRef, ODQueryRef, ODRecordRef, kODAttributeTypePrimaryGroupID, kODAttributeTypeRecordName,
    kODAttributeTypeUniqueID, kODRecordTypeUsers, kODSessionDefault,
};
use std::ptr::null_mut;
use std::str::FromStr;

pub(crate) fn get_users(users: &mut Vec<User>) {
    let node_name = b"/Local/Default\0";
    users.clear();

    unsafe {
        let node_name = CFString::with_c_string_no_copy(
            None,
            node_name.as_ptr() as *const _,
            CFStringBuiltInEncodings::EncodingMacRoman.0,
            kCFAllocatorNull,
        );
        let node_ref = ODNodeRef::with_name(
            kCFAllocatorDefault,
            kODSessionDefault,
            node_name.as_deref(),
            null_mut(),
        );
        if node_ref.is_none() {
            sysinfo_debug!("get_users failed: `ODNodeRef::with_name` returned nothing");
            return;
        }
        let Some(attr_name) = kODAttributeTypeRecordName else {
            sysinfo_debug!("Cannot get attribute for user name");
            return;
        };
        let Some(attr_uid) = kODAttributeTypeUniqueID else {
            sysinfo_debug!("Cannot get attribute for user id");
            return;
        };
        let Some(attr_gid) = kODAttributeTypePrimaryGroupID else {
            sysinfo_debug!("Cannot get attribute for user group id");
            return;
        };
        #[allow(clippy::missing_transmute_annotations)]
        let attributes: CFRetained<CFArray<CFString>> = CFArray::from_objects(&[
            // NSString <-> CFString conversion is "toll-free bridging".
            std::mem::transmute::<_, &CFString>(attr_name),
            std::mem::transmute::<_, &CFString>(attr_uid),
            std::mem::transmute::<_, &CFString>(attr_gid),
        ]);
        let Some(query) = ODQueryRef::with_node(
            kCFAllocatorDefault,
            node_ref.as_deref(),
            // NSString <-> CFString conversion is "toll-free bridging".
            kODRecordTypeUsers.map(|v| std::mem::transmute(v)),
            None,
            0,
            None,
            Some(&attributes),
            0,
            null_mut(),
        ) else {
            sysinfo_debug!("get_users failed: `ODQueryRef::with_node` returned nothing");
            return;
        };
        let Some(results) = ODQueryRef::results(&query, false, null_mut()) else {
            sysinfo_debug!("get_users failed: `ODQueryRef::results` returned nothing");
            return;
        };
        let len = results.count();
        for i in 0..len {
            if let Some(user) = add_user(results.value_at_index(i)) {
                users.push(user);
            }
        }
    }
}

fn add_user(result: *const c_void) -> Option<User> {
    if result.is_null() {
        return None;
    }
    unsafe {
        let result: &ODRecordRef = &*(result as *const _);

        let values = result.values(kODAttributeTypeRecordName, null_mut())?;
        let values = values.cast_unchecked::<CFString>();
        let name = values.get(0).map(|v| v.to_string())?;

        let values = result.values(kODAttributeTypeUniqueID, null_mut())?;
        let values = values.cast_unchecked::<CFString>();
        // FIXME: Would be nice to not have the `to_string` allocation... Maybe by iterating through
        // the chars to generate the integer? Or eventually to find a way to have a `&str` out of
        // the `CFString`.
        let uid = values
            .get(0)
            .and_then(|v| libc::uid_t::from_str(&v.to_string()).ok())?;

        let values = result.values(kODAttributeTypePrimaryGroupID, null_mut())?;
        let values = values.cast_unchecked::<CFString>();
        // FIXME: Would be nice to not have the `to_string` allocation... Maybe by iterating through
        // the chars to generate the integer? Or eventually to find a way to have a `&str` out of
        // the `CFString`.
        let gid = values
            .get(0)
            .and_then(|v| libc::gid_t::from_str(&v.to_string()).ok())?;

        Some(User {
            inner: UserInner::new(Uid(uid), Gid(gid), name),
        })
    }
}
