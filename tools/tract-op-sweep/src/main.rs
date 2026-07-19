//! tract op-coverage sweep (issue #1275, Phase 0).
//!
//! Loads each ONNX model xberg ships through the pure-Rust `tract` engine and
//! records how far it gets: parse (`model_for_path`) -> optimize (`into_optimized`)
//! -> runnable (`into_runnable`). The failing stage and error message tell us which
//! models are ready for the tract backend, which need dynamic-shape / op work, and
//! which must stay on ONNX Runtime.
//!
//! Models are read from the local HuggingFace hub cache (the same artifacts the
//! runtime downloads from `xberg-io/layout-models` and
//! `xberg-io/paddleocr-onnx-models`); no weights are converted or regenerated.
//!
//! Run: `cargo run -p tract-op-sweep -- [--cache-dir <hf-hub>] [--json report.json]`

use std::io::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use clap::Parser;
use serde::Serialize;
// tract-onnx prelude: `onnx()`, `model_for_path`, `with_input_fact`, `f32::fact`,
// `into_optimized`, `into_runnable`.
use tract_onnx::prelude::*;

/// One model in the canonical sweep set. `suffix` is matched against the tail of a
/// cached `.onnx` path; `repo_marker` disambiguates which HF repo it lives in.
struct ModelSpec {
    name: &'static str,
    arch: &'static str,
    category: &'static str,
    repo_marker: &'static str,
    suffix: &'static str,
}

/// The models xberg runs through ORT today, one per architecture archetype from
/// the issue's coverage table plus the CNN classifiers.
const MODELS: &[ModelSpec] = &[
    // Layout — DETR family
    ModelSpec {
        name: "tatr",
        arch: "DETR (Table Transformer)",
        category: "layout",
        repo_marker: "layout-models",
        suffix: "tatr/model.onnx",
    },
    ModelSpec {
        name: "rtdetr",
        arch: "RT-DETR (NMS-free)",
        category: "layout",
        repo_marker: "layout-models",
        suffix: "rtdetr/model.onnx",
    },
    ModelSpec {
        name: "pp_doclayout_v3",
        arch: "Paddle DETR + in-graph NMS",
        category: "layout",
        repo_marker: "layout-models",
        suffix: "pp_doclayout_v3/model.onnx",
    },
    // Layout / OCR — CNN classifiers (first tract targets)
    ModelSpec {
        name: "table_cls",
        arch: "PP-LCNet (CNN)",
        category: "classifier",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v2/classifiers/PP-LCNet_x1_0_table_cls.onnx",
    },
    ModelSpec {
        name: "doc_ori",
        arch: "PP-LCNet (CNN)",
        category: "auto-rotate",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx",
    },
    ModelSpec {
        name: "textline_ori",
        arch: "PP-LCNet (CNN)",
        category: "orientation",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx",
    },
    ModelSpec {
        name: "angle_cls",
        arch: "AngleNet (CNN)",
        category: "orientation",
        repo_marker: "paddleocr-onnx-models",
        suffix: "ch_ppocr_mobile_v2.0_cls_infer.onnx",
    },
    // PaddleOCR — detection (DBNet CNN)
    ModelSpec {
        name: "db_det_v5_server",
        arch: "DBNet (CNN)",
        category: "detection",
        repo_marker: "paddleocr-onnx-models",
        suffix: "PP-OCRv5_server_det_infer.onnx",
    },
    ModelSpec {
        name: "det_v6_medium",
        arch: "DBNet (CNN)",
        category: "detection",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v6/det/medium/model.onnx",
    },
    // PaddleOCR — recognition (CRNN = CNN + LSTM)
    ModelSpec {
        name: "rec_v6_medium",
        arch: "CRNN (CNN + LSTM)",
        category: "recognition",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v6/rec/medium/model.onnx",
    },
    // Table structure — SLANeXt / SLANet+ (seq2seq, Scan/Loop risk)
    ModelSpec {
        name: "slanet_wired",
        arch: "SLANeXt seq2seq",
        category: "table-structure",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v2/table/SLANeXt_wired.onnx",
    },
    ModelSpec {
        name: "slanet_wireless",
        arch: "SLANeXt seq2seq",
        category: "table-structure",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v2/table/SLANeXt_wireless.onnx",
    },
    ModelSpec {
        name: "slanet_plus",
        arch: "SLANet+ seq2seq",
        category: "table-structure",
        repo_marker: "paddleocr-onnx-models",
        suffix: "v2/table/SLANet_plus.onnx",
    },
];

