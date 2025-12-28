#![allow(clippy::let_unit_value)]

mod types;
mod utils;

use rustler::{Encoder, Env, NifResult, Term};

rustler::init!("Elixir.Kreuzberg.Native", load = on_load);

#[allow(non_local_definitions)]
fn on_load(_env: Env, _info: Term) -> bool {
    true
}

mod atoms {
    rustler::atoms! {
        ok,
        error,
        invalid_input,
        extraction_failed,
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
fn extract<'a>(env: Env<'a>, _input: rustler::Binary, _input_type: String) -> NifResult<Term<'a>> {
    // Placeholder implementation
    Ok((atoms::error(), "not implemented yet").encode(env))
}
