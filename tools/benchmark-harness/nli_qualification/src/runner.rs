use crate::{Candidate, FrozenPlan, Prediction, Result, RunReport, RuntimeIdentity, digest, require};
use std::time::Instant;
use xberg::local_classification::{LocalClassificationOptions, LocalTextClassifier};

pub fn identity(deadline_ms: u64) -> Result<RuntimeIdentity> {
    let ort_build_info = std::panic::catch_unwind(ort::info)
        .map_err(|_| "ONNX Runtime build identity is unavailable")?
        .to_owned();
    Ok(RuntimeIdentity {
        runner_sha256: digest(&std::fs::read(std::env::current_exe()?)?),
        dependency_lock_sha256: digest(include_bytes!("../Cargo.lock")),
        ort_build_info,
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        cpu: cpu_identity()?,
        threads: 0,
        deadline_ms,
    })
}

fn cpu_identity() -> Result<String> {
    match std::env::consts::OS {
        "linux" => std::fs::read_to_string("/proc/cpuinfo")?
            .lines()
            .find_map(|line| {
                line.strip_prefix("model name")
                    .and_then(|line| line.split_once(':'))
                    .map(|(_, value)| value.trim().to_owned())
            })
            .ok_or_else(|| "CPU identity is unavailable".into()),
        "macos" => {
            let output = std::process::Command::new("/usr/sbin/sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()?;
            require(output.status.success(), "CPU identity command failed")?;
            Ok(String::from_utf8(output.stdout)?.trim().to_owned())
        }
        _ => Err("CPU identity is unsupported on this target".into()),
    }
}

pub async fn run(plan: &FrozenPlan) -> Result<RunReport> {
    plan.validate()?;
    require(
        identity(plan.runtime.deadline_ms)? == plan.runtime,
        "runtime differs from frozen plan",
    )?;
    let mut report = RunReport {
        schema_version: 1,
        qualified: false,
        plan_sha256: plan.sha256()?,
        runtime: plan.runtime.clone(),
        rows: Vec::new(),
    };
    for candidate in &plan.candidates {
        for arm in &plan.arms {
            append_variant(plan, candidate, arm, &mut report.rows).await?;
        }
    }
    require(
        identity(plan.runtime.deadline_ms)? == plan.runtime,
        "runtime changed during inference",
    )?;
    report.validate(plan)?;
    Ok(report)
}

async fn append_variant(plan: &FrozenPlan, candidate: &Candidate, arm: &str, rows: &mut Vec<Prediction>) -> Result<()> {
    let options = LocalClassificationOptions {
        deadline_ms: Some(plan.runtime.deadline_ms),
        ..Default::default()
    };
    let classifier = LocalTextClassifier::new(candidate.variants[arm].config.clone())?;
    let prepared = classifier
        .prepare(&options)
        .await
        .map_err(|error| format!("prepare {} / {arm}: {error}", candidate.id))?;
    for document in &plan.documents {
        for repeat in 1..=plan.repeats {
            let started = Instant::now();
            let result = classifier
                .classify(&document.text, &plan.labels, &options)
                .await
                .map_err(|error| {
                    format!(
                        "inference {} / {} / {arm} / {repeat}: {error}",
                        document.id, candidate.id
                    )
                })?;
            require(
                result.model == prepared,
                "prepared and inferred model identities differ",
            )?;
            rows.push(Prediction {
                document: document.id.clone(),
                candidate: candidate.id.clone(),
                arm: arm.to_owned(),
                repeat,
                inference: "completed".into(),
                elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                labels: result.labels,
                model: result.model,
            });
        }
    }
    Ok(())
}