/// Candidate concrete NCHW input shapes tried on single-input models whose first
/// (fully-symbolic) pass fails — the runtime always feeds a pinned resolution, so a
/// model that only fails on dynamic shape is "ready once the input is pinned", not a
/// real op gap. Covers detector/DETR (640), classifier (224/48-line), CRNN rec.
const CANDIDATE_SHAPES: &[&[usize]] = &[&[1, 3, 224, 224], &[1, 3, 640, 640], &[1, 3, 48, 320], &[1, 3, 32, 320]];

#[derive(Serialize)]
struct Outcome {
    name: String,
    arch: String,
    category: String,
    path: Option<String>,
    inputs: Option<usize>,
    /// Steps passed with the model's declared (symbolic) input facts: 0 = load
    /// failed / missing, 1 = optimize failed, 2 = runnable failed, 3 = runnable.
    passed: u8,
    /// If `passed < 3` but the model goes fully runnable once a concrete input shape
    /// is pinned, the shape that worked — the "ready with a fixed input" class.
    ready_with_shape: Option<String>,
    /// Error at the first failing stage of the symbolic pass, if any.
    error: Option<String>,
}

impl Outcome {
    /// Overall verdict for the coverage matrix.
    fn verdict(&self) -> &'static str {
        if self.passed == 3 {
            "ready"
        } else if self.ready_with_shape.is_some() {
            "ready (pin input)"
        } else if self.path.is_none() {
            "missing"
        } else {
            "needs-work"
        }
    }
}

#[derive(Parser)]
#[command(about = "Sweep xberg's ONNX models through the tract engine (issue #1275, Phase 0)")]
struct Args {
    /// HuggingFace hub cache dir holding the model repos.
    #[arg(long, default_value_t = default_cache_dir())]
    cache_dir: String,

    /// Also write a JSON report to this path.
    #[arg(long)]
    json: Option<PathBuf>,
}

fn default_cache_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home}/.cache/huggingface/hub")
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let cache = PathBuf::from(&args.cache_dir);

    let onnx_files = collect_onnx(&cache);
    let outcomes: Vec<Outcome> = MODELS.iter().map(|spec| sweep(spec, &onnx_files)).collect();

    print_table(&outcomes);
    if let Some(json_path) = &args.json {
        let mut f = std::fs::File::create(json_path)?;
        f.write_all(serde_json::to_string_pretty(&outcomes)?.as_bytes())?;
        eprintln!("\nJSON report written to {}", json_path.display());
    }
    Ok(())
}

/// Recursively collect every `*.onnx` under `root` (following symlinks — the HF
/// cache stores snapshot files as symlinks into `blobs/`).
fn collect_onnx(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "onnx") {
                out.push(path);
            }
        }
    }
    out
}

/// Resolve a spec to a cached file, preferring the most recently modified match
/// when multiple snapshots are present.
fn resolve(spec: &ModelSpec, files: &[PathBuf]) -> Option<PathBuf> {
    files
        .iter()
        .filter(|p| {
            let s = p.to_string_lossy();
            s.contains(spec.repo_marker) && s.ends_with(spec.suffix)
        })
        .max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
        .cloned()
}

