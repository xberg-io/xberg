//! The only code that loads a model. Both plan builders derive descriptors from the canonical
//! inventory and both runners call the SDK; nothing here uses a Python runtime or a different
//! decoding path.

use crate::caption::{
    CaptionArm, CaptionPlan, CaptionPlanCandidate, CaptionPlanImage, CaptionProvenance, CaptionReport, CaptionRow,
};
use crate::corpus::{CaptionManifest, CorpusManifest};
use crate::prereg::Preregistration;
use crate::{Result, digest, require};
use nli_qualification::{FrozenPlan, RuntimeIdentity};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub fn execute(args: &[String]) -> Result<()> {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["identity", deadline] => {
            let runtime = nli_qualification::runner::identity(deadline.parse()?)?;
            println!("{}", serde_json::to_string_pretty(&runtime)?);
            Ok(())
        }
        ["plan-classification", manifest, audit, texts, inventory, identity, prereg, output] => {
            plan_classification(&ClassificationPlanFiles {
                manifest: Path::new(manifest),
                audit: Path::new(audit),
                texts: Path::new(texts),
                inventory: Path::new(inventory),
                identity: Path::new(identity),
                prereg: Path::new(prereg),
                output: Path::new(output),
            })
        }
        ["plan-captioning", manifest, audit, inventory, identity, prereg, output] => plan_captioning(
            Path::new(manifest),
            Path::new(audit),
            Path::new(inventory),
            Path::new(identity),
            Path::new(prereg),
            Path::new(output),
        ),
        ["truncate", manifest, texts, tokenizer, output, out_manifest, budget] => truncate_corpus(
            Path::new(manifest),
            Path::new(texts),
            Path::new(tokenizer),
            Path::new(output),
            Path::new(out_manifest),
            budget.parse()?,
        ),
        ["run-classification", plan, sha256, output] => run_classification(Path::new(plan), sha256, Path::new(output)),
        ["run-captioning", plan, sha256, images, output] => {
            run_captioning(Path::new(plan), sha256, Path::new(images), Path::new(output))
        }
        _ => Err(concat!(
            "usage: identity DEADLINE_MS | truncate MANIFEST TEXTS TOKENIZER OUT_TEXTS OUT_MANIFEST MAX_TOKENS | plan-classification MANIFEST AUDIT TEXTS INVENTORY IDENTITY PREREG OUT | ",
            "plan-captioning MANIFEST AUDIT INVENTORY IDENTITY PREREG OUT | ",
            "run-classification PLAN SHA OUT | run-captioning PLAN SHA IMAGES OUT"
        )
        .into()),
    }
}

/// Inputs of one classification plan build. Seven paths do not belong in a positional signature:
/// mixing up the audit and identity files would silently change what the plan pins.
pub struct ClassificationPlanFiles<'a> {
    pub manifest: &'a Path,
    pub audit: &'a Path,
    pub texts: &'a Path,
    pub inventory: &'a Path,
    pub identity: &'a Path,
    pub prereg: &'a Path,
    pub output: &'a Path,
}

pub fn plan_classification(files: &ClassificationPlanFiles) -> Result<()> {
    let manifest_bytes = std::fs::read(files.manifest)?;
    let manifest: CorpusManifest = crate::files::parse_json(&manifest_bytes)?;
    manifest.validate()?;
    let prereg: Preregistration = crate::files::read_json(files.prereg)?;
    prereg.validate()?;
    let settings = prereg
        .classification
        .as_ref()
        .ok_or("preregistration declares no classification run")?;
    let pinned = prereg
        .corpora
        .get("classification")
        .ok_or("preregistration pins no classification corpus")?;
    require(
        digest(&manifest_bytes) == pinned.manifest_sha256 && manifest.digest == pinned.digest,
        "manifest differs from the preregistered corpus",
    )?;
    let runtime: RuntimeIdentity = crate::files::read_json(files.identity)?;
    let inventory = Inventory::load(files.inventory)?;
    let documents = classification_documents(&manifest, files.texts, settings)?;
    let candidates = classification_candidates(&inventory, settings)?;

    let total = documents
        .len()
        .checked_mul(candidates.len())
        .and_then(|value| value.checked_mul(settings.arms.len()))
        .and_then(|value| value.checked_mul(settings.repeats as usize))
        .ok_or("classification job count overflows")?;
    require(
        total == settings.expected.total,
        "planned classification work differs from the preregistered count",
    )?;
    let plan: FrozenPlan = serde_json::from_value(json!({
        "schema_version": 1,
        "qualified": false,
        "expected_inferences": total,
        "repeats": settings.repeats,
        "labels": settings.labels.clone(),
        "arms": settings.arms.clone(),
        "provenance": {
            "corpus_receipt_sha256": digest(&manifest_bytes),
            "split_audit_sha256": digest(&std::fs::read(files.audit)?),
        },
        "runtime": runtime,
        "documents": documents,
        "candidates": candidates,
    }))?;
    plan.validate()?;
    crate::files::write_new(files.output, &plan)?;
    println!("{}", plan.sha256()?);
    Ok(())
}

