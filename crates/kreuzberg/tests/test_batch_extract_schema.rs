use kreuzberg::mcp::BatchExtractFilesParams;
use rmcp::schemars::schema_for;

#[test]
fn test_file_configs_items_is_object_not_boolean() {
    // Regression test for https://github.com/kreuzberg-dev/kreuzberg/issues/877
    // Moonshot AI (Kimi) rejects `"items": true` — items must be an object.
    let schema = schema_for!(BatchExtractFilesParams);
    let schema_value = serde_json::to_value(&schema).unwrap();

    let items = &schema_value["properties"]["file_configs"]["items"];
    assert!(
        items.is_object(),
        "file_configs items must be an object, got: {items} — \
         Moonshot AI rejects boolean `items: true` (issue #877)"
    );
}

#[test]
fn test_file_configs_items_accepts_null_and_object() {
    // items schema must accept both null (use default) and object (per-file config).
    let schema = schema_for!(BatchExtractFilesParams);
    let schema_value = serde_json::to_value(&schema).unwrap();

    let items = &schema_value["properties"]["file_configs"]["items"];
    let any_of = items["anyOf"].as_array().expect("items must have anyOf");

    let types: Vec<&str> = any_of.iter().filter_map(|s| s["type"].as_str()).collect();

    assert!(
        types.contains(&"null"),
        "file_configs items must accept null, got anyOf: {any_of:?}"
    );
    assert!(
        types.contains(&"object"),
        "file_configs items must accept object, got anyOf: {any_of:?}"
    );
}

#[test]
fn test_file_configs_is_not_required() {
    // schema_with bypasses automatic Option<> handling — verify schemars still
    // leaves file_configs out of `required` (the field is optional at the call site).
    let schema = schema_for!(BatchExtractFilesParams);
    let schema_value = serde_json::to_value(&schema).unwrap();

    let is_required = schema_value["required"]
        .as_array()
        .map(|r| r.iter().any(|f| f.as_str() == Some("file_configs")))
        .unwrap_or(false);

    assert!(
        !is_required,
        "file_configs must be optional — outer Option<> must not appear in required"
    );
}
