//! Built-in [`VectorStore`](crate::VectorStore) backends.
//!
//! - [`memory`] — pure-Rust brute-force store (default; WASM-safe; tests/dev).
//! - `sqlite` — embedded `rusqlite` + `sqlite-vec` store (feature `sqlite`,
//!   native-only). Heavy/native third-party backends (lancedb, pgvector, …) live
//!   in their own adapter crates, not here.
//! - `graphqlite` — SQLite-backed graph store for Cypher-like traversal,
//!   Louvain community detection, and PageRank (feature `sqlite`, native-only).

#[cfg(feature = "in-memory")]
pub mod memory;

#[cfg(feature = "sqlite")]
pub mod graphqlite;

#[cfg(feature = "sqlite")]
pub mod sqlite;