fn classification_documents(
    manifest: &CorpusManifest,
    texts: &Path,
    settings: &crate::prereg::ClassificationPrereg,
) -> Result<Vec<Value>> {
    let mut documents = Vec::new();
    for document in &manifest.documents {
        if !settings
            .scored_splits
            .iter()
            .any(|split| split == split_name(document.split))
        {
            continue;
        }
        let text_path = texts.join(format!("{}.txt", document.id));
        let text =
            std::fs::read_to_string(&text_path).map_err(|error| format!("read {}: {error}", text_path.display()))?;
        require(
            digest(text.as_bytes()) == document.text_sha256,
            &format!("text for {} differs from its committed digest", document.id),
        )?;
        documents.push(json!({
            "id": document.id,
            "family": document.family,
            "split": document.split,
            "source_sha256": document.source_sha256,
            "text": text,
            "text_sha256": document.text_sha256,
            "gold": document.gold(&settings.labels)?,
        }));
    }
    require(!documents.is_empty(), "no document matched the scored splits")?;
    Ok(documents)
}

fn classification_candidates(
    inventory: &Inventory,
    settings: &crate::prereg::ClassificationPrereg,
) -> Result<Vec<Value>> {
    let mut candidates = Vec::new();
    for candidate_id in &settings.candidate_ids {
        let model = inventory.classification_descriptor(candidate_id)?;
        let entry = inventory.entry_with_receipts(candidate_id)?;
        let mut variants = BTreeMap::new();
        for arm in &settings.arms {
            variants.insert(
                arm.clone(),
                json!({
                    "source_revision": entry["revision"],
                    "export_recipe_sha256": entry["export_recipe_sha256"],
                    "license_receipt_sha256": entry["license_receipt_sha256"],
                    "config": {
                        "model": model,
                        "hypothesis_template": settings.hypothesis_template.clone(),
                        "multi_label": false,
                        "confidence_threshold": settings.confidence_threshold,
                        "cache_directory": std::env::var("XBERG_QUALIFICATION_CACHE").ok(),
                    }
                }),
            );
        }
        candidates.push(json!({"id": candidate_id, "variants": variants}));
    }
    Ok(candidates)
}

pub fn plan_captioning(
    manifest_path: &Path,
    audit_path: &Path,
    inventory_path: &Path,
    identity_path: &Path,
    prereg_path: &Path,
    output: &Path,
) -> Result<()> {
    let manifest_bytes = std::fs::read(manifest_path)?;
    let manifest: CaptionManifest = crate::files::parse_json(&manifest_bytes)?;
    manifest.validate()?;
    let prereg: Preregistration = crate::files::read_json(prereg_path)?;
    prereg.validate()?;
    let settings = prereg
        .captioning
        .as_ref()
        .ok_or("preregistration declares no captioning run")?;
    let pinned = prereg
        .corpora
        .get("captioning")
        .ok_or("preregistration pins no captioning corpus")?;
    require(
        digest(&manifest_bytes) == pinned.manifest_sha256 && manifest.digest == pinned.digest,
        "caption manifest differs from the preregistered corpus",
    )?;
    let runtime: RuntimeIdentity = crate::files::read_json(identity_path)?;
    let inventory = Inventory::load(inventory_path)?;
    let images: Vec<CaptionPlanImage> = manifest
        .images
        .iter()
        .filter(|image| image.split == nli_qualification::Split::HeldOut)
        .map(|image| CaptionPlanImage {
            id: image.id.clone(),
            image_file: image.image_file.clone(),
            image_sha256: image.image_sha256.clone(),
            image_bytes: image.image_bytes,
        })
        .collect();
    require(!images.is_empty(), "caption corpus has no held-out images")?;
    let candidates = caption_candidates(&prereg, &inventory, settings)?;

    let total = images
        .len()
        .checked_mul(candidates.len())
        .and_then(|value| value.checked_mul(settings.arms.len()))
        .and_then(|value| value.checked_mul(settings.repeats as usize))
        .ok_or("caption job count overflows")?;
    require(
        total == settings.expected_total,
        "planned caption work differs from the preregistered count",
    )?;
    let plan = CaptionPlan {
        schema: 1,
        qualified: false,
        expected_generations: total,
        repeats: settings.repeats,
        arms: settings.arms.clone(),
        provenance: CaptionProvenance {
            corpus_receipt_sha256: digest(&manifest_bytes),
            split_audit_sha256: digest(&std::fs::read(audit_path)?),
        },
        runtime,
        images,
        candidates,
    };
    plan.validate()?;
    crate::files::write_new(output, &plan)?;
    println!("{}", plan.sha256()?);
    Ok(())
}

