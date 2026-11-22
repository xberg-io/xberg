//! XMP metadata extraction from PDF documents.
//!
//! Extracts XMP (Extensible Metadata Platform) metadata from PDF documents.
//! XMP is XML-based metadata that provides richer information than the
//! traditional Info dictionary. See ISO 32000-1:2008, Section 14.3.2.
//!
//! ## XMP Namespaces
//!
//! XMP uses several standard namespaces:
//! - Dublin Core (dc): title, creator, description, etc.
//! - XMP Core (xmp): creation date, modify date, creator tool
//! - PDF (pdf): producer, keywords, trapped
//! - XMP Rights (xmpRights): usage terms, copyright

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;
use quick_xml::Reader;
use quick_xml::events::{BytesText, Event};
use std::borrow::Cow;
use std::collections::HashMap;

/// XMP metadata extracted from a PDF document.
#[derive(Debug, Clone, Default)]
pub struct XmpMetadata {
    /// Document title (dc:title)
    pub dc_title: Option<String>,
    /// Document creators/authors (dc:creator)
    pub dc_creator: Vec<String>,
    /// Document description (dc:description)
    pub dc_description: Option<String>,
    /// Subject keywords (dc:subject)
    pub dc_subject: Vec<String>,
    /// Document language (dc:language)
    pub dc_language: Option<String>,
    /// Copyright (dc:rights)
    pub dc_rights: Option<String>,
    /// Document format (dc:format)
    pub dc_format: Option<String>,

    /// Tool used to create the document (xmp:CreatorTool)
    pub xmp_creator_tool: Option<String>,
    /// Creation date (xmp:CreateDate)
    pub xmp_create_date: Option<String>,
    /// Last modification date (xmp:ModifyDate)
    pub xmp_modify_date: Option<String>,
    /// Metadata modification date (xmp:MetadataDate)
    pub xmp_metadata_date: Option<String>,

    /// PDF producer (pdf:Producer)
    pub pdf_producer: Option<String>,
    /// PDF keywords (pdf:Keywords)
    pub pdf_keywords: Option<String>,
    /// PDF version (pdf:PDFVersion)
    pub pdf_version: Option<String>,
    /// Whether the document has been trapped (pdf:Trapped)
    pub pdf_trapped: Option<String>,

    /// Usage terms (xmpRights:UsageTerms)
    pub xmp_rights_usage_terms: Option<String>,
    /// Whether marked with rights (xmpRights:Marked)
    pub xmp_rights_marked: Option<bool>,
    /// Web statement URL (xmpRights:WebStatement)
    pub xmp_rights_web_statement: Option<String>,

    /// Custom properties (namespace:property -> value)
    pub custom: HashMap<String, String>,

    /// Raw XMP packet (the original XML)
    pub raw_xml: Option<String>,
}

impl XmpMetadata {
    /// Create a new empty XMP metadata instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any metadata is present.
    pub fn is_empty(&self) -> bool {
        self.dc_title.is_none()
            && self.dc_creator.is_empty()
            && self.dc_description.is_none()
            && self.dc_subject.is_empty()
            && self.xmp_creator_tool.is_none()
            && self.xmp_create_date.is_none()
            && self.xmp_modify_date.is_none()
            && self.pdf_producer.is_none()
            && self.pdf_keywords.is_none()
            && self.custom.is_empty()
    }

    /// Set the document title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dc_title = Some(title.into());
        self
    }

    /// Add a creator/author.
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.dc_creator.push(creator.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.dc_description = Some(desc.into());
        self
    }

    /// Set the creator tool.
    pub fn with_creator_tool(mut self, tool: impl Into<String>) -> Self {
        self.xmp_creator_tool = Some(tool.into());
        self
    }

    /// Set the creation date (ISO 8601 format).
    pub fn with_create_date(mut self, date: impl Into<String>) -> Self {
        self.xmp_create_date = Some(date.into());
        self
    }

    /// Set the modification date (ISO 8601 format).
    pub fn with_modify_date(mut self, date: impl Into<String>) -> Self {
        self.xmp_modify_date = Some(date.into());
        self
    }

    /// Set the PDF producer.
    pub fn with_producer(mut self, producer: impl Into<String>) -> Self {
        self.pdf_producer = Some(producer.into());
        self
    }

    /// Add a custom property.
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }
}

