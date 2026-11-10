use crate::{Result, require};
use std::collections::{BTreeMap, BTreeSet};

const COMMON_FIELDS: &[&str] = &["model name"];
const PLATFORM_FIELDS: &[&str] = &["Hardware", "Model", "Revision"];
const X86_FIELDS: &[&str] = &["vendor_id", "cpu family", "model", "stepping"];
const ARM_FIELDS: &[&str] = &[
    "Processor",
    "CPU implementer",
    "CPU architecture",
    "CPU variant",
    "CPU part",
    "CPU revision",
];

type Descriptors<'a> = BTreeMap<&'a str, BTreeSet<&'a str>>;

/// Preserve complete core descriptors separately from board-wide identifiers,
/// independently of logical processor numbering and `/proc/cpuinfo` record order.
pub fn linux_cpu_identity(cpuinfo: &str, architecture: &str) -> Result<String> {
    let architecture_fields = match architecture {
        "x86" | "x86_64" => X86_FIELDS,
        "arm" | "aarch64" | "arm64" | "armv7l" | "armv6l" => ARM_FIELDS,
        _ => &[],
    };
    let mut cores = BTreeSet::new();
    let mut current = Descriptors::new();
    let mut platform = Descriptors::new();
    for line in cpuinfo.lines() {
        if line.trim().is_empty() {
            finish_core(&mut cores, &mut current);
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if key == "processor" {
            finish_core(&mut cores, &mut current);
        } else if !value.is_empty() && PLATFORM_FIELDS.contains(&key) {
            platform.entry(key).or_default().insert(value);
        } else if !value.is_empty() && (COMMON_FIELDS.contains(&key) || architecture_fields.contains(&key)) {
            current.entry(key).or_default().insert(value);
        }
    }
    finish_core(&mut cores, &mut current);
    require(
        !cores.is_empty() || !platform.is_empty(),
        "CPU identity is unavailable: no stable identifiers",
    )?;
    Ok(serde_json::to_string(
        &serde_json::json!({"cores": cores, "platform": platform}),
    )?)
}

fn finish_core<'a>(cores: &mut BTreeSet<Descriptors<'a>>, current: &mut Descriptors<'a>) {
    if !current.is_empty() {
        cores.insert(std::mem::take(current));
    }
}
