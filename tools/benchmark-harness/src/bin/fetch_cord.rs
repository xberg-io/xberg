use anyhow::{Context, Result, anyhow};
use base64::Engine;
use clap::Parser;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "fetch_cord")]
#[command(about = "Fetch CORD v2 test rows from HuggingFace and write to disk")]
struct Args {
    /// Dataset root directory (defaults to ~/.kreuzberg/datasets)
    #[arg(long)]
    root: Option<PathBuf>,

    /// Number of rows to fetch (default: 20)
    #[arg(long, default_value = "20")]
    limit: usize,

    /// Offset in the dataset (default: 0)
    #[arg(long, default_value = "0")]
    offset: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Resolve dataset root
    let root = if let Some(r) = args.root {
        r
    } else {
        let home = std::env::var("HOME").context("HOME env var not set")?;
        Path::new(&home).join(".kreuzberg/datasets")
    };

    println!(
        "Fetching {} CORD rows from offset {} to {}",
        args.limit,
        args.offset,
        root.display()
    );

    // Create directory structure
    let cord_test_dir = root.join("CORD").join("test");
    let manifests_dir = root.join("manifests");
    fs::create_dir_all(&cord_test_dir).context(format!("Failed to create {}", cord_test_dir.display()))?;
    fs::create_dir_all(&manifests_dir).context(format!("Failed to create {}", manifests_dir.display()))?;

    // Fetch from HuggingFace datasets-server API
    let url = format!(
        "https://datasets-server.huggingface.co/rows?dataset=naver-clova-ix/cord-v2&config=default&split=test&offset={}&length={}",
        args.offset, args.limit
    );

    println!("Fetching from: {}", url);

    // Build async client for HF API fetch
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch CORD rows from HuggingFace")?;

    let body = response.text().await.context("Failed to read response body")?;

    let api_response: Value = serde_json::from_str(&body).context("Failed to parse HuggingFace API response")?;

    // Extract rows, handling the nested structure from datasets-server
    let rows_data = api_response
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("No 'rows' array in API response"))?;

    // Extract the actual row data (API wraps rows in {"row_idx": N, "row": {...}})
    let rows: Vec<Value> = rows_data.iter().filter_map(|r| r.get("row").cloned()).collect();

    if rows.is_empty() {
        return Err(anyhow!("No rows returned from HuggingFace API"));
    }

    println!("Fetched {} rows", rows.len());

    // First row for schema extraction
    let first_row = &rows[0];
    let cord_schema = extract_schema_from_row(first_row)?;

    // Write schema to datasets/schemas/cord.json
    let schemas_dir = root.join("datasets").join("schemas");
    fs::create_dir_all(&schemas_dir).context(format!("Failed to create {}", schemas_dir.display()))?;
    let schema_path = schemas_dir.join("cord.json");
    fs::write(&schema_path, serde_json::to_string_pretty(&cord_schema)?)
        .context(format!("Failed to write schema to {}", schema_path.display()))?;
    println!("Wrote schema to {}", schema_path.display());

    // Process rows and write fixtures
    let mut manifest_lines = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let index = args.offset + i;

        // Extract image and ground truth
        let image_data = extract_image_data(row, &client)
            .await
            .context(format!("Failed to extract image from row {}", i))?;
        let ground_truth =
            extract_ground_truth(row).context(format!("Failed to extract ground truth from row {}", i))?;

        // Write image
        let image_filename = format!("cord_{}.png", index);
        let image_path = cord_test_dir.join(&image_filename);
        fs::write(&image_path, &image_data).context(format!("Failed to write image to {}", image_path.display()))?;

        // Write ground truth JSON
        let gt_filename = format!("cord_{}.json", index);
        let gt_path = cord_test_dir.join(&gt_filename);
        fs::write(&gt_path, serde_json::to_string_pretty(&ground_truth)?)
            .context(format!("Failed to write GT to {}", gt_path.display()))?;

        manifest_lines.push(format!("{}, {}", image_filename, gt_filename));

        if (i + 1) % 5 == 0 {
            println!("  Processed {} / {} rows", i + 1, rows.len());
        }

        // Rate limiting: one per second
        if i + 1 < rows.len() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Write manifest
    let manifest_path = manifests_dir.join("cord.toml");
    let manifest_content = manifest_lines.join("\n");
    fs::write(&manifest_path, manifest_content)
        .context(format!("Failed to write manifest to {}", manifest_path.display()))?;
    println!(
        "Wrote manifest with {} entries to {}",
        rows.len(),
        manifest_path.display()
    );

    println!("Done! Fetched {} CORD fixtures", rows.len());
    Ok(())
}

