use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::quality::geometry::{Region, validate_region};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationManifest {
    pub schema_version: u32,
    pub source: String,
    pub coco_file: PathBuf,
    pub coco_sha256: String,
    pub category_mapping: BTreeMap<u64, String>,
    pub prediction_projection: BTreeMap<String, String>,
    pub pages: Vec<PageBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PageBinding {
    pub document: String,
    pub page: usize,
    pub image_id: u64,
    pub render_sha256: String,
    pub width: u32,
    pub height: u32,
    pub expected_annotations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnnotatedRegion {
    pub annotation_id: u64,
    pub source_category_id: u64,
    pub region: Region,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnnotatedPage {
    pub binding: PageBinding,
    pub regions: Vec<AnnotatedRegion>,
}

#[derive(Deserialize)]
pub struct Coco {
    images: Vec<CocoImage>,
    categories: Vec<CocoCategory>,
    annotations: Vec<CocoAnnotation>,
}

#[derive(Deserialize)]
struct CocoImage {
    id: u64,
    width: u32,
    height: u32,
}
#[derive(Deserialize)]
struct CocoCategory {
    id: u64,
    name: String,
}
#[derive(Deserialize)]
struct CocoAnnotation {
    id: u64,
    image_id: u64,
    category_id: u64,
    bbox: [f64; 4],
    #[serde(default)]
    iscrowd: u8,
    #[serde(default)]
    ignore: u8,
}

pub fn prepare(manifest: &AnnotationManifest, coco: Coco) -> Result<Vec<AnnotatedPage>, String> {
    if manifest.schema_version != 1 || manifest.source.trim().is_empty() || manifest.pages.is_empty() {
        return Err("invalid or empty geometry annotation manifest".into());
    }
    let categories = validate_categories(&coco.categories, &manifest.category_mapping)?;
    let mut images = BTreeMap::new();
    for image in &coco.images {
        if image.width == 0 || image.height == 0 || images.insert(image.id, image).is_some() {
            return Err("invalid or duplicate COCO image".into());
        }
    }
    let annotations = validate_annotations(&coco.annotations, &images, &categories)?;
    let mut identities = BTreeSet::new();
    let mut selected = BTreeSet::new();
    manifest
        .pages
        .iter()
        .map(|binding| {
            if binding.document.is_empty()
                || binding.page == 0
                || !identities.insert((&binding.document, binding.page))
                || !selected.insert(binding.image_id)
            {
                return Err("invalid or duplicate geometry page binding".into());
            }
            let image = images
                .get(&binding.image_id)
                .ok_or("geometry binding references missing image")?;
            if (binding.width, binding.height) != (image.width, image.height) {
                return Err("geometry annotation dimensions mismatch".into());
            }
            let regions = annotations.get(&binding.image_id).cloned().unwrap_or_default();
            if regions.len() != binding.expected_annotations {
                return Err("geometry annotation count mismatch".into());
            }
            Ok(AnnotatedPage {
                binding: binding.clone(),
                regions,
            })
        })
        .collect()
}

fn validate_categories(
    categories: &[CocoCategory],
    mapping: &BTreeMap<u64, String>,
) -> Result<BTreeMap<u64, String>, String> {
    let mut names = BTreeSet::new();
    let mut identifiers = BTreeSet::new();
    for category in categories {
        if category.name.trim().is_empty() || !names.insert(&category.name) || !identifiers.insert(category.id) {
            return Err("invalid or duplicate COCO category".into());
        }
    }
    if identifiers.is_empty()
        || identifiers != mapping.keys().copied().collect()
        || mapping.values().any(|name| name.trim().is_empty())
    {
        return Err("geometry mapping must explicitly cover every source category".into());
    }
    Ok(mapping.clone())
}

fn validate_annotations(
    annotations: &[CocoAnnotation],
    images: &BTreeMap<u64, &CocoImage>,
    mapping: &BTreeMap<u64, String>,
) -> Result<BTreeMap<u64, Vec<AnnotatedRegion>>, String> {
    let mut identifiers = BTreeSet::new();
    let mut pages: BTreeMap<u64, Vec<AnnotatedRegion>> = BTreeMap::new();
    for annotation in annotations {
        if !identifiers.insert(annotation.id) {
            return Err("duplicate COCO annotation ID".into());
        }
        if annotation.iscrowd != 0 || annotation.ignore != 0 {
            return Err("crowd and ignored COCO annotations are unsupported".into());
        }
        let image = images
            .get(&annotation.image_id)
            .ok_or("COCO annotation references missing image")?;
        let class = mapping
            .get(&annotation.category_id)
            .ok_or("COCO annotation references unmapped category")?;
        let [x, y, width, height] = annotation.bbox;
        let region = Region {
            class_name: class.clone(),
            bbox: [x, y, x + width, y + height],
        };
        validate_region(&region, image.width, image.height)?;
        pages.entry(annotation.image_id).or_default().push(AnnotatedRegion {
            annotation_id: annotation.id,
            source_category_id: annotation.category_id,
            region,
        });
    }
    Ok(pages)
}

pub fn decode(manifest: &AnnotationManifest, bytes: &[u8]) -> Result<Vec<AnnotatedPage>, String> {
    if digest(bytes) != manifest.coco_sha256 {
        return Err("geometry COCO hash mismatch".into());
    }
    let coco: Coco = serde_json::from_slice(bytes).map_err(|error| format!("invalid COCO JSON: {error}"))?;
    prepare(manifest, coco)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedAnnotations {
    pub manifest_json: String,
    pub coco_json: String,
}

impl RetainedAnnotations {
    pub fn decode(
        &self,
        manifest_hash: &str,
        coco_hash: &str,
    ) -> Result<(AnnotationManifest, Vec<AnnotatedPage>), String> {
        const MAX_BYTES: usize = 64 * 1024 * 1024;
        if self.manifest_json.len() > MAX_BYTES || self.coco_json.len() > MAX_BYTES {
            return Err("geometry annotation file exceeds 64 MiB limit".into());
        }
        if digest(self.manifest_json.as_bytes()) != manifest_hash {
            return Err("geometry manifest hash mismatch".into());
        }
        if digest(self.coco_json.as_bytes()) != coco_hash {
            return Err("geometry COCO hash mismatch".into());
        }
        let manifest: AnnotationManifest = serde_json::from_str(&self.manifest_json)
            .map_err(|error| format!("invalid geometry manifest JSON: {error}"))?;
        let pages = decode(&manifest, self.coco_json.as_bytes())?;
        Ok((manifest, pages))
    }
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
