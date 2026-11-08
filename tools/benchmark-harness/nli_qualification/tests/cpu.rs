use nli_qualification::cpu::linux_cpu_identity;

const ARM: &str = "processor\t: 0\nCPU implementer\t: 0x41\nCPU architecture: 8\nCPU variant: 0x0\nCPU part: 0xd0c\nCPU revision: 1\nHardware: Example ARM board\n";

#[test]
fn linux_arm_records_available_stable_identifiers_without_model_name() {
    for architecture in ["aarch64", "arm", "armv7l"] {
        let value = linux_cpu_identity(ARM, architecture).expect("ARM identity from implementer/part/Hardware");
        let fields: serde_json::Value = serde_json::from_str(&value).unwrap();
        assert_eq!(
            fields,
            serde_json::json!({
                "cores": [{"CPU implementer": ["0x41"], "CPU architecture": ["8"], "CPU variant": ["0x0"],
                    "CPU part": ["0xd0c"], "CPU revision": ["1"]}],
                "platform": {"Hardware": ["Example ARM board"]}
            })
        );
    }
}

#[test]
fn linux_x86_preserves_model_name() {
    let value = linux_cpu_identity(
        "processor: 0\nmodel name\t: Example x86 CPU\nvendor_id: GenuineIntel\n",
        "x86_64",
    )
    .expect("x86 CPU identity");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&value).unwrap(),
        serde_json::json!({"cores": [{"model name": ["Example x86 CPU"], "vendor_id": ["GenuineIntel"]}], "platform": {}})
    );
}

#[test]
fn linux_cpu_without_stable_identifiers_fails_for_missing_information() {
    for input in [
        "processor: 0\nBogoMIPS: 42\nFeatures: fp asimd\n",
        "model name:  \nHardware:\n",
        "",
    ] {
        let error = linux_cpu_identity(input, "aarch64").unwrap_err();
        assert!(error.to_string().contains("CPU identity is unavailable"));
    }
}

#[test]
fn linux_arm_accepts_partial_available_identifiers() {
    for (key, value) in [
        ("CPU implementer", "0x41"),
        ("CPU part", "0xd0c"),
        ("Hardware", "ARM board"),
    ] {
        let identity = linux_cpu_identity(&format!("{key}: {value}\n"), "aarch64").unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&identity).unwrap(),
            if key == "Hardware" {
                serde_json::json!({"cores": [], "platform": {key: [value]}})
            } else {
                serde_json::json!({"cores": [{key: [value]}], "platform": {}})
            }
        );
    }
}

#[test]
fn repeated_and_reordered_cpu_records_preserve_every_distinct_stable_identity() {
    let first = "processor: 0\nCPU part: 0xd0c\nCPU implementer: 0x41\n";
    let second = "processor: 7\nCPU part: 0xd40\nCPU implementer: 0x41\n";
    let identity = linux_cpu_identity(&format!("{first}{second}{first}"), "aarch64").unwrap();
    assert_eq!(
        identity,
        linux_cpu_identity(&format!("{second}{first}"), "aarch64").unwrap()
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&identity).unwrap(),
        serde_json::json!({"cores": [
            {"CPU implementer": ["0x41"], "CPU part": ["0xd0c"]},
            {"CPU implementer": ["0x41"], "CPU part": ["0xd40"]}
        ], "platform": {}})
    );
}

#[test]
fn architecture_specific_fields_and_exact_keys_do_not_invent_a_cpu_identity() {
    for input in ["model name bogus: Pretend CPU", "CPU part: 0xd0c", "processor: 0"] {
        assert!(
            linux_cpu_identity(input, "x86_64")
                .unwrap_err()
                .to_string()
                .contains("CPU identity is unavailable")
        );
    }
}

#[test]
fn heterogeneous_core_parts_remain_associated_with_their_revisions() {
    let original = "processor: 0\nCPU part: A\nCPU revision: 0\n\nprocessor: 1\nCPU part: B\nCPU revision: 1\n";
    let swapped = "processor: 0\nCPU part: A\nCPU revision: 1\n\nprocessor: 1\nCPU part: B\nCPU revision: 0\n";
    assert_ne!(
        linux_cpu_identity(original, "aarch64").unwrap(),
        linux_cpu_identity(swapped, "aarch64").unwrap()
    );
}
