use nli_qualification::{FrozenPlan, Result, RunReport, files};
use std::path::Path;

fn main() {
    if let Err(error) = execute(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("NLI qualification: {error}");
        std::process::exit(1);
    }
}

fn execute(args: &[String]) -> Result<()> {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["freeze", input, output] => {
            let plan: FrozenPlan = files::read_json(Path::new(input))?;
            plan.validate()?;
            files::write_new(Path::new(output), &plan)?;
            println!("{}", plan.sha256()?);
            Ok(())
        }
        ["validate", path, sha256, report] => {
            let plan = files::load_plan(Path::new(path), sha256)?;
            let report: RunReport = files::read_json(Path::new(report))?;
            report.validate(&plan)?;
            println!("{} / {} inferences; qualified=false", report.rows.len(), plan.expected_inferences);
            Ok(())
        }
        ["run", path, sha256, output] => run(Path::new(path), sha256, Path::new(output)),
        ["identity", deadline] => identity(deadline),
        _ => Err("usage: freeze INPUT NEW_PLAN | validate PLAN SHA256 REPORT | run PLAN SHA256 NEW_REPORT | identity DEADLINE_MS".into()),
    }
}

#[cfg(feature = "runtime")]
fn run(path: &Path, sha256: &str, output: &Path) -> Result<()> {
    let plan = files::load_plan(path, sha256)?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let report = runtime.block_on(nli_qualification::runner::run(&plan))?;
    files::write_new(output, &report)
}

#[cfg(feature = "runtime")]
fn identity(deadline: &str) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&nli_qualification::runner::identity(deadline.parse()?)?)?
    );
    Ok(())
}

#[cfg(not(feature = "runtime"))]
fn run(_path: &Path, _sha256: &str, _output: &Path) -> Result<()> {
    Err("run requires the explicit runtime feature; no inference was performed".into())
}

#[cfg(not(feature = "runtime"))]
fn identity(_deadline: &str) -> Result<()> {
    Err("identity requires the explicit runtime feature".into())
}
