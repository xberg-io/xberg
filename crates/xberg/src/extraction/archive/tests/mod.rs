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
fn declared_size_sum_accepts_exact_maximum_and_rejects_overflow() {
    assert_eq!(add_declared_size(40, 2).unwrap(), 42);
    assert_eq!(add_declared_size(u64::MAX - 10, 10).unwrap(), u64::MAX);
    assert!(add_declared_size(u64::MAX - 10, 11).is_err());
    assert!(add_declared_size(u64::MAX, u64::MAX).is_err());
}

#[test]
fn decompressed_budget_charges_reads_before_a_decoder_error() {
    use std::io::{self, Read};

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read failed"))
        }
    }

    let mut budget = DecompressedBudget::new(4);
    let reader = b"abc".as_slice().chain(FailingReader);
    assert!(
        budget
            .read_member(reader, &mut io::sink(), None, "first")
            .unwrap()
            .is_err()
    );
    let error = budget
        .read_member(b"de".as_slice(), &mut io::sink(), None, "second")
        .unwrap_err();
    assert!(error.to_string().contains("second"));
}
