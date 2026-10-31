// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "debug")]
#[doc(hidden)]
#[allow(unused)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{
        eprintln!($($x)*);
    }}
}

#[cfg(not(feature = "debug"))]
#[doc(hidden)]
#[allow(unused)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{}};
}

#[cfg(feature = "system")]
macro_rules! declare_signals {
    ($kind:ty, _ => None,) => (
        use crate::Signal;

        pub(crate) const fn supported_signals() -> &'static [Signal] {
            &[]
        }
    );

    ($kind:ty, $(Signal::$signal:ident => $map:expr,)+ _ => None,) => (
        use crate::Signal;

        pub(crate) const fn supported_signals() -> &'static [Signal] {
            &[$(Signal::$signal,)*]
        }

        #[inline]
        pub(crate) fn convert_signal(s: Signal) -> Option<$kind> {
            match s {
                $(Signal::$signal => Some($map),)*
                _ => None,
            }
        }
    );

    ($kind:ty, $(Signal::$signal:ident => $map:expr,)+) => (
        use crate::Signal;

        pub(crate) const fn supported_signals() -> &'static [Signal] {
            &[$(Signal::$signal,)*]
        }

        #[inline]
        pub(crate) fn convert_signal(s: Signal) -> Option<$kind> {
            match s {
                $(Signal::$signal => Some($map),)*
            }
        }
    )
}

#[cfg(all(unix, not(feature = "unknown-ci")))]
#[allow(unused_macros)]
macro_rules! retry_eintr {
    (set_to_0 => $($t:tt)+) => {{
        #[allow(unused_unsafe)]
        let errno = unsafe { crate::unix::libc_errno() };
        if !errno.is_null() {
            #[allow(unused_unsafe)]
            unsafe { *errno = 0; }
        }
        retry_eintr!($($t)+)
    }};
    ($errno_value:ident => $($t:tt)+) => {{
        loop {
            let ret = $($t)+;
            if ret < 0 {
                let tmp = std::io::Error::last_os_error();
                if tmp.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                $errno_value = tmp.raw_os_error().unwrap_or(0);
            }
            break ret;
        }
    }};
    ($($t:tt)+) => {{
        loop {
            let ret = $($t)+;
            if ret < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break ret;
        }
    }};
}