fn caption_candidates(
    prereg: &Preregistration,
    inventory: &Inventory,
    settings: &crate::prereg::CaptioningPrereg,
) -> Result<Vec<CaptionPlanCandidate>> {
    let mut candidates = Vec::new();
    for candidate_id in &settings.candidate_ids {
        let pin = prereg
            .caption_models()
            .find(|model| model.id == *candidate_id)
            .ok_or_else(|| format!("{candidate_id} has no captioning model pin"))?;
        let mut arms = BTreeMap::new();
        for arm in &settings.arms {
            let precision = match arm.as_str() {
                "fp32" => "fp32",
                "int8" | "uint8" => "quantized",
                other => return Err(format!("unsupported captioning arm {other}").into()),
            };
            let expected_graph = pin
                .arms
                .get(arm)
                .ok_or_else(|| format!("{candidate_id} / {arm} has no preregistered graph"))?;
            require(
                inventory.captioning_graph(candidate_id, precision)? == *expected_graph,
                "inventory graph for this candidate and arm differs from the preregistered pin",
            )?;
            arms.insert(
                arm.clone(),
                CaptionArm {
                    precision: precision.into(),
                },
            );
        }
        candidates.push(CaptionPlanCandidate {
            id: candidate_id.clone(),
            arms,
        });
    }
    Ok(candidates)
}

/// Rewrite the corpus text so every premise fits the adapter's token window. The adapter refuses an
/// over-long premise instead of truncating, so a caller that wants long documents has to decide the
/// cut itself. Full-text digests are preserved on each document.
pub fn truncate_corpus(
    manifest_path: &Path,
    texts: &Path,
    tokenizer_path: &Path,
    out_texts: &Path,
    out_manifest: &Path,
    max_tokens: usize,
) -> Result<()> {
    let manifest_bytes = std::fs::read(manifest_path)?;
    let mut manifest: CorpusManifest = crate::files::parse_json(&manifest_bytes)?;
    manifest.validate()?;
    let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
        .map_err(|error| format!("load tokenizer {}: {error}", tokenizer_path.display()))?;
    // Leave room for the hypothesis and the pair's special tokens; the margin absorbs a
    // decode/re-encode round trip.
    let hypothesis_tokens = manifest
        .documents
        .iter()
        .filter(|document| document.known_class)
        .map(|document| {
            tokenizer
                .encode(format!("This document is a {}.", document.kind), false)
                .map(|encoding| encoding.len())
        })
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let budget = max_tokens
        .checked_sub(hypothesis_tokens)
        .and_then(|value| value.checked_sub(12))
        .ok_or("token budget is smaller than the hypothesis")?;
    std::fs::create_dir_all(out_texts)?;
    for document in &mut manifest.documents {
        let path = texts.join(format!("{}.txt", document.id));
        let text = std::fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
        require(
            digest(text.as_bytes()) == document.text_sha256,
            "source text differs from its committed digest",
        )?;
        document.full_text_sha256 = Some(document.text_sha256.clone());
        document.full_text_bytes = Some(text.len());
        let mut encoding = tokenizer.encode(text.as_str(), false)?.get_ids().to_vec();
        let mut kept = encoding.len().min(budget);
        let bounded = loop {
            let candidate = tokenizer
                .decode(&encoding[..kept], true)
                .map_err(|error| format!("decode {}: {error}", document.id))?;
            encoding = tokenizer.encode(candidate.as_str(), false)?.get_ids().to_vec();
            if encoding.len() <= budget {
                break candidate;
            }
            kept = budget.saturating_sub(8).min(encoding.len().saturating_sub(1));
            require(kept > 0, "document cannot be reduced to the token budget")?;
        };
        let target = out_texts.join(format!("{}.txt", document.id));
        std::fs::write(&target, &bounded)?;
        document.text_sha256 = digest(bounded.as_bytes());
        document.text_bytes = bounded.len();
        document.text_tokens = Some(
            tokenizer
                .encode(bounded.as_str(), false)
                .map(|encoding| encoding.len())?,
        );
    }
    manifest.digest = crate::canonical_digest(&manifest.documents)?;
    crate::files::write_new(out_manifest, &manifest)?;
    println!("{}", manifest.digest);
    Ok(())
}