/// XMP metadata extractor.
pub struct XmpExtractor;

/// XML event reader that restores whole text nodes after quick-xml splits them
/// around entity and character references.
struct XmpEventReader<'x> {
    reader: Reader<&'x [u8]>,
    pending: Option<Event<'x>>,
}

impl<'x> XmpEventReader<'x> {
    fn from_str(content: &'x str) -> Self {
        Self {
            reader: Reader::from_str(content),
            pending: None,
        }
    }

    fn read_event(&mut self) -> quick_xml::Result<Event<'x>> {
        let first = match self.pending.take() {
            Some(event) => event,
            None => self.reader.read_event()?,
        };
        let mut text = match first {
            Event::Text(text) => Cow::Borrowed(text.as_ref()).into_owned(),
            Event::GeneralRef(reference) => resolve_xmp_reference(&reference),
            other => return Ok(other),
        };

        loop {
            match self.reader.read_event()? {
                Event::Text(fragment) => text.push_str(&Cow::Borrowed(fragment.as_ref())),
                Event::GeneralRef(reference) => {
                    text.push_str(&resolve_xmp_reference(&reference));
                }
                other => {
                    self.pending = Some(other);
                    break;
                }
            }
        }

        Ok(Event::Text(BytesText::from_escaped(text)))
    }
}

fn resolve_xmp_reference(reference: &quick_xml::events::BytesRef<'_>) -> String {
    if let Ok(Some(character)) = reference.resolve_char_ref() {
        return character.to_string();
    }

    let name: Cow<'_, str> = Cow::Borrowed(reference.as_ref());
    if let Some(resolved) = quick_xml::escape::resolve_predefined_entity(&name) {
        return resolved.to_string();
    }

    format!("&{name};")
}

