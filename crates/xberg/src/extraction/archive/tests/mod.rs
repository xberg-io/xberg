use super::*;
use crate::extractors::security::SecurityLimits;

fn default_limits() -> SecurityLimits {
    SecurityLimits::default()
}

fn entry(path: &str) -> ArchiveEntry {
    ArchiveEntry {
        path: path.to_string(),
        size: 0,
        is_dir: false,
    }
}

mod formats;
mod gzip_and_limits;

#[test]
fn declared_size_sum_saturates_instead_of_wrapping() {
    let near_limit = add_declared_size(0, u64::MAX - 10);
    assert_eq!(add_declared_size(near_limit, 100), u64::MAX);
    assert_eq!(add_declared_size(u64::MAX, u64::MAX), u64::MAX);
    assert_eq!(add_declared_size(40, 2), 42);
}