fn sweep(spec: &ModelSpec, files: &[PathBuf]) -> Outcome {
    let mut out = Outcome {
        name: spec.name.to_string(),
        arch: spec.arch.to_string(),
        category: spec.category.to_string(),
        path: None,
        inputs: None,
        passed: 0,
        ready_with_shape: None,
        error: None,
    };

    let Some(path) = resolve(spec, files) else {
        out.error = Some("model not found in cache".to_string());
        return out;
    };
    out.path = Some(path.display().to_string());

    // Stage 1: parse the ONNX graph.
    let loaded = match run_stage(|| tract_onnx::onnx().model_for_path(&path)) {
        Ok(m) => m,
        Err(e) => {
            out.error = Some(e);
            return out;
        }
    };
    out.inputs = Some(loaded.inputs.len());

    // Stage 2 + 3 with the model's declared (possibly symbolic) input facts.
    match optimize_and_run(loaded) {
        Ok(()) => {
            out.passed = 3;
            return out;
        }
        Err((stage, msg)) => {
            out.passed = stage; // 1 = optimize failed, 2 = runnable failed
            out.error = Some(msg);
        }
    }

    // Second pass: single-input models often fail only because the ONNX input is
    // fully symbolic. Pin a concrete NCHW shape (as the runtime does) and see if the
    // graph then goes fully runnable — the "ready once the input is pinned" class.
    if out.inputs == Some(1) {
        for shape in CANDIDATE_SHAPES {
            let reloaded = match run_stage(|| tract_onnx::onnx().model_for_path(&path)) {
                Ok(m) => m,
                Err(_) => break,
            };
            let pinned = run_stage(AssertUnwindSafe(|| {
                reloaded.with_input_fact(0, f32::fact(*shape).into())
            }));
            let Ok(pinned) = pinned else { continue };
            if optimize_and_run(pinned).is_ok() {
                out.ready_with_shape = Some(format!("{shape:?}"));
                break;
            }
        }
    }

    out
}

/// Run optimize (analyse + type + declutter) then build the plan. Returns
/// `Err((stage, msg))` where stage is 1 (optimize) or 2 (runnable).
fn optimize_and_run(loaded: InferenceModel) -> Result<(), (u8, String)> {
    let model = run_stage(AssertUnwindSafe(|| loaded.into_optimized())).map_err(|e| (1u8, e))?;
    run_stage(AssertUnwindSafe(|| model.into_runnable()))
        .map(|_| ())
        .map_err(|e| (2u8, e))
}

/// Run one tract stage, converting both `Err` and panics into a short message.
fn run_stage<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => Ok(v),
        // `{:#}` renders the full anyhow cause chain on one line — the deepest cause
        // names the offending op / attribute, which is what the matrix needs.
        Ok(Err(e)) => Err(format!("{e:#}")),
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panicked".to_string());
            Err(format!("panic: {msg}"))
        }
    }
}

/// Collapse whitespace and truncate for the compact table cell (full text stays in
/// the JSON report).
fn short(s: &str) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.len() > 120 {
        format!("{}…", &one[..120])
    } else {
        one
    }
}

fn cell(passed: u8, step: u8) -> &'static str {
    if passed >= step { "✓" } else { "✗" }
}

fn print_table(outcomes: &[Outcome]) {
    println!("# tract op-coverage sweep (issue #1275, Phase 0)\n");
    println!("`Load`/`Optimize`/`Runnable` are the symbolic-input pass. `Verdict` folds in the");
    println!("concrete-input retry: **ready** = runnable as-is, **ready (pin input)** = runnable once a");
    println!("fixed NCHW shape is set (the runtime always does), **needs-work** = real op/shape gap.\n");
    println!("| Model | Arch | Category | In | Load | Optimize | Runnable | Verdict | Note |");
    println!("|---|---|---|:-:|:-:|:-:|:-:|---|---|");
    for o in outcomes {
        let note = match (&o.ready_with_shape, &o.error) {
            (Some(shape), _) => format!("runnable with input `{shape}`"),
            (None, Some(e)) => short(e),
            (None, None) => String::new(),
        };
        println!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
            o.name,
            o.arch,
            o.category,
            o.inputs.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
            cell(o.passed, 1),
            cell(o.passed, 2),
            cell(o.passed, 3),
            o.verdict(),
            note.replace('|', "\\|"),
        );
    }

    let ready = outcomes.iter().filter(|o| o.passed == 3).count();
    let pinned = outcomes
        .iter()
        .filter(|o| o.passed < 3 && o.ready_with_shape.is_some())
        .count();
    let needs = outcomes.iter().filter(|o| o.verdict() == "needs-work").count();
    println!(
        "\n**{ready} ready as-is · {pinned} ready once input pinned · {needs} needs-work · {} total.**",
        outcomes.len()
    );
    let missing = outcomes.iter().filter(|o| o.path.is_none()).count();
    if missing > 0 {
        println!("\n_{missing} model(s) not found in the cache — download them first or pass `--cache-dir`._");
    }
}
