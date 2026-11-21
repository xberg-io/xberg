mod common;

use common::*;
use onnx_qualification::caption::CaptionReport;
use onnx_qualification::corpus::CorpusManifest;
use onnx_qualification::decision::{self, Verdict};
use onnx_qualification::metrics::{Estimate, Interval};
use onnx_qualification::prereg::Preregistration;
use onnx_qualification::score::{score_captioning, score_classification};
use serde_json::{Value, json};

fn score(
    outcome: &onnx_qualification::score::ClassificationOutcome,
    repeat: u32,
) -> &onnx_qualification::metrics::RepeatScore {
    outcome.variants[0]
        .held_out
        .repeats
        .iter()
        .find(|score| score.repeat == repeat)
        .expect("repeat")
}

#[test]
fn refuses_empty_corpus_and_wrong_counts_for_the_intended_reason() {
    let mut empty = serde_json::to_value(corpus_manifest()).unwrap();
    empty["documents"] = json!([]);
    empty["digest"] = json!(onnx_qualification::digest(b"[]"));
    let manifest: CorpusManifest = serde_json::from_value(empty).unwrap();
    assert!(
        manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("must not be empty")
    );

    let mut wrong = prereg_value();
    wrong["classification"]["expected"]["total"] = json!(11);
    let prereg: Preregistration = serde_json::from_value(wrong).unwrap();
    assert!(
        prereg
            .validate()
            .unwrap_err()
            .to_string()
            .contains("does not equal its factors")
    );
}

#[test]
fn scores_the_complete_run_with_hand_computed_values() {
    let outcome = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    let first = score(&outcome, 1);
    assert!(
        (first.macro_f1.value.unwrap() - 0.25).abs() < 1e-12,
        "{:?}",
        first.macro_f1
    );
    assert!((first.micro_accuracy.value.unwrap() - 1.0 / 3.0).abs() < 1e-12);
    assert!((first.abstention_rate_on_known.value.unwrap() - 1.0 / 3.0).abs() < 1e-12);
    assert_eq!(first.unknown_documents, 1);
    assert!((first.unknown_rejection.value.unwrap() - 0.0).abs() < 1e-12);
    assert!((first.ece.value.unwrap() - 0.4).abs() < 1e-6, "{:?}", first.ece);
    assert_eq!(
        first.adverse_examples.len(),
        3,
        "1 misclassification + 1 abstention + 1 forced unknown"
    );

    let second = score(&outcome, 2);
    assert!((second.macro_f1.value.unwrap() - 1.0).abs() < 1e-12);
    assert!((outcome.variants[0].held_out.repeat_stability.unwrap() - 0.75).abs() < 1e-12);

    let baseline = &outcome.variants[0].held_out.baseline;
    assert_eq!(baseline.predicted_class.as_deref(), Some("alpha"));
    assert!((baseline.macro_f1.value.unwrap() - 0.4).abs() < 1e-12);

    assert!(
        outcome
            .assertions
            .iter()
            .all(|assertion| assertion.expected == assertion.observed)
    );
    assert!(
        outcome
            .assertions
            .iter()
            .any(|assertion| assertion.name == "report rows")
    );
    assert!(
        outcome.variants[0].calibration.is_some(),
        "calibration split is scored separately"
    );
}

#[test]
fn rejects_a_report_that_skips_or_drops_work() {
    let manifest = corpus_manifest();
    let settings = preregistration().classification.unwrap();
    let mut skipped = report_value();
    skipped["rows"][0]["inference"] = json!("skipped");
    assert!(
        score_classification(&plan(), &serde_json::from_value(skipped).unwrap(), &manifest, &settings)
            .unwrap_err()
            .to_string()
            .contains("inference must complete")
    );
    let mut dropped = report_value();
    dropped["rows"].as_array_mut().unwrap().pop();
    assert!(
        score_classification(&plan(), &serde_json::from_value(dropped).unwrap(), &manifest, &settings)
            .unwrap_err()
            .to_string()
            .contains("row count")
    );
}