/// Extract image data from a HuggingFace CORD row.
/// The image.src field is either a base64 data URI or a URL.
/// This is called from an async context, so we can use async reqwest.
async fn extract_image_data(row: &Value, client: &reqwest::Client) -> Result<Vec<u8>> {
    let image_obj = row
        .get("image")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("No image object in row"))?;

    let src = image_obj
        .get("src")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No image.src in row"))?;

    // Check if it's a data URI
    if src.starts_with("data:image") {
        // Parse data URI: "data:image/png;base64,<data>"
        let parts: Vec<&str> = src.split(',').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid data URI format"));
        }
        let data_str = parts[1];
        let decoded = base64_decode(data_str).context("Failed to decode base64 image data")?;
        Ok(decoded)
    } else if src.starts_with("http") {
        // It's a URL — fetch it async
        let image_bytes = client
            .get(src)
            .send()
            .await
            .context(format!("Failed to fetch image from {}", src))?
            .bytes()
            .await
            .context("Failed to read image bytes")?
            .to_vec();

        Ok(image_bytes)
    } else {
        Err(anyhow!("Unexpected image.src format: {}", src))
    }
}

/// Extract ground truth from a HuggingFace CORD row.
/// The ground_truth field is a JSON string; we extract the gt_parse field.
fn extract_ground_truth(row: &Value) -> Result<Value> {
    let gt_str = row
        .get("ground_truth")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No ground_truth string in row"))?;

    let gt_obj: Value = serde_json::from_str(gt_str).context("Failed to parse ground_truth JSON")?;

    let gt_parse = gt_obj
        .get("gt_parse")
        .ok_or_else(|| anyhow!("No gt_parse field in ground_truth"))?
        .clone();

    Ok(gt_parse)
}

/// Decode a base64 string.
fn base64_decode(s: &str) -> Result<Vec<u8>> {
    let engine = base64::engine::general_purpose::STANDARD;
    engine.decode(s).context("Failed to decode base64")
}

/// Extract CORD schema from the first row's ground truth.
/// This creates a draft-07 schema covering the top-level fields in gt_parse.
fn extract_schema_from_row(row: &Value) -> Result<Value> {
    let gt_str = row
        .get("ground_truth")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No ground_truth in row"))?;

    let gt_obj: Value = serde_json::from_str(gt_str).context("Failed to parse ground_truth JSON")?;

    let gt_parse = gt_obj
        .get("gt_parse")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("No gt_parse object in ground_truth"))?;

    // Extract field names and infer types
    let mut properties = serde_json::Map::new();

    for (key, value) in gt_parse.iter() {
        let prop_schema = match value {
            Value::Null => json!({"type": ["null"]}),
            Value::Bool(_) => json!({"type": ["boolean", "null"]}),
            Value::Number(_) => json!({"type": ["number", "null"]}),
            Value::String(_) => json!({"type": ["string", "null"]}),
            Value::Array(arr) => {
                // For arrays, infer item type from first element
                let item_schema = if let Some(first) = arr.first() {
                    match first {
                        Value::Object(_) => json!({"type": "object"}),
                        Value::String(_) => json!({"type": "string"}),
                        Value::Number(_) => json!({"type": "number"}),
                        _ => json!({}),
                    }
                } else {
                    json!({})
                };
                json!({"type": ["array", "null"], "items": item_schema})
            }
            Value::Object(_) => json!({"type": ["object", "null"]}),
        };
        properties.insert(key.clone(), prop_schema);
    }

    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": properties,
        "additionalProperties": true
    });

    Ok(schema)
}
