//! Atom definitions for Elixir interop
//!
//! This module contains all Elixir atom definitions used in NIFs for tuples,
//! maps, and return values. Atoms are Elixir's constants used for pattern matching.

rustler::atoms! {
    ok,
    error,
    invalid_input,
    extraction_failed,
    parsing_error,
    validation_error,
    io_error,
    invalid_format,
    invalid_config,
    ocr_error,
    embedding_error,
    cancelled,
    unknown_error,
    not_found,
    done,
}
