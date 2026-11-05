use super::*;

#[tokio::test]
async fn nested_scopes_share_invalidation_but_next_operation_is_independent() {
    scope(async {
        assert!(is_cacheable());
        scope(async {
            invalidate_current();
        })
        .await;
        assert!(!is_cacheable());
    })
    .await;
    assert!(current().is_none());
    scope(async {
        assert!(is_cacheable());
    })
    .await;
}

#[tokio::test]
async fn unrelated_concurrent_operations_do_not_share_invalidation() {
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let first = barrier.clone();
    let second = barrier.clone();
    let (invalid, valid) = tokio::join!(
        scope(async move {
            invalidate_current();
            first.wait().await;
            is_cacheable()
        }),
        scope(async move {
            second.wait().await;
            is_cacheable()
        }),
    );
    assert!(!invalid);
    assert!(valid);
}

#[tokio::test]
async fn explicit_child_inheritance_shares_observations_and_invalidation() {
    scope(async {
        let context = current().unwrap();
        context.observe("root".into(), blake3::hash(b"one"));
        tokio::spawn(inherit(async {
            current().unwrap().observe("root".into(), blake3::hash(b"two"));
        }))
        .await
        .unwrap();
        assert!(!is_cacheable());
    })
    .await;
}

#[tokio::test]
async fn an_unwrapped_child_has_no_implicit_operation_state() {
    scope(async {
        invalidate_current();
        assert!(tokio::spawn(async { current().is_none() }).await.unwrap());
        assert!(!is_cacheable());
    })
    .await;
}

#[tokio::test]
async fn blocking_context_inherits_and_restores_after_normal_exit() {
    scope(async {
        let operation = blocking(|| {
            assert!(current().is_some());
            invalidate_current();
        });
        tokio::task::spawn_blocking(move || {
            assert!(BLOCKING_CONTEXT.with(|context| context.borrow().is_none()));
            operation();
            assert!(BLOCKING_CONTEXT.with(|context| context.borrow().is_none()));
        })
        .await
        .unwrap();
        assert!(!is_cacheable());
    })
    .await;
    assert!(BLOCKING_CONTEXT.with(|context| context.borrow().is_none()));
}

#[tokio::test]
async fn blocking_context_restores_the_previous_value_after_panic() {
    scope(async {
        let operation = blocking(|| panic!("controlled scope unwind"));
        let previous = Cacheability::default();
        BLOCKING_CONTEXT.with(|context| {
            context.replace(Some(previous.clone()));
        });
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
        assert!(result.is_err());
        let restored = BLOCKING_CONTEXT.with(|context| context.borrow().clone().unwrap());
        assert!(Arc::ptr_eq(&previous.0, &restored.0));
        BLOCKING_CONTEXT.with(|context| {
            context.replace(None);
        });
    })
    .await;
}

#[test]
fn repeated_roots_are_deduplicated_and_excess_distinct_roots_reject_retention() {
    let context = Cacheability::default();
    let revision = blake3::hash(b"one");
    for _ in 0..MAX_MODEL_ROOTS * 2 {
        context.observe("same".into(), revision);
    }
    assert_eq!(context.0.revisions.lock().unwrap().len(), 1);
    for index in 1..MAX_MODEL_ROOTS {
        context.observe(format!("root-{index}").into(), revision);
    }
    assert!(context.is_cacheable());
    context.observe("overflow".into(), revision);
    assert!(!context.is_cacheable());
    assert_eq!(context.0.revisions.lock().unwrap().len(), MAX_MODEL_ROOTS);
}

#[cfg(all(feature = "ocr", unix))]
#[tokio::test]
async fn model_read_failure_invalidates_outer_operation_even_after_restoration() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("osd.traineddata");
    std::fs::write(&path, b"one").unwrap();
    scope(async {
        osd_revision(root.path()).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&path, &path).unwrap();
        assert!(osd_revision(root.path()).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"one").unwrap();
        osd_revision(root.path()).unwrap();
        assert!(!is_cacheable());
    })
    .await;
}
