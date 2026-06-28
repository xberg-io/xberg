#![cfg(feature = "mcp")]

use rmcp::schemars::schema_for;
use xberg::mcp::ExtractBatchParams;

#[test]
fn test_inputs_items_is_object_not_boolean() {
    // Regression test for https://github.com/xberg-io/xberg/issues/877
    // Moonshot AI (Kimi) rejects `"items": true` — array items must be an object.
    // `inputs` is `Vec<serde_json::Value>`, which would default to `items: true`;
    // a custom `schema_with` override must keep it an object schema instead.
    let schema = schema_for!(ExtractBatchParams);
    let schema_value = serde_json::to_value(&schema).unwrap();

    let items = &schema_value["properties"]["inputs"]["items"];
    assert!(
        items.is_object(),
        "inputs items must be an object, got: {items} — \
         Moonshot AI rejects boolean `items: true` (issue #877)"
    );
}

#[test]
fn test_inputs_items_describes_the_input_envelope() {
    // The per-input object schema must expose the unified input shape so MCP
    // clients can construct `{kind, bytes|uri, mime_type, ...}` entries.
    let schema = schema_for!(ExtractBatchParams);
    let schema_value = serde_json::to_value(&schema).unwrap();

    let item_props = &schema_value["properties"]["inputs"]["items"]["properties"];
    assert!(
        item_props["kind"].is_object(),
        "inputs items must document the `kind` discriminator, got: {item_props}"
    );
}

#[test]
fn test_inputs_is_required() {
    // `inputs` is a non-optional `Vec`, so it must appear in the schema's
    // `required` set.
    let schema = schema_for!(ExtractBatchParams);
    let schema_value = serde_json::to_value(&schema).unwrap();

    let is_required = schema_value["required"]
        .as_array()
        .map(|r| r.iter().any(|f| f.as_str() == Some("inputs")))
        .unwrap_or(false);

    assert!(is_required, "inputs must be a required field on ExtractBatchParams");
}
