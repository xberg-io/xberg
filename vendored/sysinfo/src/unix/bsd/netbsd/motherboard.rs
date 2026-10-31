// Take a look at the license at the top of the repository in the LICENSE file.

use super::utils::get_sys_value_str_by_name;

pub(crate) struct MotherboardInner;

impl MotherboardInner {
    pub(crate) fn new() -> Option<Self> {
        // FIXME
        None
    }

    pub(crate) fn asset_tag(&self) -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.board-asset-tag\0")
    }

    pub(crate) fn name(&self) -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.board-product\0")
    }

    pub(crate) fn vendor_name(&self) -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.board-vendor\0")
    }

    pub(crate) fn version(&self) -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.board-version\0")
    }

    pub(crate) fn serial_number(&self) -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.board-serial\0")
    }
}
