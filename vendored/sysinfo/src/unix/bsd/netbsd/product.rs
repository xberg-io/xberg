// Take a look at the license at the top of the repository in the LICENSE file.

use super::utils::get_sys_value_str_by_name;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Option<String> {
        // FIXME
        None
    }

    pub(crate) fn name() -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-product\0")
    }

    pub(crate) fn serial_number() -> Option<String> {
        // FIXME
        get_sys_value_str_by_name(b"machdep.dmi.system-serial\0")
    }

    pub(crate) fn stock_keeping_unit() -> Option<String> {
        // FIXME
        None
    }

    pub(crate) fn uuid() -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-uuid\0")
    }

    pub(crate) fn version() -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-version\0")
    }

    pub(crate) fn vendor_name() -> Option<String> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-vendor\0")
    }
}
