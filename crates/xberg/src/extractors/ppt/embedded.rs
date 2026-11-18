use super::PptExtractor;
use crate::core::config::ExtractionConfig;
use crate::extraction::ppt::{PptEmbeddedObject, PptReadCancellation};
use std::borrow::Cow;

const EMBEDDED_WARNING_SOURCE: &str = "ppt_embedded_objects";

pub(super) struct PptEmbeddingLimits {
    max_object_bytes: usize,
    max_objects: usize,
}

impl PptEmbeddingLimits {
    pub(super) fn from_config(config: &ExtractionConfig) -> Option<Self> {
        if config.max_archive_depth == 0 {
            return None;
        }
        let security = config.security_limits.clone().unwrap_or_default();
        Some(Self {
            max_object_bytes: config
                .max_embedded_file_bytes
                .unwrap_or(security.max_archive_size as u64) as usize,
            max_objects: security.max_files_in_archive,
        })
    }
}

#[derive(Default)]
pub(super) struct PptEmbeddings {
    objects: Vec<PptEmbeddedObject>,
    warnings: Vec<crate::types::ProcessingWarning>,
}

impl PptExtractor {
    /// Read the OLE storages from the same deck snapshot used by the slide parser.
    pub(super) fn discover_embedded_objects(
        content: &[u8],
        limits: Option<PptEmbeddingLimits>,
        cancel: PptReadCancellation<'_>,
    ) -> crate::Result<PptEmbeddings> {
        let Some(limits) = limits else {
            return Ok(PptEmbeddings::default());
        };
        let (mut objects, mut warnings) = match crate::extraction::ppt::extract_ppt_embedded_objects_with_cancellation(
            content,
            limits.max_object_bytes,
            cancel,
        ) {
            Ok(parsed) => parsed,
            Err(crate::XbergError::Cancelled) => return Err(crate::XbergError::Cancelled),
            Err(e) => {
                return Ok(PptEmbeddings {
                    objects: Vec::new(),
                    warnings: vec![crate::types::ProcessingWarning {
                        source: Cow::Borrowed(EMBEDDED_WARNING_SOURCE),
                        message: Cow::Owned(format!("Failed to read embedded objects: {e}")),
                    }],
                });
            }
        };
        if objects.len() > limits.max_objects {
            let skipped = objects.len() - limits.max_objects;
            warnings.push(crate::types::ProcessingWarning {
                source: Cow::Borrowed(EMBEDDED_WARNING_SOURCE),
                message: Cow::Owned(format!(
                    "Skipped {skipped} embedded object(s): max_files_in_archive ({}) reached",
                    limits.max_objects
                )),
            });
            objects.truncate(limits.max_objects);
        }

        Ok(PptEmbeddings { objects, warnings })
    }

    pub(super) async fn extract_embedded_objects(
        PptEmbeddings { objects, mut warnings }: PptEmbeddings,
        config: &ExtractionConfig,
    ) -> (Vec<crate::types::ArchiveEntry>, Vec<crate::types::ProcessingWarning>) {
        if objects.is_empty() {
            return (Vec::new(), warnings);
        }
        let mut child_config = config.clone();
        child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

        let mut children = Vec::new();
        for object in objects {
            let path = match object.slide_number {
                Some(slide) => format!("slide{slide}/oleObject{}.bin", object.ex_obj_id),
                None => format!("oleObject{}.bin", object.ex_obj_id),
            };
            let Some((inner_bytes, inner_mime)) =
                crate::extraction::ooxml_embedded::extract_ole_embedded_object(&object.data)
            else {
                warnings.push(crate::types::ProcessingWarning {
                    source: Cow::Borrowed(EMBEDDED_WARNING_SOURCE),
                    message: Cow::Owned(format!(
                        "Skipped embedded object '{path}': format identification not supported"
                    )),
                });
                continue;
            };
            tracing::debug!(target: "xberg::ppt::embedded_dispatch", "dispatching embedded OLE object");
            match crate::core::extractor::extract_bytes(&inner_bytes, &inner_mime, &child_config).await {
                Ok(result) => children.push(crate::types::ArchiveEntry {
                    path,
                    mime_type: inner_mime,
                    result: Box::new(result),
                }),
                Err(e) => warnings.push(crate::types::ProcessingWarning {
                    source: Cow::Borrowed(EMBEDDED_WARNING_SOURCE),
                    message: Cow::Owned(format!("Failed to extract embedded object '{path}': {e}")),
                }),
            }
        }
        (children, warnings)
    }
}
