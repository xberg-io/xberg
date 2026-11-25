//! Which `xberg-*` adapters `run` registers, and how they are built and registered.

use super::{XBERG_RUN_PIPELINES, should_register_xberg_pipeline};
use benchmark_harness::adapters::subprocess::SubprocessAdapter;
use benchmark_harness::adapters::xberg::{
    AutoOcrBackend, auto_ocr_framework_name, create_xberg_auto_ocr_adapter, pipeline_runs_layout, xberg_framework_name,
    xberg_steady_framework_name,
};
use benchmark_harness::adapters::{create_xberg_adapter, create_xberg_steady_adapter};
use benchmark_harness::{
    AdapterRegistry, BenchmarkConfig, FrameworkAdapter, OutputFormat, Result, XbergPdfBackend, XbergPipeline,
};
use std::sync::Arc;

/// Shared, read-only state for registering `xberg-*` adapter permutations -- bundled into one
/// struct purely to keep the registration helpers' parameter counts down; carries no behavior
/// of its own.
pub(super) struct XbergRegistration<'a> {
    pub(super) config: &'a BenchmarkConfig,
    pub(super) batch_mode: bool,
    pub(super) has_explicit_frameworks: bool,
    pub(super) ocr: bool,
    pub(super) format: OutputFormat,
    pub(super) should_init: &'a dyn Fn(&str) -> bool,
}

/// One adapter `run` will build and register.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PlannedAdapter {
    Pipeline {
        pipeline: XbergPipeline,
        pdf_backend: XbergPdfBackend,
    },
    SteadyState {
        pipeline: XbergPipeline,
        pdf_backend: XbergPdfBackend,
    },
    AutoOcr(AutoOcrBackend),
}

impl PlannedAdapter {
    pub(super) fn framework_name(self, format: OutputFormat, batch: bool) -> String {
        match self {
            PlannedAdapter::Pipeline { pipeline, pdf_backend } => {
                xberg_framework_name(pipeline, format, batch, pdf_backend)
            }
            PlannedAdapter::SteadyState { pipeline, pdf_backend } => {
                xberg_steady_framework_name(pipeline, format, batch, pdf_backend)
            }
            PlannedAdapter::AutoOcr(backend) => auto_ocr_framework_name(backend, format, batch),
        }
    }
}

/// Plain output discards the layout-detected headings, lists and tables, so a plaintext layout
/// run only pays the layout cost (the CLI warns about this in `overrides/layout.rs`).
pub(crate) fn plaintext_layout_is_a_no_op(format: OutputFormat, pipeline: XbergPipeline) -> bool {
    format == OutputFormat::Plaintext && pipeline_runs_layout(pipeline)
}

/// Rejects an explicit request for a plaintext layout variant. Such variants are never
/// registered, so the request would otherwise surface only as an unexplained "unavailable".
pub(super) fn reject_plaintext_layout_requests(frameworks: &[String]) -> Result<()> {
    let plaintext_layout = |framework: &str| {
        let Some(rest) = framework.strip_prefix("xberg-plaintext-") else {
            return false;
        };
        XBERG_RUN_PIPELINES.iter().any(|&pipeline| {
            let slug = pipeline.as_str();
            pipeline_runs_layout(pipeline)
                && (rest == slug || rest.strip_prefix(slug).is_some_and(|tail| tail.starts_with('-')))
        })
    };
    let rejected: Vec<&str> = frameworks
        .iter()
        .map(String::as_str)
        .filter(|framework| plaintext_layout(framework))
        .collect();
    if rejected.is_empty() {
        return Ok(());
    }
    Err(benchmark_harness::Error::Config(format!(
        "plaintext output discards layout structure, so plaintext layout variants are not benchmarked: {}; \
         use the markdown variant",
        rejected.join(", ")
    )))
}

fn pipeline_requires_ocr(pipeline: XbergPipeline) -> bool {
    matches!(
        pipeline,
        XbergPipeline::PaddleOcr
            | XbergPipeline::BaselinePaddle
            | XbergPipeline::LayoutPaddle
            | XbergPipeline::SceptreOrt
            | XbergPipeline::SceptreOrtLayout
            | XbergPipeline::SceptreOrtAutoRotate
            | XbergPipeline::SceptreTract
    )
}

fn has_steady_state_variant(pipeline: XbergPipeline) -> bool {
    matches!(
        pipeline,
        XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
    )
}

