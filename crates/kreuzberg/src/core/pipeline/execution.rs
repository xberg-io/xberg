//! Core processor execution logic.
//!
//! This module handles the execution of post-processors and validators
//! in the correct order.

use crate::core::config::ExtractionConfig;
use crate::plugins::ProcessingStage;
use crate::types::{ExtractionResult, ProcessingWarning};
use crate::{KreuzbergError, Result};
use std::borrow::Cow;

/// Execute all registered post-processors by stage.
pub(super) async fn execute_processors(
    result: &mut ExtractionResult,
    config: &ExtractionConfig,
    pp_config: &Option<&crate::core::config::PostProcessorConfig>,
    early_processors: std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    middle_processors: std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    late_processors: std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
) -> Result<()> {
    for (_stage, processors_arc) in [
        (ProcessingStage::Early, early_processors),
        (ProcessingStage::Middle, middle_processors),
        (ProcessingStage::Late, late_processors),
    ] {
        for processor in processors_arc.iter() {
            let processor_name = processor.name();

            let should_run = should_processor_run(pp_config, processor_name);

            if should_run && processor.should_process(result, config) {
                match processor.process(result, config).await {
                    Ok(_) => {}
                    Err(err @ KreuzbergError::Io(_))
                    | Err(err @ KreuzbergError::LockPoisoned(_))
                    | Err(err @ KreuzbergError::Plugin { .. }) => {
                        return Err(err);
                    }
                    Err(err) => {
                        let error_msg = err.to_string();
                        result.processing_warnings.push(ProcessingWarning {
                            source: Cow::Owned(processor_name.to_string()),
                            message: Cow::Owned(error_msg.clone()),
                        });
                        // DEPRECATED: kept for backward compatibility; will be removed in next major version.
                        result.metadata.additional.insert(
                            Cow::Owned(format!("processing_error_{processor_name}")),
                            serde_json::Value::String(error_msg),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

/// Determine if a processor should run based on configuration.
fn should_processor_run(pp_config: &Option<&crate::core::config::PostProcessorConfig>, processor_name: &str) -> bool {
    if let Some(config) = pp_config {
        if let Some(ref enabled_set) = config.enabled_set {
            enabled_set.contains(processor_name)
        } else if let Some(ref disabled_set) = config.disabled_set {
            !disabled_set.contains(processor_name)
        } else if let Some(ref enabled) = config.enabled_processors {
            enabled.iter().any(|name| name == processor_name)
        } else if let Some(ref disabled) = config.disabled_processors {
            !disabled.iter().any(|name| name == processor_name)
        } else {
            true
        }
    } else {
        true
    }
}

/// Execute all registered validators.
pub(super) async fn execute_validators(result: &ExtractionResult, config: &ExtractionConfig) -> Result<()> {
    let validator_registry = crate::plugins::registry::get_validator_registry();
    let validators = {
        let registry = validator_registry
            .read()
            .map_err(|e| crate::KreuzbergError::Other(format!("Validator registry lock poisoned: {}", e)))?;
        registry.get_all()
    };

    if !validators.is_empty() {
        for validator in validators {
            if validator.should_validate(result, config) {
                validator.validate(result, config).await?;
            }
        }
    }

    Ok(())
}
