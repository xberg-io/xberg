//! In-memory registry of curated presets, embedded at compile time.
//!
//! The embedded library ships only the `generic_document` toy preset. Downstream
//! catalog consumers can add presets at runtime via [`Registry::extend_from_dir`].

use std::collections::BTreeMap;
use std::sync::OnceLock;

use include_dir::{Dir, include_dir};

use crate::presets::loader::{LoadError, MetaSchema};
use crate::presets::types::{Preset, PresetSummary};

static LIBRARY: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/presets/library");
static META_SCHEMA: &str = include_str!("preset.schema.json");
static GLOBAL: OnceLock<Registry> = OnceLock::new();

/// Sorted map of preset id → [`Preset`].
#[derive(Debug, Clone)]
pub struct Registry {
    by_id: BTreeMap<String, Preset>,
}

impl Registry {
    /// Build the registry from preset files embedded at compile time under
    /// `src/presets/library/`. Validates every file against the meta-schema.
    pub fn load_embedded() -> Result<Self, LoadError> {
        let meta = MetaSchema::compile(META_SCHEMA)?;
        let mut by_id = BTreeMap::new();
        for file in walk_json(&LIBRARY) {
            let path = file.path().to_string_lossy().to_string();
            let preset = meta.parse_preset(&path, file.contents())?;
            let expected_stem = preset_stem(&path);
            if preset.id != expected_stem {
                return Err(LoadError::IdMismatch {
                    path,
                    declared: preset.id.clone(),
                    expected: expected_stem,
                });
            }
            by_id.insert(preset.id.clone(), preset);
        }
        Ok(Self { by_id })
    }

    /// Return the global registry, loading it on first access.
    ///
    /// # Panics
    ///
    /// Panics if any embedded preset is malformed. The build-time validation
    /// test ensures this cannot happen for the embedded presets; a panic here
    /// indicates a build artifact problem, not a runtime error.
    pub fn global() -> &'static Registry {
        GLOBAL.get_or_init(|| Registry::load_embedded().expect("embedded presets must validate against meta-schema"))
    }

    /// Look up a preset by its identifier.
    pub fn get(&self, id: &str) -> Option<&Preset> {
        self.by_id.get(id)
    }

    /// Iterate over presets sorted by id.
    #[cfg_attr(alef, alef(skip))]
    pub fn iter(&self) -> impl Iterator<Item = &Preset> {
        self.by_id.values()
    }

    /// Materialize a [`PresetSummary`] list for the public registry endpoint.
    pub fn summaries(&self) -> Vec<PresetSummary> {
        self.iter().map(PresetSummary::from).collect()
    }

    /// Number of presets currently loaded.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether the registry contains zero presets.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Read raw sample bytes for `<preset_id>` from
    /// `library/<id>/samples/<name>`. Returns `None` when the file is absent.
    pub fn sample_bytes(&self, preset_id: &str, name: &str) -> Option<&'static [u8]> {
        let path = format!("{preset_id}/samples/{name}");
        LIBRARY.get_file(&path).map(|f| f.contents())
    }

    /// Load additional preset files from a runtime directory and insert them
    /// into this registry.
    ///
    /// Reads every `*.json` file directly under `dir` (non-recursive),
    /// validates each against the meta-schema, and inserts it. Files that fail
    /// validation are rejected — the error is returned immediately and the
    /// registry is left in a partially-updated state. Existing entries with the
    /// same id are overwritten.
    ///
    /// Returns the number of presets successfully loaded from `dir`.
    ///
    /// # Use case
    ///
    /// This is the injection point for downstream catalogs that add curated
    /// presets on top of the single embedded OSS preset.
    pub fn extend_from_dir(&mut self, dir: &std::path::Path) -> Result<usize, LoadError> {
        let meta = MetaSchema::compile(META_SCHEMA)?;
        let mut count = 0usize;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let path_str = path.to_string_lossy().to_string();
            let raw = std::fs::read(&path)?;
            let preset = meta.parse_preset(&path_str, &raw)?;
            self.by_id.insert(preset.id.clone(), preset);
            count += 1;
        }
        Ok(count)
    }
}

fn walk_json<'d>(dir: &'d Dir<'d>) -> Vec<&'d include_dir::File<'d>> {
    let mut out = Vec::new();
    collect(dir, &mut out);
    out
}

fn collect<'d>(dir: &'d Dir<'d>, out: &mut Vec<&'d include_dir::File<'d>>) {
    for f in dir.files() {
        if f.path().extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(f);
        }
    }
    for sub in dir.dirs() {
        // Skip per-preset `samples/` subdirs — those carry sample document
        // outputs (and other artifacts), not preset definitions.
        if sub.path().file_name().and_then(|n| n.to_str()) == Some("samples") {
            continue;
        }
        collect(sub, out);
    }
}

/// `library/invoice/v1.json` → `invoice`. `library/invoice.json` → `invoice`.
fn preset_stem(path: &str) -> String {
    let p = std::path::Path::new(path);
    if let Some(parent) = p.parent()
        && let Some(stem) = parent.file_name().and_then(|n| n.to_str())
        && !stem.is_empty()
    {
        return stem.to_string();
    }
    p.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string()
}
