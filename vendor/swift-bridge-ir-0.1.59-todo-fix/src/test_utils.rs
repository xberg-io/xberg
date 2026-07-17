use crate::errors::ParseErrors;
use crate::parse::SwiftBridgeModuleAndErrors;
use crate::SwiftBridgeModule;
use proc_macro2::TokenStream;

pub fn assert_tokens_eq(left: &TokenStream, right: &TokenStream) {
    assert_eq!(
        token_stream_to_vec(&left),
        token_stream_to_vec(&right),
        r#"
Left Tokens:
{}

Right Tokens:
{}
"#,
        left.to_string(),
        right.to_string()
    )
}

/// Converts both token streams to strings, removes all of the whitespace then checks that the outer
/// token stream contains the inner one.
pub fn assert_tokens_contain(outer: &TokenStream, inner: &TokenStream) {
    let outer_string = outer.to_string();
    let outer_string = outer_string.replace(" ", "").replace("\n", "");

    let inner_string = inner.to_string();
    let inner_string = inner_string.replace(" ", "").replace("\n", "");

    let is_contained = outer_string.contains(&inner_string);

    assert!(
        is_contained,
        r#"
Outer tokens do not contain the inner tokens. 

Outer Tokens:
{}

Inner Tokens:
{}
"#,
        outer.to_string(),
        inner.to_string()
    )
}

/// Converts both token streams to strings, removes all of the whitespace then checks that the outer
/// token stream does not contain the inner one.
pub fn assert_tokens_do_not_contain(outer: &TokenStream, inner: &TokenStream) {
    let outer_string = outer.to_string();
    let outer_string = outer_string.replace(" ", "").replace("\n", "");

    let inner_string = inner.to_string();
    let inner_string = inner_string.replace(" ", "").replace("\n", "");

    let is_contained = outer_string.contains(&inner_string);

    assert!(
        !is_contained,
        r#"
Outer tokens do not contain the inner tokens. 

Outer Tokens:
{}

Inner Tokens:
{}
"#,
        outer.to_string(),
        inner.to_string()
    )
}

/// Trims both generated and expected.
pub fn assert_trimmed_generated_equals_trimmed_expected(generated: &str, expected: &str) {
    assert_eq!(
        generated.trim(),
        expected.trim(),
        r#"Expected did not equal generated.
Generated:
{}
Expected:
{}"#,
        generated.trim(),
        expected.trim()
    );
}

/// Trims both generated and expected.
pub fn assert_trimmed_generated_contains_trimmed_expected(generated: &str, expected: &str) {
    assert!(
        generated.trim().contains(&expected.trim()),
        r#"Expected was not contained by generated.
Generated:
{}
Expected:
{}"#,
        generated.trim(),
        expected.trim()
    );
}

/// Trims both generated and expected.
pub fn assert_trimmed_generated_does_not_contain_trimmed_expected(generated: &str, expected: &str) {
    assert!(
        !generated.trim().contains(&expected.trim()),
        r#"Expected was contained by generated.
Generated:
{}
Expected:
{}"#,
        generated.trim(),
        expected.trim()
    );
}

pub(crate) fn parse_ok(tokens: TokenStream) -> SwiftBridgeModule {
    let module_and_errors: SwiftBridgeModuleAndErrors = syn::parse2(tokens).unwrap();
    module_and_errors.module
}

pub(crate) fn parse_errors(tokens: TokenStream) -> ParseErrors {
    let parsed: SwiftBridgeModuleAndErrors = syn::parse2(tokens).unwrap();
    parsed.errors
}

fn token_stream_to_vec(tokens: &TokenStream) -> Vec<String> {
    tokens
        .clone()
        .into_iter()
        .map(|t| t.to_string().trim().to_string())
        .collect()
}
