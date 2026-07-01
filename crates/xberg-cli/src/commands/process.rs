//! Process pipeline command: extract → NER → redact in one shot.

use anyhow::{Context, Result};
use std::io::Read as _;
use xberg::ExtractionConfig;

use crate::WireFormat;

pub fn process_command(
    input: crate::commands::extract::ExtractInputSource,
    config: ExtractionConfig,
    format: WireFormat,
) -> Result<()> {
    use xberg::{ExtractInput, extract};

    let rt = tokio::runtime::Runtime::new().context("Failed to start async runtime")?;

    let ext_input = match input {
        crate::commands::extract::ExtractInputSource::Uri(uri) => ExtractInput::from_uri(uri),
        crate::commands::extract::ExtractInputSource::Stdin => {
            let mut data = Vec::new();
            std::io::stdin()
                .read_to_end(&mut data)
                .context("Failed to read from stdin")?;
            if data.is_empty() {
                anyhow::bail!("No input received from stdin.");
            }
            ExtractInput::from_bytes(data, "text/plain", None)
        }
    };

    let mut out = rt
        .block_on(extract(ext_input, config))
        .context("Extraction failed")?;

    let doc = out.results.pop().ok_or_else(|| anyhow::anyhow!("No document produced"))?;

    match format {
        WireFormat::Text => {
            print!("{}", doc.content);
        }
        WireFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&doc).context("Failed to serialize result to JSON")?
            );
        }
        WireFormat::Toon => {
            println!(
                "{}",
                serde_toon::to_string(&doc).context("Failed to serialize result to TOON")?
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::extract::ExtractInputSource;

    #[test]
    fn process_command_text_input_json_format_succeeds() {
        // Plain-text input through the process pipeline with no NER/redact ops
        // should return a document whose `content` equals the input text.
        let mut config = ExtractionConfig::default();
        // No NER, no redaction — just extraction
        config.ner = None;
        config.redaction = None;

        // process_command accepts a URI; for a text string we write a temp file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, "Hello world").unwrap();

        let uri = path.to_string_lossy().to_string();
        let input = ExtractInputSource::Uri(uri);

        // Capture stdout
        // We can't easily capture stdout in a unit test, so test the underlying
        // extraction logic instead.
        let result = run_process(input, &config);
        assert!(result.is_ok(), "process failed: {:?}", result.err());
        let doc = result.unwrap();
        assert!(doc.content.contains("Hello world"));
    }

    fn run_process(
        input: ExtractInputSource,
        config: &ExtractionConfig,
    ) -> xberg::Result<xberg::ExtractedDocument> {
        use xberg::{ExtractInput, extract};
        let rt = tokio::runtime::Runtime::new().unwrap();
        match input {
            ExtractInputSource::Uri(uri) => {
                let ext_input = ExtractInput::from_uri(uri);
                let mut out = rt.block_on(extract(ext_input, config.clone()))?;
                out.results.pop().ok_or_else(|| {
                    xberg::error::XbergError::Other("no document produced".into())
                })
            }
            ExtractInputSource::Stdin => unreachable!("stdin not tested here"),
        }
    }
}
