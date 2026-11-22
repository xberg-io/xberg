use crate::{Result, hex, identifier, require};
use nli_qualification::Split;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Committed description of one corpus. Only digests and metadata are stored here; document
/// bytes stay in a content cache that the plan builder re-verifies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusManifest {
    pub schema: u32,
    pub purpose: String,
    pub source: Value,
    pub selection: Value,
    pub counts: Value,
    pub documents: Vec<CorpusDocument>,
    pub failures: Vec<Value>,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusDocument {
    pub id: String,
    pub family: String,
    pub subfamily: String,
    pub split: Split,
    #[serde(rename = "type")]
    pub kind: String,
    pub type_code: String,
    pub known_class: bool,
    pub document_number: String,
    pub publication_date: String,
    pub agency: String,
    pub source_url: String,
    pub source_sha256: String,
    pub text_sha256: String,
    pub text_bytes: usize,
    pub removed_metadata_lines: Vec<String>,
    /// Token-bounded text: the digest and size of the full text before the model window was
    /// applied. Absent when the corpus was frozen without token truncation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_tokens: Option<usize>,
}

impl CorpusDocument {
    /// Gold labels for a document given the caller vocabulary. An unknown-class document has an
    /// empty gold set, which is how the frozen plan represents "no label in this vocabulary".
    pub fn gold(&self, labels: &[String]) -> Result<Vec<String>> {
        if self.known_class {
            require(
                labels.contains(&self.kind),
                "known-class document does not carry a vocabulary label",
            )?;
            Ok(vec![self.kind.clone()])
        } else {
            require(
                !labels.contains(&self.kind),
                "unknown-class document carries a vocabulary label",
            )?;
            Ok(Vec::new())
        }
    }
}

impl CorpusManifest {
    pub fn validate(&self) -> Result<()> {
        require(self.schema == 1, "corpus schema must be version 1")?;
        require(!self.documents.is_empty(), "corpus must not be empty")?;
        require(
            self.failures.is_empty(),
            "corpus acquisition recorded unrecovered failures",
        )?;
        require(
            crate::canonical_digest(&self.documents)? == self.digest,
            "corpus digest mismatch",
        )?;
        let mut ids = BTreeSet::new();
        let mut families: BTreeMap<&str, Split> = BTreeMap::new();
        let mut held_out = 0usize;
        for document in &self.documents {
            require(
                identifier(&document.id) && identifier(&document.family),
                "corpus identifiers must be opaque",
            )?;
            require(ids.insert(document.id.as_str()), "duplicate corpus document")?;
            require(
                hex(&document.source_sha256, 64)
                    && hex(&document.text_sha256, 64)
                    && document.text_bytes > 0
                    && !document.kind.trim().is_empty(),
                "corpus document digest, size or type is invalid",
            )?;
            if let Some(previous) = families.insert(&document.family, document.split) {
                require(previous == document.split, "corpus family spans two splits")?;
            }
            held_out += usize::from(document.split == Split::HeldOut);
        }
        require(held_out > 0, "corpus has no held-out documents")?;
        Ok(())
    }

    pub fn by_id(&self) -> BTreeMap<&str, &CorpusDocument> {
        self.documents
            .iter()
            .map(|document| (document.id.as_str(), document))
            .collect()
    }

    pub fn count(&self, split: Split) -> usize {
        self.documents.iter().filter(|document| document.split == split).count()
    }

    /// Documents whose retained text contains the literal label phrase are the honest upper bound:
    /// a classifier can read the type off the page. The complement is the cue-free sensitivity
    /// stratum, which is reported separately because it is small.
    pub fn has_lexical_cue(document: &CorpusDocument, text: &str, labels: &[String]) -> bool {
        if !document.known_class {
            return false;
        }
        let lower = text.to_lowercase();
        labels
            .iter()
            .filter(|label| **label == document.kind)
            .any(|label| lower.contains(&label.to_lowercase()))
    }
}

/// Committed description of the image-captioning corpus. Images are fetched by pinned URL; only
/// reference captions and digests are stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionManifest {
    pub schema: u32,
    pub purpose: String,
    pub source: Value,
    pub selection: Value,
    pub counts: Value,
    pub images: Vec<CaptionImage>,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionImage {
    pub id: String,
    pub image_file: String,
    pub image_url: String,
    pub image_sha256: String,
    pub image_bytes: usize,
    pub width: u32,
    pub height: u32,
    pub split: Split,
    pub stratum: String,
    pub reference_captions: Vec<String>,
    pub reference_numbers: Vec<String>,
    pub reference_terms: Vec<String>,
}

impl CaptionManifest {
    pub fn validate(&self) -> Result<()> {
        require(self.schema == 1, "caption corpus schema must be version 1")?;
        require(!self.images.is_empty(), "caption corpus must not be empty")?;
        require(
            crate::canonical_digest(&self.images)? == self.digest,
            "caption corpus digest mismatch",
        )?;
        let mut ids = BTreeSet::new();
        for image in &self.images {
            require(identifier(&image.id), "caption image identifiers must be opaque")?;
            require(ids.insert(image.id.as_str()), "duplicate caption image")?;
            require(
                hex(&image.image_sha256, 64)
                    && image.image_bytes > 0
                    && image.width > 0
                    && image.height > 0
                    && !image.image_url.trim().is_empty()
                    && !image.reference_captions.is_empty(),
                "caption image pin or reference caption is invalid",
            )?;
            require(
                image
                    .reference_captions
                    .iter()
                    .all(|caption| !caption.trim().is_empty()),
                "empty reference caption",
            )?;
        }
        require(
            self.images.iter().any(|image| image.split == Split::HeldOut),
            "caption corpus has no held-out images",
        )?;
        Ok(())
    }

    pub fn by_id(&self) -> BTreeMap<&str, &CaptionImage> {
        self.images.iter().map(|image| (image.id.as_str(), image)).collect()
    }
}
