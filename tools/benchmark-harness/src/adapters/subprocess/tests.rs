//! Test suite for the `subprocess` adapter module, split by what each group exercises.

mod support;

mod adapter_basics;
mod batch_exec_tests;
mod builders;
mod docling_version;
mod error_kind;
mod layout_failure_rejection;
mod liteparse_staging_tests;
mod ocr_args_tests;
mod parse_output_and_timeout;
mod process_exec_tests;
mod xberg_batch_threading;
