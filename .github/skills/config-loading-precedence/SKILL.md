---
name: config-loading-precedence
description: "config loading precedence"
---

## priority: high

# Configuration Loading & Precedence

## CLI Mode Precedence (highest → lowest)

1. Individual CLI flags (`--ocr`, `--output-format`, `--chunk`)
2. Inline JSON config (`--config-json` or `--config-json-base64`)
3. Config file (`--config path.toml`)
4. Auto-discovered config (`kreuzberg.{toml,yaml,json}` in cwd/parents)
5. Default values

## Server/MCP Mode Precedence

1. CLI arguments (`--host`, `--port`)
2. Environment variables (`KREUZBERG_HOST`, `KREUZBERG_PORT`)
3. Config file `[server]` section
4. Defaults (`127.0.0.1:8000`)

## Config File Discovery

Searches current directory and parents for `kreuzberg.toml`, `kreuzberg.yaml`, or `kreuzberg.json`. Stops at first match.

## Inline JSON Config

Field-level merge (not whole-object replacement):

```rust
fn merge_json_into_config(base: &ExtractionConfig, json: Value) -> Result<ExtractionConfig> {
    let mut config_json = serde_json::to_value(base)?;
    // Merge fields from json into config_json
    serde_json::from_value(merged)?
}
```

Use `--config-json-base64` for shell escaping.

## Config File Formats

**TOML** (`kreuzberg.toml`):

```toml
use_cache = true
[ocr]
backend = "tesseract"
languages = ["eng", "deu"]
[security_limits]
max_archive_size = 524288000
```

**YAML** and **JSON** follow equivalent structure.

## CLI Flag Overrides

In `commands.rs`: `apply_extraction_overrides()` applies individual flags on top of merged config.

## Critical Rules

1. CLI flags always win over config file
2. JSON merge is field-level, not whole-object
3. Auto-discovery stops at first config file found
4. `--config-json-base64` for shell-safe JSON passing
5. Server config uses `[server]` section + extraction config
