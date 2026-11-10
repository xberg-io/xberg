use super::qualification::{Mode, Report, recorded};
use std::path::Path;

fn invoke(report: &Report, root: &Path) -> std::process::Output {
    let input = root.join("input.json");
    std::fs::write(&input, serde_json::to_vec(report).unwrap()).unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "paired_public_source_word_qualification",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("XBERG_2711_REPLAY", &input)
        .env("XBERG_2711_QUALIFICATION", root)
        .env("XBERG_2711_ARM", "checked")
        .env_remove("XBERG_2711_DENOISE");
    if report.mode == Mode::Denoise {
        command.env("XBERG_2711_DENOISE", "1");
    }
    command.output().expect("run the public qualification command")
}

fn assert_rejected(report: &Report, reason: &str) {
    assert_eq!(report.rows.len(), 13);
    assert!(report.rows.iter().all(|row| !row.text.is_empty()));
    let root = tempfile::tempdir().unwrap();
    let output = invoke(report, root.path());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "public quality command accepted invalid report: {stdout}"
    );
    assert!(stdout.contains("0 passed; 1 failed; 0 ignored"), "{stdout}");
    assert!(stderr.contains(reason), "wrong rejection: {stdout}\n{stderr}");
    assert!(
        !root.path().join("checked.json").exists(),
        "a rejected report was published"
    );
}

#[test]
fn retained_reports_pass_through_the_public_quality_command() {
    for mode in [Mode::Default, Mode::Denoise] {
        let report = recorded(mode);
        let root = tempfile::tempdir().unwrap();
        let output = invoke(&report, root.path());
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed; 0 ignored"));
        assert!(String::from_utf8_lossy(&output.stdout).contains("quality gate passed: 13 pinned cases"));
        let checked: Report =
            serde_json::from_slice(&std::fs::read(root.path().join("checked.json")).unwrap()).unwrap();
        assert_eq!(checked.rows.len(), 13);
        assert_eq!(
            serde_json::to_value(checked).unwrap(),
            serde_json::to_value(report).unwrap()
        );
    }
}

#[test]
fn public_quality_command_rejects_lost_source_occurrences() {
    for mode in [Mode::Default, Mode::Denoise] {
        let mut report = recorded(mode);
        let row = report.rows.iter_mut().find(|row| row.name == "I-right").unwrap();
        assert!(row.text.contains("report"));
        row.text = row.text.replacen("report", "", 1);
        assert_rejected(&report, "lost source occurrence");
    }
}

#[test]
fn public_quality_command_rejects_new_false_occurrences() {
    for mode in [Mode::Default, Mode::Denoise] {
        let mut report = recorded(mode);
        let row = report.rows.iter_mut().find(|row| row.name == "I-right").unwrap();
        row.text.push_str(" inventedword");
        assert_rejected(&report, "new false word occurrence");
    }
}

#[test]
fn public_quality_command_rejects_nonempty_wrong_fixture_hash() {
    for mode in [Mode::Default, Mode::Denoise] {
        let mut report = recorded(mode);
        report.rows.iter_mut().find(|row| row.name == "I-right").unwrap().sha256 = "a".repeat(64);
        assert_rejected(&report, "fixture SHA-256 mismatch");
    }
}

#[test]
fn public_quality_command_rejects_one_repeated_source_occurrence() {
    for mode in [Mode::Default, Mode::Denoise] {
        let mut report = recorded(mode);
        let row = report.rows.iter_mut().find(|row| row.name == "I-right").unwrap();
        assert!(row.text.matches("in a").count() > 1);
        row.text = row.text.replacen("in a", "a", 1);
        assert_rejected(&report, "lost source occurrence");
    }
}

#[test]
fn public_quality_command_rejects_excess_source_occurrences() {
    for mode in [Mode::Default, Mode::Denoise] {
        let mut report = recorded(mode);
        let row = report.rows.iter_mut().find(|row| row.name == "I-right").unwrap();
        row.text.push_str(" report");
        assert_rejected(&report, "new false word occurrence");
    }
}
