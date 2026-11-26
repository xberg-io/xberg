use super::*;

const FEATURE_COLLECTION: &[u8] = br#"{
    "type": "FeatureCollection",
    "name": "cities",
    "features": [
        {
            "type": "Feature",
            "properties": {"name": "Berlin", "population": 3645000},
            "geometry": {"type": "Point", "coordinates": [13.405, 52.52]}
        },
        {
            "type": "Feature",
            "properties": {"name": "Paris", "capital": true},
            "geometry": {
                "type": "LineString",
                "coordinates": [[2.20, 48.80], [2.35, 48.86], [2.45, 48.90]]
            }
        }
    ]
}"#;

#[test]
fn test_json_array_objects_render_as_nested_markdown() {
    let value = serde_json::json!({
        "people": [
            {
                "name": "Ada",
                "details": {
                    "role": "Engineer",
                    "active": true
                }
            },
            {
                "name": "Grace",
                "skills": ["compilers", "mathematics"]
            }
        ]
    });
    let mut builder = InternalDocumentBuilder::new("json");
    let mut budget = SecurityBudget::from_config(&ExtractionConfig::default());

    build_json_internal_structure(&value, &mut builder, 1, &mut budget).unwrap();
    let markdown = crate::rendering::render_markdown(&builder.build());

    assert!(markdown.contains("# people"), "missing array heading: {markdown}");
    assert!(markdown.contains("name: Ada"), "missing first nested value: {markdown}");
    assert!(
        markdown.contains("## details"),
        "missing nested object heading: {markdown}"
    );
    assert!(
        markdown.contains("role: Engineer"),
        "missing deeply nested value: {markdown}"
    );
    assert!(markdown.contains("active: true"), "missing boolean value: {markdown}");
    assert!(
        markdown.contains("name: Grace"),
        "missing second nested value: {markdown}"
    );
    assert!(
        markdown.contains("- compilers"),
        "missing nested array value: {markdown}"
    );
    assert!(
        markdown.contains("- mathematics"),
        "missing nested array value: {markdown}"
    );
    let ada = markdown.find("name: Ada").unwrap();
    let grace = markdown.find("name: Grace").unwrap();
    assert!(
        markdown[ada..grace].contains("\n-----\n"),
        "records must be separated by a thematic break: {markdown}"
    );
    assert!(
        !markdown.contains(r#"{\"name\":\"Ada\""#),
        "object remained compact JSON: {markdown}"
    );
}

async fn render_json(content: &[u8]) -> (String, String) {
    let result = StructuredExtractor::new()
        .extract_content(content, "application/json", &ExtractionConfig::default())
        .await
        .unwrap();
    (
        crate::rendering::render_markdown(&result),
        crate::rendering::render_plain(&result),
    )
}

#[tokio::test]
async fn array_records_are_separated_by_a_thematic_break_without_synthetic_labels() {
    let (markdown, _) = render_json(br#"[{"name":"Ada","age":3},{"name":"Grace","age":4}]"#).await;

    assert_eq!(markdown.trim(), "name: Ada\n\nage: 3\n\n-----\n\nname: Grace\n\nage: 4");
}

#[tokio::test]
async fn a_record_ending_in_a_list_is_still_separated_from_the_next() {
    let (markdown, _) = render_json(br#"[{"name": "Grace", "tags": ["x"]}, {"name": "Ada"}]"#).await;

    assert_eq!(markdown.trim(), "name: Grace\n\n# tags\n\n- x\n\n-----\n\nname: Ada");
}

#[tokio::test]
async fn inner_arrays_become_separate_lists_split_by_a_thematic_break() {
    let (markdown, _) = render_json(br#"[[1, 2], [3, 4]]"#).await;

    assert_eq!(markdown.trim(), "- 1\n- 2\n\n-----\n\n- 3\n- 4");
}

#[tokio::test]
async fn scalar_arrays_stay_one_bullet_list() {
    let (markdown, _) = render_json(br#"["a", "b", 3]"#).await;

    assert_eq!(markdown.trim(), "- a\n- b\n- 3");
}

#[tokio::test]
async fn plain_output_separates_records_with_blank_lines_and_no_rule() {
    let (_, plain) = render_json(br#"[{"name":"Ada","age":3},{"name":"Grace","age":4}]"#).await;

    assert!(!plain.contains("--"), "{plain:?}");
    let records: Vec<&str> = plain
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty())
        .collect();
    assert_eq!(records, ["name: Ada", "age: 3", "name: Grace", "age: 4"], "{plain:?}");
}

#[test]
fn test_structured_extractor_plugin_interface() {
    let extractor = StructuredExtractor::new();
    assert_eq!(extractor.name(), "structured-extractor");
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

#[test]
fn test_structured_extractor_supported_mime_types() {
    let extractor = StructuredExtractor::new();
    let mime_types = extractor.supported_mime_types();
    assert_eq!(mime_types.len(), 14);
    assert!(mime_types.contains(&"application/json"));
    assert!(mime_types.contains(&"application/x-ndjson"));
    assert!(mime_types.contains(&"application/jsonl"));
    assert!(mime_types.contains(&"application/x-jsonlines"));
    assert!(mime_types.contains(&"application/x-yaml"));
    assert!(mime_types.contains(&"application/toml"));
    assert!(mime_types.contains(&"application/csl+json"));
    assert!(mime_types.contains(&GEOJSON_MIME_TYPE));
}

#[tokio::test]
async fn geojson_uses_json_extraction_and_preserves_its_mime_type() {
    let extractor = StructuredExtractor::new();
    let content = br#"{"type":"Point","coordinates":[13.4,52.5]}"#;

    assert!(extractor.supported_mime_types().contains(&"application/geo+json"));
    let result = extractor
        .extract_content(content, "application/geo+json", &ExtractionConfig::default())
        .await
        .unwrap();

    assert_eq!(result.mime_type, "application/geo+json");
    assert_eq!(
        result.metadata.additional.get("data_format"),
        Some(&serde_json::json!("json"))
    );
    let rendered = crate::rendering::render_plain(&result);
    assert!(rendered.contains("type: Point"));
    assert!(rendered.contains("position_count: 1"));
    assert!(rendered.contains("bounds"));
    assert!(!rendered.contains("\ncoordinates\n"));
    assert_eq!(result.processing_warnings.len(), 1);
    assert_eq!(result.processing_warnings[0].source, "geojson");
    assert_eq!(
        result.metadata.additional.get("geojson_summarized"),
        Some(&serde_json::json!(true))
    );
}

#[tokio::test]
async fn geojson_summary_aggregates_features_properties_geometry_and_bounds() {
    let result = StructuredExtractor::new()
        .extract_content(FEATURE_COLLECTION, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();
    let summary = result
        .metadata
        .additional
        .get("geojson_summary")
        .expect("summary metadata must be present");

    assert_eq!(summary["type"], "FeatureCollection");
    assert_eq!(summary["feature_count"], 2);
    assert_eq!(summary["position_count"], 4);
    assert_eq!(summary["bounds"], serde_json::json!([2.2, 48.8, 13.405, 52.52]));
    assert_eq!(summary["geometry_types"]["Point"], 1);
    assert_eq!(summary["geometry_types"]["LineString"], 1);
    assert_eq!(
        summary["property_keys"],
        serde_json::json!(["capital", "name", "population"])
    );
    assert!(!crate::rendering::render_plain(&result).contains("Berlin"));
}

#[tokio::test]
async fn geojson_full_coordinates_require_explicit_opt_in() {
    let config = ExtractionConfig {
        geojson: Some(crate::core::config::GeoJsonExtractionConfig {
            include_full_coordinates: true,
        }),
        ..ExtractionConfig::default()
    };
    let result = StructuredExtractor::new()
        .extract_content(FEATURE_COLLECTION, GEOJSON_MIME_TYPE, &config)
        .await
        .unwrap();
    let rendered = crate::rendering::render_plain(&result);

    assert!(rendered.contains("13.405"));
    assert!(rendered.contains("52.52"));
    assert!(result.processing_warnings.is_empty());
    assert!(result.metadata.additional.get("geojson_summary").is_none());
    assert_eq!(
        result.metadata.additional.get("geojson_summarized"),
        Some(&serde_json::json!(false))
    );
}

#[tokio::test]
async fn ordinary_json_keeps_full_coordinate_named_arrays() {
    let result = StructuredExtractor::new()
        .extract_content(
            br#"{"coordinates":[[1,2],[3,4]]}"#,
            "application/json",
            &ExtractionConfig::default(),
        )
        .await
        .unwrap();

    assert!(crate::rendering::render_plain(&result).contains("1\n2"));
    assert!(result.metadata.additional.get("geojson_summarized").is_none());
}

#[tokio::test]
async fn geojson_summary_output_stays_bounded_for_large_coordinate_arrays() {
    const POSITION_COUNT: usize = 10_000;
    const MAX_EXPECTED_SUMMARY_BYTES: usize = 2_000;
    let positions = (0..POSITION_COUNT)
        .map(|index| serde_json::json!([index, index + 1]))
        .collect::<Vec<_>>();
    let content = serde_json::to_vec(&serde_json::json!({
        "type": "LineString",
        "coordinates": positions,
    }))
    .unwrap();

    let result = StructuredExtractor::new()
        .extract_content(&content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();

    let rendered = crate::rendering::render_plain(&result);
    assert!(rendered.len() < MAX_EXPECTED_SUMMARY_BYTES, "summary was {rendered}");
    assert_eq!(
        result.metadata.additional["geojson_summary"]["position_count"],
        serde_json::json!(POSITION_COUNT)
    );
}

#[tokio::test]
async fn geojson_summary_bounds_names_property_keys_and_unknown_geometry_types() {
    const UNKNOWN_GEOMETRY_COUNT: usize = 1_000;
    const PROPERTY_COUNT: usize = 300;
    const MAX_SUMMARY_BYTES: usize = 8 * 1024;
    let geometries = (0..UNKNOWN_GEOMETRY_COUNT)
        .map(|index| serde_json::json!({"type": format!("FakeGeometry{index}")}))
        .collect::<Vec<_>>();
    let mut properties = (0..PROPERTY_COUNT)
        .map(|index| {
            (
                format!("property_{index:03}_{}", "x".repeat(80)),
                serde_json::json!(index),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    properties.insert("k".repeat(10_000), serde_json::json!("oversized key"));
    let content = serde_json::to_vec(&serde_json::json!({
        "type": "FeatureCollection",
        "name": "n".repeat(10_000),
        "features": [{
            "type": "Feature",
            "properties": properties,
            "geometry": {"type": "GeometryCollection", "geometries": geometries}
        }]
    }))
    .unwrap();

    let result = StructuredExtractor::new()
        .extract_content(&content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();
    let summary = &result.metadata.additional["geojson_summary"];
    let encoded = serde_json::to_vec(summary).unwrap();

    assert!(encoded.len() < MAX_SUMMARY_BYTES, "summary was {} bytes", encoded.len());
    assert_eq!(summary["name"].as_str().unwrap().len(), MAX_GEOJSON_NAME_BYTES);
    assert_eq!(summary["name_truncated"], true);
    assert_eq!(summary["property_count"], PROPERTY_COUNT + 1);
    assert_eq!(summary["property_keys_truncated"], true);
    assert_eq!(summary["geometry_types"]["Unknown"], UNKNOWN_GEOMETRY_COUNT);
    assert!(!String::from_utf8(encoded).unwrap().contains("FakeGeometry999"));
    assert!(crate::rendering::render_plain(&result).len() < MAX_SUMMARY_BYTES);
}

#[tokio::test]
async fn geojson_summary_discloses_every_discarded_category() {
    let content = br#"{
        "type":"Feature",
        "id":"feature-1",
        "bbox":[1,2,3,4],
        "foreign":{"nested":[1,2,3]},
        "properties":{"name":"Berlin"},
        "geometry":{"type":"Point","coordinates":[13.4,52.5],"foreignGeometry":true}
    }"#;
    let result = StructuredExtractor::new()
        .extract_content(content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();
    let discarded = result.metadata.additional["geojson_summary"]["discarded_categories"]
        .as_array()
        .unwrap();

    for category in [
        "coordinate_arrays",
        "feature_property_values",
        "feature_ids",
        "declared_bounding_boxes",
        "foreign_members",
    ] {
        assert!(discarded.contains(&serde_json::json!(category)), "missing {category}");
    }
}

#[tokio::test]
async fn every_default_geojson_shape_is_explicitly_marked_as_summarized() {
    for content in [
        br#"null"#.as_slice(),
        br#"{}"#.as_slice(),
        br#"{"properties":{"name":"value"}}"#.as_slice(),
        br#"{"type":"Feature","properties":null,"geometry":null}"#.as_slice(),
        br#"{"type":"Point","coordinates":[]}"#.as_slice(),
        br#"{"type":"Point","coordinates":[1,"bad",3]}"#.as_slice(),
    ] {
        let result = StructuredExtractor::new()
            .extract_content(content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
            .await
            .unwrap();
        assert_eq!(result.metadata.additional["geojson_summarized"], true);
        assert_eq!(result.processing_warnings.len(), 1);
    }
}

#[tokio::test]
async fn geojson_positions_require_two_or_more_numeric_ordinates() {
    let content = br#"{
        "type":"GeometryCollection",
        "geometries":[
            {"type":"Point","coordinates":[1,2,3]},
            {"type":"Point","coordinates":[4]},
            {"type":"Point","coordinates":[5,"bad"]},
            {"type":"Point","coordinates":[]}
        ]
    }"#;
    let result = StructuredExtractor::new()
        .extract_content(content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();
    let summary = &result.metadata.additional["geojson_summary"];

    assert_eq!(summary["position_count"], 1);
    assert_eq!(summary["bounds"], serde_json::json!([1.0, 2.0, 1.0, 2.0]));
    assert_eq!(summary["malformed_position_array_count"], 2);
    assert_eq!(summary["empty_coordinate_array_count"], 1);
}

#[tokio::test]
async fn geojson_summary_validates_coordinate_shape_for_each_geometry_type() {
    let content = br#"{
        "type":"GeometryCollection",
        "geometries":[
            {"type":"Point","coordinates":[[1,2],[3,4]]},
            {"type":"LineString","coordinates":[1,2]},
            {"type":"LineString","coordinates":[[10,11],[12,13]]},
            {"type":"Polygon","coordinates":[[[20,21],[22,23],[24,25],[20,21]]]},
            {"type":"Polygon","coordinates":[[[30,31],[32,33],[34,35],[36,37]]]},
            {"type":"MultiPoint","coordinates":[[40,41],[42,43,44]]},
            {"type":"MultiLineString","coordinates":[[[50,51],[52,53]]]},
            {"type":"MultiPolygon","coordinates":[[[[60,61],[62,63],[64,65],[60,61]]]]},
            {"type":"GeometryCollection","coordinates":[70,71],"geometries":[]}
        ]
    }"#;
    let result = StructuredExtractor::new()
        .extract_content(content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();
    let summary = &result.metadata.additional["geojson_summary"];

    assert_eq!(summary["invalid_geometry_count"], 4);
    assert_eq!(summary["position_count"], 14);
    assert_eq!(summary["bounds"], serde_json::json!([10.0, 11.0, 64.0, 65.0]));
}

#[tokio::test]
async fn geojson_discarded_categories_only_report_values_that_were_omitted() {
    for properties in [serde_json::Value::Null, serde_json::json!({})] {
        let content = serde_json::to_vec(&serde_json::json!({
            "type": "Feature",
            "properties": properties,
            "geometry": null,
        }))
        .unwrap();
        let result = StructuredExtractor::new()
            .extract_content(&content, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
            .await
            .unwrap();
        let discarded = result.metadata.additional["geojson_summary"]["discarded_categories"]
            .as_array()
            .unwrap();

        assert!(discarded.contains(&serde_json::json!("feature_structure")));
        assert!(!discarded.contains(&serde_json::json!("feature_property_values")));
    }
}

#[tokio::test]
async fn geojson_vendor_alias_has_identical_summary_and_rendering() {
    let extractor = StructuredExtractor::new();
    let canonical = extractor
        .extract_content(FEATURE_COLLECTION, GEOJSON_MIME_TYPE, &ExtractionConfig::default())
        .await
        .unwrap();
    let alias = extractor
        .extract_content(
            FEATURE_COLLECTION,
            "application/vnd.geo+json",
            &ExtractionConfig::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        crate::rendering::render_plain(&canonical),
        crate::rendering::render_plain(&alias)
    );
    assert_eq!(
        canonical.metadata.additional["geojson_summary"],
        alias.metadata.additional["geojson_summary"]
    );
}

#[tokio::test]
async fn geojson_summary_validates_depth_inside_discarded_values() {
    let config = ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_nesting_depth: 3,
            max_xml_depth: 3,
            ..SecurityLimits::default()
        }),
        ..ExtractionConfig::default()
    };
    let content = br#"{"type":"Feature","properties":{"ignored":{"a":{"b":{"c":1}}}},"geometry":null}"#;

    let error = StructuredExtractor::new()
        .extract_content(content, GEOJSON_MIME_TYPE, &config)
        .await
        .expect_err("discarded property values must still consume the whole-input depth budget");
    assert!(matches!(error, crate::XbergError::Security { .. }));
}

#[tokio::test]
async fn geojson_summary_honors_iteration_security_limit() {
    let config = ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_iterations: 2,
            ..SecurityLimits::default()
        }),
        ..ExtractionConfig::default()
    };

    let error = StructuredExtractor::new()
        .extract_content(FEATURE_COLLECTION, GEOJSON_MIME_TYPE, &config)
        .await
        .expect_err("feature traversal must exhaust the configured iteration budget");
    assert!(matches!(error, crate::XbergError::Security { .. }));
}

#[tokio::test]
async fn geojson_summary_honors_input_size_security_limit() {
    let config = ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_content_size: 10,
            ..SecurityLimits::default()
        }),
        ..ExtractionConfig::default()
    };

    let error = StructuredExtractor::new()
        .extract_content(FEATURE_COLLECTION, GEOJSON_MIME_TYPE, &config)
        .await
        .expect_err("oversized GeoJSON must be rejected before parsing");
    assert!(matches!(error, crate::XbergError::Security { .. }));
}
