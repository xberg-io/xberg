use crate::types::ExtractedDocument;
use blake3::Hasher;

const MAX_PROOF_BYTES: usize = 256 * 1024;

pub(in super::super) struct ReadProof([u8; 32]);

impl ReadProof {
    pub(in super::super) fn capture(read: &ExtractedDocument, scale_only: bool) -> Option<Self> {
        if !scale_only || read.raw_ocr_source_frame.is_some() || read.ocr_internal_document.is_none() {
            return None;
        }
        fingerprint(read).map(Self)
    }

    pub(in super::super) fn matches(&self, selected: &ExtractedDocument) -> bool {
        selected.raw_ocr_source_frame.is_none() && fingerprint(selected) == Some(self.0)
    }
}

fn fingerprint(read: &ExtractedDocument) -> Option<[u8; 32]> {
    let frame = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let mut sink = BoundedHash {
        hash: Hasher::new(),
        remaining: MAX_PROOF_BYTES,
    };
    // Bind the observed primary to the exact text geometry used to build accepted paragraphs. ~keep
    if let Err(error) = serde_json::to_writer(&mut sink, &(read.content.as_str(), &read.ocr_internal_document, frame)) {
        tracing::debug!(%error, "Cannot bind numeric source geometry to the selected OCR read");
        return None;
    }
    Some(*sink.hash.finalize().as_bytes())
}

struct BoundedHash {
    hash: Hasher,
    remaining: usize,
}

impl std::io::Write for BoundedHash {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("OCR geometry proof exceeds its byte bound"))?;
        self.hash.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
