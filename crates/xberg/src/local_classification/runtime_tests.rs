use super::tests::configuration;
use super::{LocalClassificationOptions, LocalTextClassifier};

#[tokio::test]
async fn an_unprepared_classifier_rejects_inference_without_resolving_artifacts() {
    let classifier = LocalTextClassifier::new(configuration()).unwrap();
    let error = classifier
        .classify("alpha", &["science".to_owned()], &Default::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("prepare"), "{error}");
}

#[tokio::test]
async fn cancellation_precedes_permit_wait_and_preparation_checks() {
    let classifier = LocalTextClassifier::new(configuration()).unwrap();
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let options = LocalClassificationOptions {
        cancel_token: Some(token),
        deadline_ms: None,
    };
    let error = classifier
        .classify("alpha", &["science".to_owned()], &options)
        .await
        .unwrap_err();
    assert!(matches!(error, crate::XbergError::Cancelled), "{error}");
}

#[tokio::test]
async fn an_expired_deadline_precedes_model_access() {
    let classifier = LocalTextClassifier::new(configuration()).unwrap();
    let options = LocalClassificationOptions {
        cancel_token: None,
        deadline_ms: Some(0),
    };
    let error = classifier
        .classify("alpha", &["science".to_owned()], &options)
        .await
        .unwrap_err();
    assert!(matches!(error, crate::XbergError::Timeout { .. }), "{error}");
}
