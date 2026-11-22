use nli_qualification::{FrozenPlan, RunReport};
use onnx_qualification::caption::{CaptionPlan, CaptionReport};
use onnx_qualification::corpus::{CaptionManifest, CorpusManifest};
use onnx_qualification::prereg::Preregistration;
use onnx_qualification::score::{
    score_captioning as score_captioning_outcome, score_classification as score_classification_outcome,
};
use onnx_qualification::{Result, files};
use std::path::Path;

fn main() {
    if let Err(error) = execute(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("ONNX qualification: {error}");
        std::process::exit(1);
    }
}

fn execute(args: &[String]) -> Result<()> {
    let command = args.first().map(String::as_str).unwrap_or_default();
    match command {
        "audit" => audit(args),
        "audit-captions" => audit_captions(args),
        "prereg" => render_prereg(args),
        "score-classification" => score_classification(args),
        "score-captioning" => score_captioning(args),
        "decide" => decide(args),
        "controls" => controls(args),
        _ => runtime::execute(args),
    }
}

fn audit(args: &[String]) -> Result<()> {
    let [_, manifest] = args else {
        return Err("usage: audit MANIFEST".into());
    };
    let manifest: CorpusManifest = files::read_json(Path::new(manifest))?;
    manifest.validate()?;
    println!(
        "{} documents, {} held-out, digest {}",
        manifest.documents.len(),
        manifest.count(nli_qualification::Split::HeldOut),
        manifest.digest
    );
    Ok(())
}

fn audit_captions(args: &[String]) -> Result<()> {
    let [_, manifest] = args else {
        return Err("usage: audit-captions MANIFEST".into());
    };
    let manifest: CaptionManifest = files::read_json(Path::new(manifest))?;
    manifest.validate()?;
    println!("{} images, digest {}", manifest.images.len(), manifest.digest);
    Ok(())
}

fn render_prereg(args: &[String]) -> Result<()> {
    let [_, path] = args else {
        return Err("usage: prereg PREREGISTRATION".into());
    };
    let prereg: Preregistration = files::read_json(Path::new(path))?;
    print!("{}", prereg.render_markdown()?);
    Ok(())
}

fn score_classification(args: &[String]) -> Result<()> {
    let [_, plan_path, sha256, report_path, manifest_path, prereg_path, output] = args else {
        return Err("usage: score-classification PLAN SHA REPORT MANIFEST PREREG OUT".into());
    };
    let prereg: Preregistration = files::read_json(Path::new(prereg_path))?;
    prereg.validate()?;
    let settings = prereg
        .classification
        .as_ref()
        .ok_or("preregistration declares no classification run")?;
    let plan: FrozenPlan = files::load_plan(Path::new(plan_path), sha256)?;
    let report: RunReport = files::read_json(Path::new(report_path))?;
    let manifest: CorpusManifest = files::read_json(Path::new(manifest_path))?;
    let outcome = score_classification_outcome(&plan, &report, &manifest, settings)?;
    files::write_new(Path::new(output), &outcome)?;
    println!("{} classification variants scored", outcome.variants.len());
    Ok(())
}

fn score_captioning(args: &[String]) -> Result<()> {
    let [_, plan_path, report_path, manifest_path, prereg_path, output] = args else {
        return Err("usage: score-captioning PLAN REPORT MANIFEST PREREG OUT".into());
    };
    let prereg: Preregistration = files::read_json(Path::new(prereg_path))?;
    prereg.validate()?;
    let settings = prereg
        .captioning
        .as_ref()
        .ok_or("preregistration declares no captioning run")?;
    let plan: CaptionPlan = files::read_json(Path::new(plan_path))?;
    let report: CaptionReport = files::read_json(Path::new(report_path))?;
    let manifest: CaptionManifest = files::read_json(Path::new(manifest_path))?;
    let outcome = score_captioning_outcome(&plan, &report, &manifest, settings)?;
    files::write_new(Path::new(output), &outcome)?;
    println!("{} caption variants scored", outcome.scores.len());
    Ok(())
}

fn decide(args: &[String]) -> Result<()> {
    let [_, prereg_path, classification_path, captioning_path, output, markdown] = args else {
        return Err("usage: decide PREREG CLASSIFICATION|- CAPTIONING|- OUT MARKDOWN".into());
    };
    let prereg: Preregistration = files::read_json(Path::new(prereg_path))?;
    prereg.validate()?;
    let classification: Option<onnx_qualification::score::ClassificationOutcome> =
        optional_json(Path::new(classification_path))?;
    let captioning: Option<onnx_qualification::score::CaptionOutcome> = optional_json(Path::new(captioning_path))?;
    let decision = onnx_qualification::decision::build(&prereg, classification.as_ref(), captioning.as_ref())?;
    files::write_new(Path::new(output), &decision)?;
    std::fs::write(Path::new(markdown), decision.render_markdown()?)?;
    for item in &decision.decisions {
        println!("{}: {:?}", item.workload, item.verdict);
    }
    Ok(())
}

fn controls(args: &[String]) -> Result<()> {
    let [_, output] = args else {
        return Err("usage: controls OUT".into());
    };
    let report = onnx_qualification::controls::run();
    files::write_new(Path::new(output), &report)?;
    for control in &report.controls {
        println!(
            "{} {}: {}",
            if control.control_passed { "PASS" } else { "FAILED" },
            control.id,
            control.observed.as_deref().unwrap_or("accepted the mutation")
        );
    }
    report.require_all_passed()
}

fn optional_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if path.as_os_str() == "-" {
        return Ok(None);
    }
    Ok(Some(files::read_json(path)?))
}

#[cfg(feature = "runtime")]
mod runtime {
    use onnx_qualification::Result;

    pub fn execute(args: &[String]) -> Result<()> {
        onnx_qualification::runtime::execute(args)
    }
}

#[cfg(not(feature = "runtime"))]
mod runtime {
    use onnx_qualification::Result;

    pub fn execute(args: &[String]) -> Result<()> {
        let _ = args;
        Err(concat!(
            "usage: audit MANIFEST | audit-captions MANIFEST | prereg PREREG | ",
            "score-classification PLAN SHA REPORT MANIFEST PREREG OUT | ",
            "score-captioning PLAN REPORT MANIFEST PREREG OUT | ",
            "decide PREREG CLASSIFICATION|- CAPTIONING|- OUT MARKDOWN | controls OUT; ",
            "plan and run commands require the explicit runtime feature"
        )
        .into())
    }
}