pub fn run_classification(plan_path: &Path, sha256: &str, output: &Path) -> Result<()> {
    let plan: FrozenPlan = crate::files::load_plan(plan_path, sha256)?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let report = runtime.block_on(nli_qualification::runner::run(&plan))?;
    crate::files::write_new(output, &report)?;
    Ok(())
}

pub fn run_captioning(plan_path: &Path, sha256: &str, images: &Path, output: &Path) -> Result<()> {
    let plan: CaptionPlan = crate::files::read_json(plan_path)?;
    plan.validate()?;
    require(plan.sha256()? == sha256, "caption plan digest mismatch")?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let report = runtime.block_on(caption(plan, images))?;
    crate::files::write_new(output, &report)?;
    Ok(())
}

async fn caption(plan: CaptionPlan, images: &Path) -> Result<CaptionReport> {
    use xberg::local_captioning::{
        LocalCaptioningConfig, LocalCaptioningOptions, LocalCaptioningPrecision, LocalImageCaptioner,
    };
    require(
        nli_qualification::runner::identity(plan.runtime.deadline_ms)? == plan.runtime,
        "runtime differs from the frozen caption plan",
    )?;
    let mut rows = Vec::new();
    for candidate in &plan.candidates {
        for (arm, settings) in &candidate.arms {
            let precision = match settings.precision.as_str() {
                "fp32" => LocalCaptioningPrecision::Fp32,
                "quantized" => LocalCaptioningPrecision::Quantized,
                other => return Err(format!("unsupported caption precision {other}").into()),
            };
            let config = LocalCaptioningConfig {
                model: xberg::local_captioning::LocalCaptioningModel::staged(&candidate.id, precision)?,
                limits: Default::default(),
                prompt: None,
                cache_directory: std::env::var("XBERG_QUALIFICATION_CACHE").ok(),
            };
            let captioner = LocalImageCaptioner::new(config)?;
            let options = LocalCaptioningOptions {
                deadline_ms: Some(plan.runtime.deadline_ms),
                ..Default::default()
            };
            captioner
                .stage(&options)
                .await
                .map_err(|error| format!("stage {} / {arm}: {error}", candidate.id))?;
            let prepared = captioner
                .prepare(&options)
                .await
                .map_err(|error| format!("prepare {} / {arm}: {error}", candidate.id))?;
            for image in &plan.images {
                let path = images.join(&image.image_file);
                let bytes = std::fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
                require(
                    bytes.len() == image.image_bytes && digest(&bytes) == image.image_sha256,
                    &format!("image {} differs from its frozen pin", image.id),
                )?;
                for repeat in 1..=plan.repeats {
                    let started = std::time::Instant::now();
                    let result = captioner
                        .caption(&bytes, &options)
                        .await
                        .map_err(|error| format!("caption {} / {} / {arm}: {error}", image.id, candidate.id))?;
                    require(
                        result.model == prepared,
                        "prepared and generated model identities differ",
                    )?;
                    rows.push(CaptionRow {
                        image: image.id.clone(),
                        candidate: candidate.id.clone(),
                        arm: arm.clone(),
                        repeat,
                        inference: "completed".into(),
                        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                        caption: result.caption,
                        model: serde_json::to_value(&result.model)?,
                    });
                }
            }
        }
    }
    let report = CaptionReport {
        schema: 1,
        qualified: false,
        plan_sha256: plan.sha256()?,
        runtime: plan.runtime.clone(),
        rows,
    };
    require(
        nli_qualification::runner::identity(plan.runtime.deadline_ms)? == plan.runtime,
        "runtime changed during captioning",
    )?;
    report.validate(&plan)?;
    Ok(report)
}

