//! Post-processor plugin registration and management

use crate::{error_handling::{kreuzberg_error, runtime_error}, gc_guarded_value::GcGuardedValue, helpers::get_kw};
use magnus::{Error, Ruby, Value, scan_args::scan_args, TryConvert};
use magnus::value::ReprValue;
use std::sync::Arc;

/// Register a post-processor plugin
pub fn register_post_processor(args: &[Value]) -> Result<(), Error> {
    let _ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String, Value), (Option<i32>,), (), (), (), ()>(args)?;
    let (name, processor) = args.required;
    let (priority,) = args.optional;
    let priority = priority.unwrap_or(50);

    if !processor.respond_to("call", true)? {
        return Err(runtime_error("Post-processor must be a Proc or respond to 'call'"));
    }

    use async_trait::async_trait;
    use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};

    struct RubyPostProcessor {
        name: String,
        processor: GcGuardedValue,
    }

    unsafe impl Send for RubyPostProcessor {}
    unsafe impl Sync for RubyPostProcessor {}

    impl Plugin for RubyPostProcessor {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> kreuzberg::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> kreuzberg::Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for RubyPostProcessor {
        async fn process(
            &self,
            result: &mut kreuzberg::ExtractionResult,
            _config: &kreuzberg::ExtractionConfig,
        ) -> kreuzberg::Result<()> {
            let processor_name = self.name.clone();
            let processor = self.processor.value();
            let result_clone = result.clone();

            let updated_result = tokio::task::block_in_place(|| {
                let ruby = Ruby::get().expect("Ruby not initialized");
                let result_hash = crate::result::extraction_result_to_ruby(&ruby, result_clone.clone()).map_err(|e| {
                    kreuzberg::KreuzbergError::Plugin {
                        message: format!("Failed to convert result to Ruby: {}", e),
                        plugin_name: processor_name.clone(),
                    }
                })?;

                let modified = processor
                    .funcall::<_, _, magnus::Value>("call", (result_hash,))
                    .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Ruby post-processor failed: {}", e),
                        plugin_name: processor_name.clone(),
                    })?;

                let modified_hash =
                    magnus::RHash::try_convert(modified).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Post-processor must return a Hash: {}", e),
                        plugin_name: processor_name.clone(),
                    })?;

                let mut updated_result = result_clone;

                if let Some(content_val) = get_kw(&ruby, modified_hash, "content") {
                    let new_content =
                        String::try_convert(content_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                            message: format!("Failed to convert content: {}", e),
                            plugin_name: processor_name.clone(),
                        })?;
                    updated_result.content = new_content;
                }

                if let Some(mime_val) = get_kw(&ruby, modified_hash, "mime_type") {
                    let new_mime = String::try_convert(mime_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Failed to convert mime_type: {}", e),
                        plugin_name: processor_name.clone(),
                    })?;
                    updated_result.mime_type = new_mime;
                }

                Ok::<kreuzberg::ExtractionResult, kreuzberg::KreuzbergError>(updated_result)
            })?;

            *result = updated_result;
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    let processor_impl = Arc::new(RubyPostProcessor {
        name: name.clone(),
        processor: GcGuardedValue::new(processor),
    });

    let registry = kreuzberg::get_post_processor_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .register(processor_impl, priority)
        .map_err(kreuzberg_error)?;

    Ok(())
}
