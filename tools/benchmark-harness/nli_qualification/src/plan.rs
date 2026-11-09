use crate::{FrozenPlan, Result, Variant, digest, hex, identifier, require};
use std::collections::{BTreeMap, BTreeSet};

const MAX_INFERENCES: usize = 100_000;

impl FrozenPlan {
    pub fn validate(&self) -> Result<()> {
        self.validate_portable()?;
        #[cfg(feature = "runtime")]
        for candidate in &self.candidates {
            for variant in candidate.variants.values() {
                xberg::local_classification::LocalTextClassifier::new(variant.config.clone())?;
            }
        }
        Ok(())
    }

    /// Validate the frozen accounting contract without SDK runtime admission.
    /// Runtime-enabled `validate` additionally checks every SDK constructor.
    pub fn validate_portable(&self) -> Result<()> {
        require(
            self.schema_version == 1 && !self.qualified,
            "schema must be version 1 and unqualified",
        )?;
        require(!self.documents.is_empty(), "documents must not be empty")?;
        require(!self.candidates.is_empty(), "candidates must not be empty")?;
        require(!self.arms.is_empty(), "arms must not be empty")?;
        require(self.repeats > 0, "repeats must be positive")?;
        let count = [
            self.documents.len(),
            self.candidates.len(),
            self.arms.len(),
            self.repeats as usize,
        ]
        .into_iter()
        .try_fold(1usize, usize::checked_mul);
        require(
            count == Some(self.expected_inferences) && self.expected_inferences <= MAX_INFERENCES,
            "expected_inferences must equal documents*candidates*arms*repeats within the job limit",
        )?;
        unique(self.labels.iter().map(String::as_str), "labels")?;
        require(
            !self.labels.is_empty() && self.labels.len() <= 64,
            "labels must contain 1..64 entries",
        )?;
        unique(self.arms.iter().map(String::as_str), "arms")?;
        require(
            self.arms
                .iter()
                .all(|arm| matches!(arm.as_str(), "fp32" | "int8" | "uint8")),
            "unsupported arm",
        )?;
        self.validate_provenance()?;
        self.validate_documents()?;
        self.validate_candidates()
    }

    fn validate_provenance(&self) -> Result<()> {
        require(
            hex(&self.provenance.corpus_receipt_sha256, 64) && hex(&self.provenance.split_audit_sha256, 64),
            "provenance receipts require SHA-256",
        )?;
        let runtime = &self.runtime;
        require(
            hex(&runtime.runner_sha256, 64)
                && hex(&runtime.dependency_lock_sha256, 64)
                && !runtime.ort_build_info.trim().is_empty()
                && !runtime.os.trim().is_empty()
                && !runtime.arch.trim().is_empty()
                && !runtime.cpu.trim().is_empty()
                && runtime.threads == 0
                && runtime.deadline_ms > 0,
            "runtime requires pinned identity, SDK-default threads (0) and a positive deadline",
        )
    }

    fn validate_documents(&self) -> Result<()> {
        unique(self.documents.iter().map(|doc| doc.id.as_str()), "document IDs")?;
        let mut groups = BTreeMap::new();
        for doc in &self.documents {
            require(
                identifier(&doc.id) && identifier(&doc.family),
                "document and family identifiers must be opaque",
            )?;
            require(
                !doc.text.trim().is_empty()
                    && doc.text.len() <= 65_536
                    && digest(doc.text.as_bytes()) == doc.text_sha256
                    && hex(&doc.source_sha256, 64),
                "document text digest, source digest or input size is invalid",
            )?;
            for key in [
                format!("family:{}", doc.family.to_ascii_lowercase()),
                format!("source:{}", doc.source_sha256),
                format!("text:{}", doc.text_sha256),
            ] {
                if let Some((split, family)) = groups.insert(key, (doc.split, doc.family.to_ascii_lowercase())) {
                    require(
                        split == doc.split && family == doc.family.to_ascii_lowercase(),
                        "split/family leakage",
                    )?;
                }
            }
            unique(doc.gold.iter().map(String::as_str), "gold labels")?;
            require(
                doc.gold.iter().all(|label| self.labels.contains(label)),
                "unknown gold label",
            )?;
        }
        Ok(())
    }