/// Decode an XMP packet to text, honouring a UTF-8/UTF-16 byte-order mark and the BOM-less UTF-16 forms
/// the XMP specification allows (a `<` followed or preceded by a NUL byte).
fn decode_xmp_bytes(bytes: &[u8]) -> String {
    if let Some((encoding, _)) = encoding_rs::Encoding::for_bom(bytes) {
        return encoding.decode_with_bom_removal(bytes).0.into_owned();
    }
    let encoding = match bytes {
        [0, b'<', ..] => Some(encoding_rs::UTF_16BE),
        [b'<', 0, ..] => Some(encoding_rs::UTF_16LE),
        _ => None,
    };
    match encoding {
        Some(encoding) => encoding.decode_without_bom_handling(bytes).0.into_owned(),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

impl XmpExtractor {
    /// Helper function to resolve an Object (handles indirect references).
    fn resolve_object(doc: &PdfDocument, obj: &Object) -> Result<Object> {
        if let Some(ref_val) = obj.as_reference() {
            doc.load_object(ref_val)
        } else {
            Ok(obj.clone())
        }
    }

    /// Extract XMP metadata from a PDF document.
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to extract XMP metadata from
    ///
    /// # Returns
    ///
    /// XMP metadata if present, or None if no XMP metadata exists.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::document::PdfDocument;
    /// use xberg_native_pdf::extractors::xmp::XmpExtractor;
    ///
    /// let mut doc = PdfDocument::open("document.pdf")?;
    /// if let Some(xmp) = XmpExtractor::extract(&mut doc)? {
    ///     if let Some(title) = &xmp.dc_title {
    ///         println!("Title: {}", title);
    ///     }
    ///     for creator in &xmp.dc_creator {
    ///         println!("Author: {}", creator);
    ///     }
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn extract(doc: &PdfDocument) -> Result<Option<XmpMetadata>> {
        let catalog = doc.catalog()?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".to_string()))?;

        let metadata_obj = match catalog_dict.get("Metadata") {
            Some(obj) => obj.clone(),
            None => return Ok(None),
        };

        let metadata_resolved = Self::resolve_object(doc, &metadata_obj)?;

        let xml_bytes = match &metadata_resolved {
            Object::Stream { .. } => match metadata_obj.as_reference() {
                Some(metadata_ref) => doc.decode_stream_with_encryption(&metadata_resolved, metadata_ref)?,
                None => metadata_resolved.decode_stream_data()?.to_vec(),
            },
            _ => return Err(Error::InvalidPdf("Metadata is not a stream".to_string())),
        };

        let xml_str = decode_xmp_bytes(&xml_bytes);

        Self::parse_xmp(&xml_str)
    }

    /// Parse XMP XML content.
    pub fn parse_xmp(xml: &str) -> Result<Option<XmpMetadata>> {
        // Prefer the `<x:xmpmeta>` wrapper, falling back to a bare
        // `<rdf:RDF>` root — but always search for a tag's closing marker
        // starting at THAT tag's own opening position, never independently
        // across both tag kinds. The previous code picked the start via a
        // forward `find` across both kinds and the end via a backward
        // `rfind` across both kinds; a stray closing-tag literal earlier in
        // the stream (of the *other* kind — e.g. leftover text mentioning
        // `</x:xmpmeta>` while the real content is a bare `<rdf:RDF>`)
        // could resolve to `end < start` and panic slicing
        // `xml[s..end_adjusted]` with "slice index starts at N but ends at
        // M". Scoping the closing search to `xml[open..]` makes that
        // impossible: any match found is guaranteed to be at or after
        // `open`. ~keep
        let xmp_content = match Self::find_wrapped_section(xml, "<x:xmpmeta", "</x:xmpmeta>")
            .or_else(|| Self::find_wrapped_section(xml, "<rdf:RDF", "</rdf:RDF>"))
        {
            Some(content) => content,
            None => return Ok(None),
        };

        let mut metadata = XmpMetadata::new();
        metadata.raw_xml = Some(xml.to_string());

        let mut reader = XmpEventReader::from_str(xmp_content);

        let mut element_stack: Vec<String> = Vec::new();
        let mut list_item_is_default = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name().as_ref().to_string();
                    match name.as_str() {
                        "rdf:Description" => apply_description_attributes(&mut metadata, &e),
                        "rdf:li" => list_item_is_default = has_default_language(&e),
                        _ => {}
                    }
                    element_stack.push(name);
                }
                Ok(Event::Empty(e)) => {
                    // Property attributes are the compact form of child property elements. ~keep
                    if e.name().as_ref() == "rdf:Description" {
                        apply_description_attributes(&mut metadata, &e);
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.xml11_content().trim().to_string();
                    if text.is_empty() {
                        continue;
                    }

                    // Find the relevant property element (skip rdf:li, rdf:Seq, rdf:Bag, rdf:Alt)
                    // ~keep
                    if let Some(prop) = current_xmp_property(&element_stack) {
                        apply_xmp_property_text(&mut metadata, &prop, text, list_item_is_default);
                    }
                }
                Ok(Event::End(_)) => {
                    if element_stack.pop().as_deref() == Some("rdf:li") {
                        list_item_is_default = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => {
                    tracing::warn!(
                        target: crate::LOG_TARGET_ROOT,
                        operation = "parse_xmp_metadata",
                        error_code = "malformed_xml",
                        error_offset = reader.reader.buffer_position(),
                        "stopping XMP metadata parsing"
                    );
                    break;
                }
                _ => {}
            }
        }

        Ok(Some(metadata))
    }

    /// Find the first `open .. close` wrapped section in `xml`, scoping the
    /// closing-tag search to start at `open`'s own match position so a
    /// result can never resolve to an end index before the start. See the
    /// call site in [`Self::parse_xmp`] for why this matters.
    fn find_wrapped_section<'a>(xml: &'a str, open: &str, close: &str) -> Option<&'a str> {
        let start = xml.find(open)?;
        let end_rel = xml[start..].find(close)?;
        Some(&xml[start..start + end_rel + close.len()])
    }
}

/// Returns the innermost element on the stack that isn't an RDF/XMP wrapper
/// tag (`rdf:li`, `rdf:Seq`, `rdf:Bag`, `rdf:Alt`, `x:xmpmeta`, …) — the
/// property a text node belongs to. Split out of [`XmpExtractor::parse_xmp`]
/// purely to keep that function's event loop short. ~keep
fn current_xmp_property(element_stack: &[String]) -> Option<String> {
    element_stack
        .iter()
        .rev()
        .find(|el| !el.starts_with("rdf:") && !el.starts_with("x:"))
        .cloned()
}

fn has_default_language(element: &quick_xml::events::BytesStart<'_>) -> bool {
    element
        .attributes()
        .flatten()
        .any(|attribute| attribute.key.as_ref() == "xml:lang" && attribute.value.as_ref() == "x-default")
}

