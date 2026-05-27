<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Custom validator implementation is not available in the Elixir binding. Validators must be implemented in Rust using the `Validator` trait.

To implement a custom validator in Rust and use it from Elixir:

1. Implement the `Plugin` and `Validator` traits in Rust
2. Register the validator in the Rust core
3. Call extraction functions from Elixir, which will automatically apply registered validators

The validator will run after extraction completes and can reject results that don't meet validation criteria.