#[test]
fn refuses_a_plan_document_absent_from_the_corpus() {
    let mut corpus = corpus();
    corpus["documents"].as_array_mut().unwrap().remove(0);
    corpus["digest"] = json!(onnx_qualification::canonical_digest(&corpus["documents"]).unwrap());
    let manifest: CorpusManifest = serde_json::from_value(corpus).unwrap();
    assert!(
        score_classification(
            &plan(),
            &report(),
            &manifest,
            preregistration().classification.as_ref().unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("absent from the committed corpus")
    );
}

#[test]
fn binds_the_score_to_the_preregistered_corpus_digest() {
    let outcome = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    let mut prereg = preregistration();
    prereg.corpora.get_mut("classification").unwrap().digest = "0".repeat(64);
    assert!(
        decision::build(&prereg, Some(&outcome), None)
            .unwrap_err()
            .to_string()
            .contains("different corpus")
    );
}

#[test]
fn an_all_abstaining_run_reports_unmeasured_calibration_instead_of_zero() {
    let settings = preregistration().classification.unwrap();
    let mut report = serde_json::to_value(report()).unwrap();
    for row in report["rows"].as_array_mut().unwrap() {
        row["labels"] = json!([]);
    }
    let outcome = score_classification(
        &plan(),
        &serde_json::from_value(report).unwrap(),
        &corpus_manifest(),
        &settings,
    )
    .unwrap();
    let first = score(&outcome, 1);
    assert!(first.ece.value.is_none());
    assert!(first.selective_accuracy.value.is_none());
    assert!((first.abstention_rate_on_known.value.unwrap() - 1.0).abs() < 1e-12);
    assert!((first.macro_f1.value.unwrap() - 0.0).abs() < 1e-12);
    assert!((first.unknown_rejection.value.unwrap() - 1.0).abs() < 1e-12);
}

fn passing(outcome: &mut onnx_qualification::score::ClassificationOutcome) {
    for variant in &mut outcome.variants {
        for repeat in &mut variant.held_out.repeats {
            repeat.macro_f1 = Estimate::measured(0.9, 138);
            repeat.ci = Some(Interval {
                lower: 0.8,
                upper: 0.95,
                level: 0.95,
                replicates: 2000,
                clusters: 25,
            });
            for class in repeat.per_class.values_mut() {
                class.recall = Estimate::measured(0.85, 40);
            }
            repeat.ece = Estimate::measured(0.05, 120);
            repeat.abstention_rate_on_known = Estimate::measured(0.1, 138);
            repeat.unknown_rejection = Estimate::measured(0.9, 48);
        }
    }
}

#[test]
fn decision_requires_every_check_to_pass() {
    let mut outcome = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    passing(&mut outcome);
    let decision = decision::build(&preregistration(), Some(&outcome), None).unwrap();
    assert_eq!(decision.decisions[0].verdict, Verdict::Go);
    assert!(!decision.qualified && !decision.defaults_changed);

    outcome.variants[0].held_out.repeats[0].macro_f1 = Estimate::measured(0.3, 138);
    let decision = decision::build(&preregistration(), Some(&outcome), None).unwrap();
    assert_eq!(decision.decisions[0].verdict, Verdict::NoGo);
    assert!(
        decision.decisions[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("macro-F1"))
    );

    let mut outcome = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    passing(&mut outcome);
    outcome.variants[0].held_out.repeats[0].unknown_rejection = Estimate::measured(0.2, 48);
    let decision = decision::build(&preregistration(), Some(&outcome), None).unwrap();
    assert_eq!(decision.decisions[0].verdict, Verdict::NoGo);
    assert!(
        decision.decisions[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("unknown-class rejection"))
    );
}

#[test]
fn a_missing_ci_or_metric_cannot_pass_the_gate() {
    let mut outcome = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    passing(&mut outcome);
    outcome.variants[0].held_out.repeats[0].ci = None;
    let decision = decision::build(&preregistration(), Some(&outcome), None).unwrap();
    assert_eq!(decision.decisions[0].verdict, Verdict::NoGo);
}

#[test]
fn caption_scores_only_the_defensible_subset() {
    let outcome = score_captioning(
        &caption_plan(),
        &caption_report(),
        &caption_manifest(),
        preregistration().captioning.as_ref().unwrap(),
    )
    .unwrap();
    let fp32 = outcome.scores.iter().find(|score| score.arm == "fp32").unwrap();
    assert_eq!(fp32.generations, 2);
    assert!((fp32.number_recall.value.unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(fp32.number_recall.observations, 1);
    assert!((fp32.term_recall.value.unwrap() - 1.0).abs() < 1e-12);
    assert!((fp32.invented_number_image_rate.value.unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(fp32.invented_numbers.get("5"), Some(&1));
    assert_eq!(fp32.empty_captions, 0);
    assert_eq!(outcome.contrasts.len(), 1);
    assert!((outcome.contrasts[0].term_recall_delta.as_ref().unwrap().lower - 0.0).abs() < 1e-12);
}

#[test]
fn caption_plan_refuses_a_generation_count_mismatch() {
    let mut report = serde_json::to_value(caption_report()).unwrap();
    report["rows"].as_array_mut().unwrap().pop();
    let report: CaptionReport = serde_json::from_value(report).unwrap();
    assert!(
        score_captioning(
            &caption_plan(),
            &report,
            &caption_manifest(),
            preregistration().captioning.as_ref().unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("generation count")
    );
}

#[test]
fn captioning_cannot_go_while_human_rating_is_required() {
    let outcome = score_captioning(
        &caption_plan(),
        &caption_report(),
        &caption_manifest(),
        preregistration().captioning.as_ref().unwrap(),
    )
    .unwrap();
    let decision = decision::build(&preregistration(), None, Some(&outcome)).unwrap();
    assert_eq!(decision.decisions[0].verdict, Verdict::PendingHuman);
    assert!(
        decision.decisions[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("human raters"))
    );
}

#[test]
fn captioning_fails_the_gate_when_an_automatic_bar_fails() {
    let mut prereg = preregistration();
    prereg.captioning.as_mut().unwrap().thresholds.number_recall_min = 0.9;
    let outcome = score_captioning(
        &caption_plan(),
        &caption_report(),
        &caption_manifest(),
        prereg.captioning.as_ref().unwrap(),
    )
    .unwrap();
    let decision = decision::build(&prereg, None, Some(&outcome)).unwrap();
    assert_eq!(decision.decisions[0].verdict, Verdict::NoGo);
}

#[test]
fn preregistration_requires_a_post_url_and_metric_definitions() {
    let mut value = prereg_value();
    value["posted_url"] = json!("");
    let prereg: Preregistration = serde_json::from_value(value).unwrap();
    assert!(
        prereg
            .validate()
            .unwrap_err()
            .to_string()
            .contains("where it was posted")
    );
    let mut value = prereg_value();
    value["metric_definitions"] = json!({});
    let prereg: Preregistration = serde_json::from_value(value).unwrap();
    assert!(
        prereg
            .validate()
            .unwrap_err()
            .to_string()
            .contains("define its metrics")
    );
}

#[test]
fn corpus_manifest_refuses_family_leakage_and_tampering() {
    let mut value = corpus();
    value["documents"][4]["family"] = json!("f1");
    value["digest"] = json!(onnx_qualification::canonical_digest(&value["documents"]).unwrap());
    let manifest: CorpusManifest = serde_json::from_value(value).unwrap();
    assert!(
        manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("spans two splits")
    );
    let mut value = corpus();
    value["documents"][0]["text_sha256"] = json!("0".repeat(64));
    let manifest: CorpusManifest = serde_json::from_value(value).unwrap();
    assert!(manifest.validate().unwrap_err().to_string().contains("digest mismatch"));
}

#[test]
fn negative_controls_all_fire_for_the_intended_reason() {
    let report = onnx_qualification::controls::run();
    assert_eq!(report.controls.len(), 14);
    assert_eq!(
        report.failed,
        0,
        "{:?}",
        report.controls.iter().filter(|c| !c.control_passed).collect::<Vec<_>>()
    );
    assert!(report.require_all_passed().is_ok());
}

#[test]
fn scoring_is_reproducible_for_a_fixed_seed() {
    let first = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    let second = score_classification(
        &plan(),
        &report(),
        &corpus_manifest(),
        &preregistration().classification.unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );
}

#[test]
fn unknown_backend_rows_cannot_deserialize_as_cpu_work() {
    let mut value = report_value();
    value["rows"][0]["model"]["effective_backend"] = json!("provider");
    assert!(
        nli_qualification::files::parse_json::<nli_qualification::RunReport>(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .to_string()
            .contains("unknown variant")
    );
}

#[test]
fn corpus_manifest_rejects_unknown_fields() {
    let mut value: Value = serde_json::to_value(corpus_manifest()).unwrap();
    value["documents"][0]["extra"] = json!(true);
    assert!(serde_json::from_value::<CorpusManifest>(value).is_err());
}