/// Every `xberg-*` adapter `run` builds for these selections, in registration order.
pub(super) fn plan_xberg_adapters(
    context: &XbergRegistration<'_>,
    effective_pdf_backends: &[XbergPdfBackend],
) -> Vec<PlannedAdapter> {
    // `Pdfium` is a distinct cohort dimension, not a pipeline: it is opt-in only (must be
    // requested explicitly via `--pdf-backends pdfium`) and restricted to `Baseline`, since the
    // pdfium engine has no OCR fallback path of its own. Every other pipeline always runs
    // `native`, regardless of `--pdf-backends`. ~keep
    let native_only_backend = [XbergPdfBackend::Native];
    let mut plan = Vec::new();
    for pipeline in XBERG_RUN_PIPELINES {
        if !should_register_xberg_pipeline(pipeline, context.has_explicit_frameworks)
            || plaintext_layout_is_a_no_op(context.format, pipeline)
            || (!context.ocr && pipeline_requires_ocr(pipeline))
        {
            continue;
        }
        let pdf_backends: &[XbergPdfBackend] = if pipeline == XbergPipeline::Baseline {
            effective_pdf_backends
        } else {
            &native_only_backend
        };
        for &pdf_backend in pdf_backends {
            plan.push(PlannedAdapter::Pipeline { pipeline, pdf_backend });
            if context.has_explicit_frameworks && has_steady_state_variant(pipeline) {
                plan.push(PlannedAdapter::SteadyState { pipeline, pdf_backend });
            }
        }
    }
    // Opt-in like the other unproven pipelines: a CI run has not yet shown the automatic OCR
    // trigger end to end, so a default run must not register them. ~keep
    if context.ocr && context.has_explicit_frameworks {
        plan.extend(AutoOcrBackend::ALL.into_iter().map(PlannedAdapter::AutoOcr));
    }
    plan.retain(|planned| (context.should_init)(&planned.framework_name(context.format, context.batch_mode)));
    plan
}

fn register_adapter(
    registry: &mut AdapterRegistry,
    framework_name: &str,
    created: Result<Arc<dyn FrameworkAdapter>>,
) -> bool {
    match created {
        Ok(adapter) => match registry.register(adapter) {
            Ok(()) => {
                tracing::info!(framework = %framework_name, "adapter registered");
                true
            }
            Err(error) => {
                tracing::warn!(framework = %framework_name, %error, "adapter registration failed");
                false
            }
        },
        Err(error) => {
            tracing::warn!(framework = %framework_name, %error, "adapter initialization failed");
            false
        }
    }
}

fn configure_subprocess_adapter(
    created: Result<SubprocessAdapter>,
    context: &XbergRegistration<'_>,
) -> Result<Arc<dyn FrameworkAdapter>> {
    created
        .map(|adapter| adapter.with_batch_workers(context.config.max_concurrent))
        .map(|adapter| match context.config.xberg_max_threads {
            Some(max_threads) => adapter.with_xberg_max_threads(max_threads),
            None => adapter,
        })
        .map(|adapter| Arc::new(adapter) as Arc<dyn FrameworkAdapter>)
}

fn build_planned_adapter(
    planned: PlannedAdapter,
    context: &XbergRegistration<'_>,
) -> Result<Arc<dyn FrameworkAdapter>> {
    match planned {
        PlannedAdapter::Pipeline { pipeline, pdf_backend } => configure_subprocess_adapter(
            create_xberg_adapter(pipeline, context.format, context.batch_mode, context.ocr, pdf_backend),
            context,
        ),
        PlannedAdapter::AutoOcr(backend) => configure_subprocess_adapter(
            create_xberg_auto_ocr_adapter(backend, context.format, context.batch_mode, context.ocr),
            context,
        ),
        PlannedAdapter::SteadyState { pipeline, pdf_backend } => create_xberg_steady_adapter(
            pipeline,
            context.format,
            context.batch_mode,
            context.ocr,
            pdf_backend,
            context.config.xberg_max_threads,
        )
        .map(|adapter| Arc::new(adapter) as Arc<dyn FrameworkAdapter>),
    }
}

/// Registers every planned `xberg-*` adapter and returns how many were registered.
pub(super) fn register_xberg_pipelines(
    registry: &mut AdapterRegistry,
    context: &XbergRegistration<'_>,
    effective_pdf_backends: &[XbergPdfBackend],
) -> usize {
    plan_xberg_adapters(context, effective_pdf_backends)
        .into_iter()
        .filter(|&planned| {
            let framework_name = planned.framework_name(context.format, context.batch_mode);
            register_adapter(registry, &framework_name, build_planned_adapter(planned, context))
        })
        .count()
}