    fn validate_candidates(&self) -> Result<()> {
        unique(
            self.candidates.iter().map(|candidate| candidate.id.as_str()),
            "candidate IDs",
        )?;
        let arms: BTreeSet<_> = self.arms.iter().collect();
        for candidate in &self.candidates {
            require(identifier(&candidate.id), "invalid candidate ID")?;
            require(
                candidate.variants.keys().collect::<BTreeSet<_>>() == arms,
                "candidate arms do not match plan",
            )?;
            for variant in candidate.variants.values() {
                variant.validate()?;
                crate::config::validate_requests(self, &variant.config)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn unique<'a>(values: impl Iterator<Item = &'a str>, name: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        require(
            !value.trim().is_empty() && value == value.trim() && seen.insert(value.to_ascii_lowercase()),
            &format!("empty or duplicate {name}"),
        )?;
    }
    Ok(())
}

impl Variant {
    fn validate(&self) -> Result<()> {
        require(
            hex(&self.source_revision, 40)
                && hex(&self.export_recipe_sha256, 64)
                && hex(&self.license_receipt_sha256, 64),
            "candidate provenance is not pinned",
        )?;
        let config = &self.config;
        require(
            hex(&config.model.revision, 40)
                && config.model.repository.split('/').count() == 2
                && config
                    .model
                    .repository
                    .split('/')
                    .all(crate::config::repository_component),
            "model revision or repository is invalid",
        )?;
        require(
            config.hypothesis_template.matches("{label}").count() == 1
                && config.confidence_threshold.is_finite()
                && (0.0..=1.0).contains(&config.confidence_threshold),
            "invalid NLI template or threshold",
        )?;
        require(
            config.model.entailment_index < 3
                && config.model.contradiction_index < 3
                && config.model.entailment_index != config.model.contradiction_index,
            "invalid three-way NLI indices",
        )?;
        self.graph_sha256()?;
        crate::config::validate_config(config)?;
        Ok(())
    }

    pub(crate) fn graph_sha256(&self) -> Result<String> {
        let model = &self.config.model;
        let mut pins = BTreeMap::new();
        let mut names = BTreeSet::new();
        for line in model.sha256_manifest.lines().filter(|line| !line.trim().is_empty()) {
            let fields: Vec<_> = line.split_whitespace().collect();
            require(fields.len() == 2 && hex(fields[0], 64), "invalid artifact manifest pin")?;
            let path = fields[1];
            require(
                crate::config::artifact_path(path)
                    && names.insert(path.to_ascii_lowercase())
                    && pins.insert(path.to_owned(), fields[0].to_owned()).is_none(),
                "invalid or duplicate artifact path",
            )?;
        }
        let graph = pins.remove(&model.model_file).ok_or("artifact manifest lacks graph")?;
        for path in &model.external_files {
            require(pins.remove(path).is_some(), "missing external artifact pin")?;
        }
        for name in [
            "tokenizer.json",
            "config.json",
            "tokenizer_config.json",
            "special_tokens_map.json",
        ] {
            let nested = model
                .model_file
                .rsplit_once('/')
                .map(|(dir, _)| format!("{dir}/{name}"));
            let matches: Vec<_> = pins
                .keys()
                .filter(|path| path.as_str() == name || Some(*path) == nested.as_ref())
                .cloned()
                .collect();
            require(matches.len() == 1, "missing or ambiguous companion artifact pin")?;
            pins.remove(&matches[0]);
        }
        require(pins.is_empty(), "unexpected artifact pin")?;
        Ok(graph)
    }
}