fn split_name(split: nli_qualification::Split) -> &'static str {
    match split {
        nli_qualification::Split::Training => "training",
        nli_qualification::Split::Calibration => "calibration",
        nli_qualification::Split::HeldOut => "held_out",
    }
}

/// Read-only view of the canonical inventory. It never rewrites the inventory and never falls back
/// to another candidate, precision or revision.
struct Inventory(Value);

impl Inventory {
    fn load(path: &Path) -> Result<Self> {
        let value: Value = crate::files::read_json(path)?;
        require(
            value["schema_version"].as_u64() == Some(2),
            "candidate inventory schema is not version 2",
        )?;
        Ok(Self(value))
    }

    fn entry(&self, id: &str) -> Result<&Value> {
        self.0["models"]
            .as_array()
            .and_then(|models| models.iter().find(|model| model["id"] == id))
            .ok_or_else(|| format!("{id} is not in the candidate inventory").into())
    }

    /// Canonical derivation of the two receipts the frozen plan requires. Both are digests of the
    /// inventory entry itself, so a plan cannot claim provenance the inventory does not pin.
    fn entry_with_receipts(&self, id: &str) -> Result<Value> {
        let entry = self.entry(id)?;
        let files = entry["files"].clone();
        let export_recipe = digest(&canonical(&json!({
            "id": entry["id"],
            "upstream_repository": entry["upstream_repository"],
            "upstream_revision": entry["upstream_revision"],
            "provenance": entry["provenance"],
            "files": files,
        }))?);
        let license_receipt = digest(&canonical(&json!({
            "license": entry["license"],
            "license_caveat": entry["license_caveat"],
            "upstream_repository": entry["upstream_repository"],
            "upstream_revision": entry["upstream_revision"],
        }))?);
        let mut value = entry.clone();
        value["export_recipe_sha256"] = json!(export_recipe);
        value["license_receipt_sha256"] = json!(license_receipt);
        Ok(value)
    }

    fn classification_descriptor(&self, id: &str) -> Result<Value> {
        use xberg::local_classification::{LocalClassificationPrecision, LocalNliModel};
        // The SDK owns the descriptor: the plan reuses it instead of rebuilding graph, manifest
        // and output selection by hand.
        let model = LocalNliModel::staged(id, LocalClassificationPrecision::Quantized)?;
        let entry = self.entry(id)?;
        require(
            entry["task"] == "zero_shot_classification"
                && entry["repository"].as_str() == Some(model.repository.as_str())
                && entry["revision"].as_str() == Some(model.revision.as_str()),
            "SDK descriptor drifted from the candidate inventory",
        )?;
        serde_json::to_value(model).map_err(Into::into)
    }

    fn captioning_graph(&self, id: &str, precision: &str) -> Result<String> {
        let entry = self.entry(id)?;
        let files = entry["files"]
            .as_array()
            .ok_or("inventory files must be a list")?
            .iter()
            .filter_map(|file| file["path"].as_str())
            .collect::<Vec<_>>();
        let decoder = files
            .iter()
            .find(|path| path.ends_with("decoder_model_merged.onnx"))
            .copied();
        let quantized = files
            .iter()
            .find(|path| path.ends_with("decoder_model_merged_int8.onnx"))
            .copied();
        match precision {
            "fp32" => decoder
                .map(str::to_owned)
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("{id} has no fp32 decoder").into()
                }),
            "quantized" => quantized
                .map(str::to_owned)
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("{id} has no int8 decoder").into()
                }),
            other => Err(format!("unsupported precision {other}").into()),
        }
    }
}

fn canonical(value: &Value) -> Result<Vec<u8>> {
    crate::canonical_json(value)
}
