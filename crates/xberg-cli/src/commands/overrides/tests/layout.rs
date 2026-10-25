#[cfg(feature = "layout-detection")]
use super::super::*;
#[cfg(feature = "layout-detection")]
use super::default_overrides;
#[cfg(feature = "layout-detection")]
use xberg::ExtractionConfig;

#[cfg(feature = "layout-detection")]
#[test]
fn test_validate_layout_confidence_out_of_range() {
    let overrides = ExtractionOverrides {
        layout_confidence: Some(1.5),
        ..default_overrides()
    };
    assert!(overrides.validate().is_err());

    let overrides = ExtractionOverrides {
        layout_confidence: Some(-0.1),
        ..default_overrides()
    };
    assert!(overrides.validate().is_err());
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_validate_layout_confidence_valid() {
    let overrides = ExtractionOverrides {
        layout_confidence: Some(0.5),
        ..default_overrides()
    };
    assert!(overrides.validate().is_ok());
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_table_model_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        layout_table_model: Some("slanet_wired".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let layout = config.layout.unwrap();
    assert_eq!(layout.table_model, xberg::TableModel::SlanetWired);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_strategy_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        layout_strategy: Some("auto".to_string()),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let layout = config.layout.unwrap();
    assert_eq!(layout.strategy, xberg::LayoutStrategy::Auto);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_strategy_rejects_unknown_value() {
    let overrides = ExtractionOverrides {
        layout_strategy: Some("adaptive".to_string()),
        ..default_overrides()
    };
    let error = overrides.validate().expect_err("unknown strategy must fail");
    assert!(error.to_string().contains("Invalid layout strategy"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_strategy_conflicts_with_layout_false() {
    let overrides = ExtractionOverrides {
        layout: Some(false),
        layout_strategy: Some("auto".to_string()),
        ..default_overrides()
    };
    let error = overrides.validate().expect_err("conflicting flags must fail");
    assert!(error.to_string().contains("--layout-strategy"));
}

#[cfg(feature = "layout-detection")]
#[derive(clap::Parser)]
struct OverridesOnly {
    #[command(flatten)]
    overrides: ExtractionOverrides,
}

#[cfg(feature = "layout-detection")]
fn parse_overrides(args: &[&str]) -> Result<ExtractionOverrides, clap::Error> {
    use clap::Parser;
    OverridesOnly::try_parse_from(std::iter::once("xberg").chain(args.iter().copied())).map(|parsed| parsed.overrides)
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_pages_parsed_and_applied_alongside_strategy() {
    let overrides = parse_overrides(&["--layout-strategy", "auto", "--layout-pages", "3,1,3"]).expect("valid flags");
    overrides.validate().expect("page list with a strategy is valid");
    let mut config = ExtractionConfig::default();
    overrides.apply(&mut config);
    let layout = config.layout.expect("--layout-pages enables layout");
    assert_eq!(layout.pages, Some(vec![3, 1, 3]));
    assert_eq!(layout.strategy, xberg::LayoutStrategy::Auto);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_pages_alone_enables_layout() {
    let overrides = parse_overrides(&["--layout-pages", "2"]).expect("valid flags");
    overrides.validate().expect("a page list on its own is valid");
    let mut config = ExtractionConfig::default();
    overrides.apply(&mut config);
    let layout = config.layout.expect("--layout-pages alone enables layout");
    assert_eq!(layout.pages, Some(vec![2]));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_pages_rejects_non_numeric_entries() {
    for value in ["1-3", "a", "1,x", "4294967296"] {
        let error = match parse_overrides(&["--layout-pages", value]) {
            Ok(_) => panic!("{value} must not parse as a page list"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation, "{value}");
    }
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_pages_conflicts_with_layout_false() {
    let overrides = ExtractionOverrides {
        layout: Some(false),
        layout_pages: Some(vec![1]),
        ..default_overrides()
    };
    let error = overrides.validate().expect_err("conflicting flags must fail");
    assert!(error.to_string().contains("--layout-pages"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_layout_confidence_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        layout_confidence: Some(0.7),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let layout = config.layout.unwrap();
    assert_eq!(layout.confidence_threshold, Some(0.7));
}

/// Make every callsite in the process permanently interesting, once, so a
/// `tracing::warn!` reached on some other test's thread first isn't cached as
/// `Interest::never()` for the whole process and lost to `capture_logs` below.
/// Mirrors the pattern documented at `crates/xberg/src/cache/mod.rs` (#272/#301).
#[cfg(feature = "layout-detection")]
fn install_permissive_global_subscriber() {
    struct AlwaysInterested;

    impl tracing::Subscriber for AlwaysInterested {
        fn register_callsite(&self, _: &'static tracing::Metadata<'static>) -> tracing::subscriber::Interest {
            tracing::subscriber::Interest::always()
        }

        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
            Some(tracing::level_filters::LevelFilter::TRACE)
        }

        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::Id {
            tracing::Id::from_u64(1)
        }

        fn record(&self, _: &tracing::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::Id, _: &tracing::Id) {}
        fn event(&self, _: &tracing::Event<'_>) {}
        fn enter(&self, _: &tracing::Id) {}
        fn exit(&self, _: &tracing::Id) {}
    }

    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let _ = tracing::subscriber::set_global_default(AlwaysInterested);
        tracing::callsite::rebuild_interest_cache();
    });
}

/// Capture `tracing` output emitted on this thread while `body` runs.
#[cfg(feature = "layout-detection")]
fn capture_logs<T>(body: impl FnOnce() -> T) -> (T, String) {
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("log buffer poisoned").write_all(buf)?;
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let capture = Capture(Arc::clone(&buffer));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(move || capture.clone())
        .finish();

    install_permissive_global_subscriber();

    let value = tracing::subscriber::with_default(subscriber, body);
    let logs =
        String::from_utf8(buffer.lock().expect("log buffer poisoned").clone()).expect("log output must be UTF-8");
    (value, logs)
}

/// Regression test for contract point 4: enabling layout detection while
/// `output_format` stays `Plain` (the default) wastes the layout pass -- the extraction
/// still pays the model's cost (20s-202s per the WP-E measurements) and `Plain` never
/// renders the headings/lists/tables it detects. Before this warning was wired in
/// (i.e. removing the `warn_layout_wastes_plain_output` call from `apply`), `apply_layout`
/// never read `output_format` at all, so no warning was logged and this assertion on the
/// captured log output fails.
#[cfg(feature = "layout-detection")]
#[test]
fn test_warns_when_layout_enabled_with_plain_output_format() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        layout: Some(true),
        ..default_overrides()
    };

    let (_, logs) = capture_logs(|| overrides.apply(&mut config));

    assert!(config.layout.is_some(), "--layout must still enable layout detection");
    assert_eq!(
        config.output_format,
        xberg::OutputFormat::Plain,
        "Plain must stay the default"
    );
    assert!(
        logs.contains("layout detection is enabled but the output format is 'plain'"),
        "expected a layout/Plain contract warning in the captured log, got: {logs}"
    );
}

/// Companion guard: layout combined with a structured output format must not warn.
#[cfg(feature = "layout-detection")]
#[test]
fn test_does_not_warn_when_layout_enabled_with_markdown_output_format() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        layout: Some(true),
        content_format: Some(ContentOutputFormatArg::Markdown),
        ..default_overrides()
    };

    let (_, logs) = capture_logs(|| overrides.apply(&mut config));

    assert!(config.layout.is_some());
    assert!(
        !logs.contains("layout detection is enabled but the output format is 'plain'"),
        "no wasted-layout warning expected for markdown output, got: {logs}"
    );
}

/// Companion guard: `Plain` output alone, with layout left off (the double default),
/// must not warn.
#[cfg(feature = "layout-detection")]
#[test]
fn test_does_not_warn_when_layout_disabled_with_plain_output_format() {
    let mut config = ExtractionConfig::default();
    let overrides = default_overrides();

    let (_, logs) = capture_logs(|| overrides.apply(&mut config));

    assert!(config.layout.is_none(), "layout must stay off by default");
    assert!(
        !logs.contains("layout detection is enabled but the output format is 'plain'"),
        "no warning expected when layout is off, got: {logs}"
    );
}