/// Applies the property attributes of an `rdf:Description` (`<rdf:Description pdf:Producer="…"/>`), the
/// compact form of child property elements. ~keep
fn apply_description_attributes(metadata: &mut XmpMetadata, element: &quick_xml::events::BytesStart<'_>) {
    for attribute in element.attributes().flatten() {
        let key = attribute.key.as_ref();
        if key.starts_with("xmlns") || key.starts_with("xml:") || key.starts_with("rdf:") {
            continue;
        }
        let Ok(value) = attribute.normalized_value(quick_xml::XmlVersion::Implicit1_0) else {
            continue;
        };
        apply_xmp_property_text(metadata, key, value.trim().to_string(), false);
    }
}

/// Records a text node's value against the [`XmpMetadata`] field its
/// namespaced property name maps to, falling back to `custom` for anything
/// unrecognized. Split out of [`XmpExtractor::parse_xmp`] purely to keep that
/// function's event loop short. ~keep
fn apply_xmp_property_text(metadata: &mut XmpMetadata, prop: &str, text: String, is_default_language: bool) {
    // Language alternatives: the `x-default` entry wins over whichever entry came first. ~keep
    match prop {
        "dc:title" => {
            if is_default_language || metadata.dc_title.is_none() {
                metadata.dc_title = Some(text);
            }
        }
        "dc:creator" => {
            metadata.dc_creator.push(text);
        }
        "dc:description" => {
            if is_default_language || metadata.dc_description.is_none() {
                metadata.dc_description = Some(text);
            }
        }
        "dc:subject" => {
            metadata.dc_subject.push(text);
        }
        "dc:language" => metadata.dc_language = Some(text),
        "dc:rights" => {
            if is_default_language || metadata.dc_rights.is_none() {
                metadata.dc_rights = Some(text);
            }
        }
        "dc:format" => metadata.dc_format = Some(text),

        "xmp:CreatorTool" => metadata.xmp_creator_tool = Some(text),
        "xmp:CreateDate" => metadata.xmp_create_date = Some(text),
        "xmp:ModifyDate" => metadata.xmp_modify_date = Some(text),
        "xmp:MetadataDate" => metadata.xmp_metadata_date = Some(text),

        "pdf:Producer" => metadata.pdf_producer = Some(text),
        "pdf:Keywords" => metadata.pdf_keywords = Some(text),
        "pdf:PDFVersion" => metadata.pdf_version = Some(text),
        "pdf:Trapped" => metadata.pdf_trapped = Some(text),

        "xmpRights:UsageTerms" => {
            if is_default_language || metadata.xmp_rights_usage_terms.is_none() {
                metadata.xmp_rights_usage_terms = Some(text);
            }
        }
        "xmpRights:Marked" => {
            metadata.xmp_rights_marked = Some(text.to_lowercase() == "true");
        }
        "xmpRights:WebStatement" => metadata.xmp_rights_web_statement = Some(text),

        _ => {
            metadata.custom.insert(prop.to_string(), text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use tracing::Level;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::SubscriberExt as _;

    #[derive(Clone, Default)]
    struct EventCapture(Arc<Mutex<Vec<(Level, String, BTreeMap<String, String>)>>>);

    impl<S> Layer<S> for EventCapture
    where
        S: tracing::Subscriber,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
            let mut fields = FieldCapture::default();
            event.record(&mut fields);
            self.0.lock().unwrap().push((
                *event.metadata().level(),
                event.metadata().target().to_string(),
                fields.0,
            ));
        }
    }

    #[derive(Default)]
    struct FieldCapture(BTreeMap<String, String>);

    impl tracing::field::Visit for FieldCapture {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0.insert(field.name().to_string(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.insert(field.name().to_string(), value.to_string());
        }

        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.0.insert(field.name().to_string(), value.to_string());
        }
    }

    #[test]
    fn test_parse_xmp_basic() {
        let xmp = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:xmp="http://ns.adobe.com/xap/1.0/"
        xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
      <dc:title>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Test Document</rdf:li>
        </rdf:Alt>
      </dc:title>
      <dc:creator>
        <rdf:Seq>
          <rdf:li>John Doe</rdf:li>
          <rdf:li>Jane Smith</rdf:li>
        </rdf:Seq>
      </dc:creator>
      <xmp:CreatorTool>native_pdf</xmp:CreatorTool>
      <xmp:CreateDate>2024-01-15T10:30:00Z</xmp:CreateDate>
      <pdf:Producer>native_pdf 0.3.0</pdf:Producer>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

        let metadata = XmpExtractor::parse_xmp(xmp).unwrap().unwrap();

        assert_eq!(metadata.dc_title, Some("Test Document".to_string()));
        assert_eq!(metadata.dc_creator, vec!["John Doe", "Jane Smith"]);
        assert_eq!(metadata.xmp_creator_tool, Some("native_pdf".to_string()));
        assert_eq!(metadata.xmp_create_date, Some("2024-01-15T10:30:00Z".to_string()));
        assert_eq!(metadata.pdf_producer, Some("native_pdf 0.3.0".to_string()));
    }

    /// A stray `</x:xmpmeta>` literal appearing *before* the real content —
    /// which here uses a bare `<rdf:RDF>` root with no `<x:xmpmeta>` wrapper
    /// at all — used to make `parse_xmp` compute `start` (from a forward
    /// `find` across both tag kinds) and `end` (from a backward `rfind`
    /// across both tag kinds) inconsistently, giving `end < start`.
    ///
    /// Before the fix: `parse_xmp` panics with
    /// "slice index starts at 13 but ends at 12" instead of returning a
    /// `Result`. After the fix, the closing-tag search is scoped to start
    /// at the matched opening tag, so it returns `Ok` and recovers the real
    /// `<rdf:RDF>...</rdf:RDF>` content.
    #[test]
    fn parse_xmp_does_not_panic_on_closing_tag_before_real_content() {
        let malformed = "</x:xmpmeta>x<rdf:RDF>hello</rdf:RDF>";

        let result = XmpExtractor::parse_xmp(malformed);

        assert!(result.is_ok(), "malformed XMP data must not panic: {result:?}");
    }

    #[test]
    fn test_xmp_metadata_builder() {
        let metadata = XmpMetadata::new()
            .with_title("My Document")
            .with_creator("Author 1")
            .with_creator("Author 2")
            .with_description("A test document")
            .with_creator_tool("native_pdf")
            .with_producer("native_pdf 0.3.0");

        assert_eq!(metadata.dc_title, Some("My Document".to_string()));
        assert_eq!(metadata.dc_creator, vec!["Author 1", "Author 2"]);
        assert_eq!(metadata.dc_description, Some("A test document".to_string()));
        assert_eq!(metadata.xmp_creator_tool, Some("native_pdf".to_string()));
        assert_eq!(metadata.pdf_producer, Some("native_pdf 0.3.0".to_string()));
    }

    #[test]
    fn test_xmp_is_empty() {
        let empty = XmpMetadata::new();
        assert!(empty.is_empty());

        let non_empty = XmpMetadata::new().with_title("Title");
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_parse_xmp_with_subjects() {
        let xmp = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:subject>
        <rdf:Bag>
          <rdf:li>PDF</rdf:li>
          <rdf:li>Rust</rdf:li>
          <rdf:li>Metadata</rdf:li>
        </rdf:Bag>
      </dc:subject>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

        let metadata = XmpExtractor::parse_xmp(xmp).unwrap().unwrap();
        assert_eq!(metadata.dc_subject, vec!["PDF", "Rust", "Metadata"]);
    }

    #[test]
    fn parse_xmp_preserves_named_references_in_scalar_values() {
        let xmp = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
            xmlns:dc="http://purl.org/dc/elements/1.1/">
            <rdf:Description>
                <dc:description><rdf:Alt>
                    <rdf:li>&lt;div&gt;alpha &amp; beta&lt;/div&gt;</rdf:li>
                </rdf:Alt></dc:description>
            </rdf:Description>
        </rdf:RDF>"#;

        let metadata = XmpExtractor::parse_xmp(xmp).unwrap().unwrap();

        assert_eq!(metadata.dc_description, Some("<div>alpha & beta</div>".to_string()));
    }

    #[test]
    fn parse_xmp_preserves_numeric_references_in_scalar_values() {
        let xmp = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
            xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
            <rdf:Description>
                <pdf:Keywords>alpha&#32;beta&#x20AC;&#33;</pdf:Keywords>
            </rdf:Description>
        </rdf:RDF>"#;

        let metadata = XmpExtractor::parse_xmp(xmp).unwrap().unwrap();

        assert_eq!(metadata.pdf_keywords, Some("alpha beta€!".to_string()));
    }

    #[test]
    fn parse_xmp_coalesces_fragmented_sequence_values() {
        let xmp = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
            xmlns:dc="http://purl.org/dc/elements/1.1/">
            <rdf:Description>
                <dc:creator><rdf:Seq>
                    <rdf:li>Jane &amp; John &#35;1</rdf:li>
                    <rdf:li>Smith</rdf:li>
                </rdf:Seq></dc:creator>
            </rdf:Description>
        </rdf:RDF>"#;

        let metadata = XmpExtractor::parse_xmp(xmp).unwrap().unwrap();

        assert_eq!(metadata.dc_creator, vec!["Jane & John #1", "Smith"]);
    }

    #[test]
    fn parse_xmp_reads_attribute_form_properties() {
        let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
                xmlns:xmp="http://ns.adobe.com/xap/1.0/" pdf:Producer="Acme &amp; Sons"
                xmp:CreatorTool="Writer" xmp:CreateDate="2024-01-15T10:30:00Z"/>
        </rdf:RDF></x:xmpmeta>"#;

        let metadata = XmpExtractor::parse_xmp(xmp).unwrap().unwrap();

        assert_eq!(metadata.pdf_producer.as_deref(), Some("Acme & Sons"));
        assert_eq!(metadata.xmp_creator_tool.as_deref(), Some("Writer"));
        assert_eq!(metadata.xmp_create_date.as_deref(), Some("2024-01-15T10:30:00Z"));
        assert!(!metadata.custom.contains_key("rdf:about"));
    }

    #[test]
    fn parse_xmp_prefers_the_x_default_language_alternative() {
        for alternatives in [
            r#"<rdf:li xml:lang="de">Titel</rdf:li><rdf:li xml:lang="x-default">Title</rdf:li>"#,
            r#"<rdf:li xml:lang="x-default">Title</rdf:li><rdf:li xml:lang="de">Titel</rdf:li>"#,
        ] {
            let xmp = format!(
                r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:dc="http://purl.org/dc/elements/1.1/">
                <rdf:Description><dc:title><rdf:Alt>{alternatives}</rdf:Alt></dc:title></rdf:Description></rdf:RDF>"#
            );

            let metadata = XmpExtractor::parse_xmp(&xmp).unwrap().unwrap();

            assert_eq!(metadata.dc_title.as_deref(), Some("Title"), "{alternatives}");
        }
    }

    #[test]
    fn decode_xmp_bytes_sniffs_utf16_and_utf8_bom() {
        let text = "<x:xmpmeta>t\u{e4}st \u{20ac}</x:xmpmeta>";
        let utf16le_bom: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        let utf16be_bom_less: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let utf8_bom: Vec<u8> = [0xEF, 0xBB, 0xBF].into_iter().chain(text.bytes()).collect();

        assert_eq!(decode_xmp_bytes(&utf16le_bom), text);
        assert_eq!(decode_xmp_bytes(&utf16be_bom_less), text);
        assert_eq!(decode_xmp_bytes(&utf8_bom), text);
        assert_eq!(decode_xmp_bytes(text.as_bytes()), text);
    }

    #[test]
    fn malformed_xmp_warning_has_stable_identity_without_parser_text() {
        const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_XMP_ELEMENT_2fd6";
        let xmp = format!("<rdf:RDF><{CONFIDENTIAL_MARKER}></wrong></rdf:RDF>");
        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());

        let result = tracing::subscriber::with_default(subscriber, || XmpExtractor::parse_xmp(&xmp));

        assert!(result.is_ok());
        let events = capture.0.lock().unwrap().clone();
        let warnings: Vec<_> = events
            .iter()
            .filter(|(level, target, fields)| {
                *level == Level::WARN
                    && target == crate::LOG_TARGET_ROOT
                    && fields.get("operation").map(String::as_str) == Some("parse_xmp_metadata")
            })
            .collect();
        assert_eq!(warnings.len(), 1, "expected one XMP parser warning: {events:#?}");
        let fields = &warnings[0].2;
        assert_eq!(fields.get("error_code").map(String::as_str), Some("malformed_xml"));
        assert!(fields.contains_key("error_offset"));
        assert!(!fields.contains_key("error"));
        assert!(!format!("{events:?}").contains(CONFIDENTIAL_MARKER));
    }
}
