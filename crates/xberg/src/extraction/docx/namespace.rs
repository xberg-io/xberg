//! Namespace-prefix canonicalization for WordprocessingML parts.
//!
//! The streaming parsers match literal qualified names (`w:p`, `w:val`, `r:id`, `m:oMath`,
//! `mc:Fallback`). XML lets a producer bind any prefix to a namespace, or none at all, so a part
//! declaring `xmlns:x="…/wordprocessingml/2006/main"` would match nothing and its text would be
//! dropped. [`canonicalize_prefixes`] rewrites element and attribute names whose namespace is one
//! of the known ones to the conventional prefix, once per part, before any parser sees it.
//!
//! Parts whose namespace declarations are all conventional (the overwhelming majority, drawings
//! with inline `xmlns:a` included) take a scan-only fast path and are returned borrowed.

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use std::borrow::Cow;
use std::collections::HashMap;

const TRANSITIONAL_WORDPROCESSINGML: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT_WORDPROCESSINGML: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const TRANSITIONAL_DRAWINGML: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const STRICT_DRAWINGML: &str = "http://purl.oclc.org/ooxml/drawingml/main";

/// Known namespace URIs and the prefix the parsers expect for each. Both the transitional and the
/// strict (ISO 29500 `purl.oclc.org`) URIs map to the same prefix.
const KNOWN_NAMESPACES: [(&str, &str); 7] = [
    (TRANSITIONAL_WORDPROCESSINGML, "w"),
    (STRICT_WORDPROCESSINGML, "w"),
    (
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
        "r",
    ),
    ("http://purl.oclc.org/ooxml/officeDocument/relationships", "r"),
    ("http://schemas.openxmlformats.org/officeDocument/2006/math", "m"),
    ("http://purl.oclc.org/ooxml/officeDocument/math", "m"),
    ("http://schemas.openxmlformats.org/markup-compatibility/2006", "mc"),
];

/// Prefixes the parsers rely on; a foreign namespace bound to one of them is renamed so it cannot
/// be mistaken for the real thing.
const RESERVED_PREFIXES: [&str; 4] = ["w", "r", "m", "mc"];

fn canonical_prefix(uri: &str) -> Option<&'static str> {
    KNOWN_NAMESPACES
        .iter()
        .find_map(|(known, prefix)| (*known == uri).then_some(*prefix))
}

/// Result of [`canonicalize_prefixes`].
pub(crate) struct Canonicalized<'a> {
    pub(crate) xml: Cow<'a, str>,
    /// The part declares a WordprocessingML namespace under a non-conventional prefix but could
    /// not be rewritten (malformed XML), so the parsers will see names they do not match.
    pub(crate) gave_up: bool,
}

/// Rewrite namespace prefixes in `xml` to the conventional `w`/`r`/`m`/`mc` set.
///
/// Returns the input borrowed when nothing needs rewriting. Malformed XML is returned unchanged
/// so the downstream parser reports or tolerates it as before; `gave_up` flags the case where
/// that loses content. Nesting depth is not limited here: the body parsers enforce the shared
/// budget identically whatever the prefix, and the pass is linear in the part size.
pub(crate) fn canonicalize_prefixes(xml: &str) -> Canonicalized<'_> {
    let Some(scan) = scan_declarations(xml) else {
        return Canonicalized {
            xml: Cow::Borrowed(xml),
            gave_up: false,
        };
    };
    match rewrite(xml) {
        Some(Some(rewritten)) => Canonicalized {
            xml: Cow::Owned(rewritten),
            gave_up: false,
        },
        Some(None) => Canonicalized {
            xml: Cow::Borrowed(xml),
            gave_up: false,
        },
        None => Canonicalized {
            xml: Cow::Borrowed(xml),
            gave_up: scan.binds_wordprocessingml,
        },
    }
}

/// Point the strict ISO 29500 WordprocessingML and DrawingML URIs at the transitional ones.
///
/// For parts read with `roxmltree`, which resolves namespaces itself and compares them against the
/// transitional URIs. Only namespace declarations are edited, so attribute values and text that
/// happen to contain a strict URI (a style ID, say) are left alone.
pub(crate) fn strict_to_transitional(xml: &str) -> Cow<'_, str> {
    let mut out: Option<String> = None;
    let mut copied = 0usize;
    for (index, _) in xml.match_indices("xmlns") {
        let Some((_, range)) = read_declaration(xml, index + "xmlns".len()) else {
            continue;
        };
        let transitional = match decode_references(&xml[range.clone()]).as_str() {
            STRICT_WORDPROCESSINGML => TRANSITIONAL_WORDPROCESSINGML,
            STRICT_DRAWINGML => TRANSITIONAL_DRAWINGML,
            _ => continue,
        };
        let out = out.get_or_insert_with(|| String::with_capacity(xml.len()));
        out.push_str(&xml[copied..range.start]);
        out.push_str(transitional);
        copied = range.end;
    }
    match out {
        Some(mut out) => {
            out.push_str(&xml[copied..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(xml),
    }
}

struct DeclarationScan {
    binds_wordprocessingml: bool,
}

/// Textual scan of every `xmlns` declaration in the part, without building a tree. `Some` when at
/// least one declaration (or anything that looks like one but cannot be read) is not conventional,
/// so a rewrite pass is needed. A declaration is conventional when a known URI is bound to its
/// conventional prefix, or an unknown URI to a prefix the parsers do not reserve; since every
/// declaration is checked, scoping cannot change that answer.
fn scan_declarations(xml: &str) -> Option<DeclarationScan> {
    let mut needs_rewrite = false;
    let mut binds_wordprocessingml = false;
    for (index, _) in xml.match_indices("xmlns") {
        let Some((prefix, range)) = read_declaration(xml, index + "xmlns".len()) else {
            needs_rewrite = true;
            continue;
        };
        let uri = decode_references(&xml[range]);
        if !declaration_is_canonical(prefix, &uri) {
            needs_rewrite = true;
            binds_wordprocessingml |= matches!(canonical_prefix(&uri), Some("w"));
        }
    }
    needs_rewrite.then_some(DeclarationScan { binds_wordprocessingml })
}

/// How far past an `xmlns` token a declaration may extend. Bounds every search so that text which
/// merely repeats `xmlns` cannot make the scan quadratic; real declarations are far shorter.
const DECLARATION_WINDOW: usize = 1024;

/// Parse `[:prefix] = "uri"` after the `xmlns` token that ends at `start`, returning the prefix
/// and the byte range of the raw URI text.
fn read_declaration(xml: &str, start: usize) -> Option<(&str, std::ops::Range<usize>)> {
    let bytes = xml.as_bytes();
    let window_end = bytes.len().min(start + DECLARATION_WINDOW);
    let mut at = start;
    let mut prefix = "";
    if bytes.get(at) == Some(&b':') {
        let name_start = at + 1;
        at = name_start;
        while at < window_end && is_name_byte(bytes[at]) {
            at += 1;
        }
        if at >= window_end {
            return None;
        }
        prefix = &xml[name_start..at];
    }
    while at < window_end && bytes[at].is_ascii_whitespace() {
        at += 1;
    }
    if bytes.get(at) != Some(&b'=') {
        return None;
    }
    at += 1;
    while at < window_end && bytes[at].is_ascii_whitespace() {
        at += 1;
    }
    let quote = *bytes.get(at).filter(|b| matches!(b, b'"' | b'\''))?;
    let value_start = at + 1;
    let length = bytes.get(value_start..window_end)?.iter().position(|b| *b == quote)?;
    Some((prefix, value_start..value_start + length))
}

/// Bytes allowed in a namespace prefix (an XML NCName; every non-ASCII byte is part of one).
fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') || byte >= 0x80
}

/// Resolve character and entity references in a declared URI.
fn decode_references(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    quick_xml::escape::unescape(raw).map_or_else(|_| raw.to_string(), Cow::into_owned)
}

/// `Some(prefix)` for an `xmlns` / `xmlns:prefix` attribute key, `""` for the default namespace.
fn declared_prefix(key: &str) -> Option<&str> {
    if key == "xmlns" {
        Some("")
    } else {
        key.strip_prefix("xmlns:")
    }
}

/// The declared URI with character and entity references resolved; the raw text is kept for output.
fn declared_uri(attr: &Attribute) -> String {
    attr.normalized_value(XmlVersion::Implicit1_0)
        .map_or_else(|_| attr.value.to_string(), Cow::into_owned)
}

fn declaration_is_canonical(prefix: &str, uri: &str) -> bool {
    match canonical_prefix(uri) {
        Some(canonical) => prefix == canonical,
        None => !RESERVED_PREFIXES.contains(&prefix),
    }
}

/// Prefix bindings in scope. Each prefix keeps its own stack so a lookup never walks past
/// unrelated declarations, however deeply they nest.
struct Scope {
    bindings: HashMap<String, Vec<String>>,
    log: Vec<String>,
    frames: Vec<usize>,
}

impl Scope {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            log: Vec::new(),
            frames: Vec::new(),
        }
    }

    fn declare(&mut self, prefix: &str, uri: String) {
        self.bindings.entry(prefix.to_string()).or_default().push(uri);
        self.log.push(prefix.to_string());
    }

    /// Drop every declaration made since the log held `mark` entries.
    fn rewind(&mut self, mark: usize) {
        while self.log.len() > mark {
            if let Some(prefix) = self.log.pop()
                && let Some(stack) = self.bindings.get_mut(&prefix)
            {
                stack.pop();
            }
        }
    }

    fn resolve(&self, prefix: &str) -> Option<&str> {
        self.bindings.get(prefix)?.last().map(String::as_str)
    }

    /// The name to emit for `name`, or `None` when it is already canonical (no allocation then).
    /// Unprefixed attributes belong to no namespace and are never rewritten.
    fn canonical_name(&self, name: &str, is_attribute: bool) -> Option<String> {
        let (prefix, local) = match name.split_once(':') {
            Some((prefix, local)) => (prefix, local),
            None if is_attribute => return None,
            None => ("", name),
        };
        if prefix == "xmlns" || prefix == "xml" {
            return None;
        }
        let target: Cow<'static, str> = match canonical_prefix(self.resolve(prefix)?) {
            Some(canonical) => Cow::Borrowed(canonical),
            None if RESERVED_PREFIXES.contains(&prefix) => Cow::Owned(format!("xberg_ns_{prefix}")),
            None => return None,
        };
        (target != prefix).then(|| format!("{target}:{local}"))
    }
}

/// Edits are copied into `out` lazily: nothing is allocated until the first rename is found.
struct Rewriter<'a> {
    xml: &'a str,
    scope: Scope,
    out: Option<String>,
    copied: usize,
}

impl<'a> Rewriter<'a> {
    fn replace(&mut self, before: usize, after: usize, text: &str) {
        let xml = self.xml;
        let out = self
            .out
            .get_or_insert_with(|| String::with_capacity(xml.len() + xml.len() / 16));
        out.push_str(&xml[self.copied..before]);
        out.push_str(text);
        self.copied = after;
    }

    /// Registers the element's declarations, then replaces the tag when anything in it needs a
    /// new name. `None` when its attributes cannot be read.
    fn start_tag(&mut self, e: &BytesStart, is_empty: bool, before: usize, after: usize) -> Option<()> {
        let mark = self.scope.log.len();
        for attr in e.attributes() {
            let attr = attr.ok()?;
            if let Some(prefix) = declared_prefix(attr.key.as_ref()) {
                self.scope.declare(prefix, declared_uri(&attr));
            }
        }
        let new_name = self.scope.canonical_name(e.name().as_ref(), false);
        let mut attribute_renamed = false;
        for attr in e.attributes() {
            let attr = attr.ok()?;
            if self.scope.canonical_name(attr.key.as_ref(), true).is_some() {
                attribute_renamed = true;
                break;
            }
        }
        if new_name.is_some() || attribute_renamed {
            let tag = self.tag_text(e, new_name, is_empty)?;
            self.replace(before, after, &tag);
        }
        if is_empty {
            self.scope.rewind(mark);
        } else {
            self.scope.frames.push(mark);
        }
        Some(())
    }

    /// The tag with renamed element and attributes. When renaming makes two attributes share a
    /// name, the first is kept and the later ones are dropped.
    fn tag_text(&self, e: &BytesStart, new_name: Option<String>, is_empty: bool) -> Option<String> {
        let mut tag = String::from("<");
        tag.push_str(new_name.as_deref().unwrap_or(e.name().as_ref()));
        let mut seen: Vec<String> = Vec::new();
        for attr in e.attributes() {
            let attr = attr.ok()?;
            let key = self
                .scope
                .canonical_name(attr.key.as_ref(), true)
                .unwrap_or_else(|| attr.key.as_ref().to_string());
            if seen.contains(&key) {
                continue;
            }
            let value = attr.value.as_ref();
            let quote = if value.contains('"') { '\'' } else { '"' };
            tag.push(' ');
            tag.push_str(&key);
            tag.push('=');
            tag.push(quote);
            tag.push_str(value);
            tag.push(quote);
            seen.push(key);
        }
        tag.push_str(if is_empty { "/>" } else { ">" });
        Some(tag)
    }

    fn end_tag(&mut self, name: &str, before: usize, after: usize) {
        if let Some(new_name) = self.scope.canonical_name(name, false) {
            self.replace(before, after, &format!("</{new_name}>"));
        }
        if let Some(mark) = self.scope.frames.pop() {
            self.scope.rewind(mark);
        }
    }
}

/// `Some(None)` when nothing needed renaming, `None` when the XML could not be read.
fn rewrite(xml: &str) -> Option<Option<String>> {
    let mut reader = Reader::from_str(xml);
    let mut rewriter = Rewriter {
        xml,
        scope: Scope::new(),
        out: None,
        copied: 0,
    };
    loop {
        let before = usize::try_from(reader.buffer_position()).ok()?;
        let event = reader.read_event().ok()?;
        let after = usize::try_from(reader.buffer_position()).ok()?;
        match event {
            Event::Start(e) => rewriter.start_tag(&e, false, before, after)?,
            Event::Empty(e) => rewriter.start_tag(&e, true, before, after)?,
            Event::End(e) => rewriter.end_tag(e.name().as_ref(), before, after),
            Event::Eof => break,
            _ => {}
        }
    }
    let Rewriter { out, copied, .. } = rewriter;
    Some(out.map(|mut out| {
        out.push_str(&xml[copied..]);
        out
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WML: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    fn canon(xml: &str) -> Cow<'_, str> {
        canonicalize_prefixes(xml).xml
    }

    #[test]
    fn conventional_prefixes_are_returned_borrowed() {
        let xml = format!(r#"<w:document xmlns:w="{WML}"><w:body><w:p/></w:body></w:document>"#);
        assert!(matches!(canon(&xml), Cow::Borrowed(_)));
    }

    #[test]
    fn alternate_prefix_is_rewritten_for_elements_and_attributes() {
        let xml = format!(r#"<x:document xmlns:x="{WML}"><x:b x:val='0'/><x:t>a &amp; b</x:t></x:document>"#);
        assert_eq!(
            canon(&xml),
            format!(r#"<w:document xmlns:x="{WML}"><w:b w:val="0"/><w:t>a &amp; b</w:t></w:document>"#)
        );
    }

    #[test]
    fn default_namespace_elements_gain_the_conventional_prefix() {
        let xml = format!(r#"<document xmlns="{WML}" xmlns:w="{WML}"><p><r w:val="1"/></p></document>"#);
        assert_eq!(
            canon(&xml),
            format!(r#"<w:document xmlns="{WML}" xmlns:w="{WML}"><w:p><w:r w:val="1"/></w:p></w:document>"#)
        );
    }

    #[test]
    fn strict_namespace_maps_to_the_conventional_prefix() {
        let xml = r#"<s:p xmlns:s="http://purl.oclc.org/ooxml/wordprocessingml/main"/>"#;
        assert_eq!(
            canon(xml),
            r#"<w:p xmlns:s="http://purl.oclc.org/ooxml/wordprocessingml/main"/>"#
        );
    }

    #[test]
    fn foreign_namespace_bound_to_a_reserved_prefix_is_renamed() {
        let xml = format!(r#"<x:p xmlns:x="{WML}" xmlns:w="urn:other"><w:t>no</w:t></x:p>"#);
        assert_eq!(
            canon(&xml),
            format!(r#"<w:p xmlns:x="{WML}" xmlns:w="urn:other"><xberg_ns_w:t>no</xberg_ns_w:t></w:p>"#)
        );
    }

    #[test]
    fn nested_redeclaration_is_resolved_in_scope() {
        let xml = format!(r#"<w:body xmlns:w="{WML}"><q:p xmlns:q="{WML}"><q:t/></q:p><w:p/></w:body>"#);
        assert_eq!(
            canon(&xml),
            format!(r#"<w:body xmlns:w="{WML}"><w:p xmlns:q="{WML}"><w:t/></w:p><w:p/></w:body>"#)
        );
    }

    #[test]
    fn malformed_xml_is_returned_unchanged() {
        let xml = format!(r#"<x:document xmlns:x="{WML}"><x:p></y:q>"#);
        assert_eq!(canon(&xml), xml);
    }

    #[derive(Clone, Copy)]
    enum Flavor {
        Conventional,
        Alias,
        DefaultNamespace,
        Strict,
    }

    const STRICT_WML: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
    const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    const STRICT_REL: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";
    const MATH: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
    const STRICT_MATH: &str = "http://purl.oclc.org/ooxml/officeDocument/math";
    const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

    /// Turn a template written with `w:`/`r:` into `flavor`, declaring the namespaces on the root.
    fn render(template: &str, flavor: Flavor) -> String {
        let (element, attribute, root_decl) = match flavor {
            Flavor::Conventional => ("w:", "w:", format!(r#"xmlns:w="{WML}""#)),
            Flavor::Alias => ("x:", "x:", format!(r#"xmlns:x="{WML}""#)),
            Flavor::DefaultNamespace => ("", "w:", format!(r#"xmlns="{WML}" xmlns:w="{WML}""#)),
            Flavor::Strict => ("s:", "s:", format!(r#"xmlns:s="{STRICT_WML}""#)),
        };
        let conventional = matches!(flavor, Flavor::Conventional);
        let (relationship, markup, math) = if conventional {
            ("r", "mc", "m")
        } else {
            ("rel", "mcx", "mth")
        };
        let (rel_uri, math_uri) = if matches!(flavor, Flavor::Strict) {
            (STRICT_REL, STRICT_MATH)
        } else {
            (REL, MATH)
        };
        let declarations =
            format!(r#"{root_decl} xmlns:{relationship}="{rel_uri}" xmlns:{markup}="{MC}" xmlns:{math}="{math_uri}""#);
        template
            .replace("@NS@", &declarations)
            .replace("<w:", &format!("<{element}"))
            .replace("</w:", &format!("</{element}"))
            .replace(" w:", &format!(" {attribute}"))
            .replace(" r:id", &format!(" {relationship}:id"))
            .replace("<mc:", &format!("<{markup}:"))
            .replace("</mc:", &format!("</{markup}:"))
            .replace("<m:", &format!("<{math}:"))
            .replace("</m:", &format!("</{math}:"))
    }

    const DOCUMENT: &str = r#"<w:document @NS@><w:body>
        <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>
        <w:p><w:r><w:rPr><w:b/><w:i/></w:rPr><w:t xml:space="preserve">bold italic </w:t></w:r>
          <w:r><w:rPr><w:b w:val="0"/><w:u w:val="single"/></w:rPr><w:t>plain underlined</w:t></w:r>
          <w:hyperlink r:id="rId5"><w:r><w:t>link</w:t></w:r></w:hyperlink>
          <w:r><w:footnoteReference w:id="2"/></w:r><w:r><w:commentReference w:id="7"/></w:r></w:p>
        <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>first item</w:t></w:r></w:p>
        <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>second item</w:t></w:r></w:p>
        <w:tbl><w:tr><w:tc><w:p><w:r><w:t>kept</w:t></w:r></w:p></w:tc></w:tr>
          <w:tr><w:trPr><w:del w:id="1" w:author="A"/></w:trPr><w:tc><w:p><w:r><w:t>removed row</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
        <w:p><mc:AlternateContent><mc:Choice Requires="wps"><w:r><w:t>choice text</w:t></w:r></mc:Choice><mc:Fallback><w:r><w:t>fallback text</w:t></w:r></mc:Fallback></mc:AlternateContent></w:p>
        <w:p><m:oMath><m:r><m:t>xy</m:t></m:r></m:oMath></w:p>
        <w:p><w:ins w:id="3" w:author="B"><w:r><w:t>inserted</w:t></w:r></w:ins></w:p>
        <w:sectPr><w:pgSz w:w="12240" w:h="15840" w:orient="portrait"/><w:headerReference r:id="rId9"/></w:sectPr>
      </w:body></w:document>"#;

    const NUMBERING: &str = r#"<w:numbering @NS@>
        <w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl></w:abstractNum>
        <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#;

    const FOOTNOTES: &str = r#"<w:footnotes @NS@>
        <w:footnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:footnote>
        <w:footnote w:id="2"><w:p><w:r><w:t>the footnote</w:t></w:r></w:p></w:footnote></w:footnotes>"#;

    const COMMENTS: &str = r#"<w:comments @NS@>
        <w:comment w:id="7" w:author="Reviewer"><w:p><w:r><w:t>the comment</w:t></w:r></w:p></w:comment></w:comments>"#;

    const HEADER: &str = r#"<w:hdr @NS@><w:p><w:r><w:t>the header</w:t></w:r></w:p></w:hdr>"#;

    const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/" TargetMode="External"/>
        <Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
        </Relationships>"#;

    fn zip_of(parts: &[(&str, String)]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (path, content) in parts {
            zip.start_file(*path, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn build_package(flavors: [Flavor; 5], rels: &str) -> Vec<u8> {
        let templates = [
            ("word/document.xml", DOCUMENT),
            ("word/numbering.xml", NUMBERING),
            ("word/footnotes.xml", FOOTNOTES),
            ("word/comments.xml", COMMENTS),
            ("word/header1.xml", HEADER),
        ];
        let mut parts: Vec<(&str, String)> = templates
            .into_iter()
            .zip(flavors)
            .map(|((path, template), flavor)| (path, render(template, flavor)))
            .collect();
        parts.push(("word/_rels/document.xml.rels", rels.to_string()));
        zip_of(&parts)
    }

    fn parse_with_rels(flavors: [Flavor; 5], rels: &str) -> String {
        use crate::extractors::security::{SecurityBudget, SecurityLimits};
        let mut budget = SecurityBudget::with_defaults();
        let document = crate::extraction::docx::parser::parse_document(
            &build_package(flavors, rels),
            &mut budget,
            &SecurityLimits::default(),
        )
        .expect("package parses");
        // Hash-map fields have no stable Debug order, so list labels are sorted and the
        // remaining maps are covered through `extract_text` and the ordered fields.
        let list_labels: std::collections::BTreeMap<_, _> = document.list_labels.iter().collect();
        format!(
            "{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{list_labels:?}\n{}",
            document.paragraphs,
            document.tables,
            document.headers,
            document.footnotes,
            document.comments,
            document.revisions,
            document.elements,
            document.sections,
            document.extract_text()
        )
    }

    fn parse(flavors: [Flavor; 5]) -> String {
        parse_with_rels(flavors, RELS)
    }

    #[test]
    fn conventional_package_extracts_every_part() {
        let output = parse([Flavor::Conventional; 5]);
        for expected in [
            "Title",
            "bold italic ",
            "plain underlined",
            "https://example.com/",
            "first item",
            "second item",
            "kept",
            "inserted",
            "the footnote",
            "the comment",
            "the header",
            "choice text",
            "xy",
        ] {
            assert!(output.contains(expected), "{expected:?} missing from {output}");
        }
        assert!(!output.contains("removed row"), "deleted row leaked: {output}");
    }

    #[test]
    fn every_part_is_identical_under_alias_default_and_strict_namespaces() {
        let baseline = parse([Flavor::Conventional; 5]);
        for flavor in [Flavor::Alias, Flavor::DefaultNamespace, Flavor::Strict] {
            assert_eq!(parse([flavor; 5]), baseline);
        }
    }

    #[test]
    fn parts_may_use_different_prefixes_from_each_other() {
        let baseline = parse([Flavor::Conventional; 5]);
        let mixed = [
            Flavor::Alias,
            Flavor::Conventional,
            Flavor::DefaultNamespace,
            Flavor::Strict,
            Flavor::Alias,
        ];
        assert_eq!(parse(mixed), baseline);
    }

    #[test]
    fn one_part_may_mix_two_prefixes_for_the_same_namespace() {
        let xml = format!(
            r#"<w:document xmlns:w="{WML}" xmlns:x="{WML}"><w:body><w:p><x:r><x:t>mixed</x:t></x:r></w:p></w:body></w:document>"#
        );
        assert_eq!(
            canon(&xml),
            format!(
                r#"<w:document xmlns:w="{WML}" xmlns:x="{WML}"><w:body><w:p><w:r><w:t>mixed</w:t></w:r></w:p></w:body></w:document>"#
            )
        );
    }

    #[test]
    fn declaration_written_with_character_references_still_names_the_namespace() {
        let xml = r#"<x:p xmlns:x="http://schemas.openxmlformats.org/wordprocessingml/2006/ma&#105;n"/>"#;
        assert_eq!(
            canon(xml),
            r#"<w:p xmlns:x="http://schemas.openxmlformats.org/wordprocessingml/2006/ma&#105;n"/>"#
        );
    }

    #[test]
    fn strict_styles_namespace_is_pointed_at_the_transitional_uri() {
        let strict = format!(
            r#"<w:styles xmlns:w="{STRICT_WORDPROCESSINGML}"><w:style w:type="paragraph" w:styleId="H1"/></w:styles>"#
        );
        let catalog = crate::extraction::docx::styles::parse_styles_xml(&strict_to_transitional(&strict)).unwrap();
        assert!(catalog.styles.contains_key("H1"));
        let unconverted = crate::extraction::docx::styles::parse_styles_xml(&strict).unwrap();
        assert!(
            unconverted.styles.is_empty(),
            "control: strict URI alone must not match"
        );
    }

    const DRAWINGML: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

    #[test]
    fn inline_declarations_of_unrelated_prefixes_keep_the_fast_path() {
        let xml = format!(
            r#"<w:document xmlns:w="{WML}" xmlns:r="{REL}"><w:body><w:p><w:r><w:drawing><a:graphic xmlns:a="{DRAWINGML}"><a:graphicData xmlns:pic="urn:pic"><pic:pic/></a:graphicData></a:graphic></w:drawing></w:r></w:p><w:p xmlns:w="{WML}"/></w:body></w:document>"#
        );
        assert!(matches!(canon(&xml), Cow::Borrowed(_)));
    }

    #[test]
    fn rewrite_pass_that_changes_nothing_does_not_copy_the_part() {
        let xml = format!(r#"<w:document xmlns:w="{WML}" xmlns:w2="urn:other"><w:p/></w:document>"#);
        assert!(matches!(rewrite(&xml), Some(None)));
        let unreadable = format!(r#"<x:d xmlns:x="{WML}"><x:p x:a=></x:d>"#);
        assert!(rewrite(&unreadable).is_none());
    }

    #[test]
    fn tags_are_told_apart_by_event_not_by_trailing_slash() {
        let xml = format!(r#"<x:p xmlns:x="{WML}" x:v="a/"><x:q x:v='/'>t/</x:q><x:r x:v="b/"/></x:p>"#);
        assert_eq!(
            canon(&xml),
            format!(r#"<w:p xmlns:x="{WML}" w:v="a/"><w:q w:v="/">t/</w:q><w:r w:v="b/"/></w:p>"#)
        );
    }

    #[test]
    fn attributes_that_collide_after_renaming_keep_the_first() {
        let xml = format!(r#"<x:b x:val="1" w:val="2" xmlns:w="{WML}" xmlns:x="{WML}"/>"#);
        assert_eq!(
            canon(&xml),
            format!(r#"<w:b w:val="1" xmlns:w="{WML}" xmlns:x="{WML}"/>"#)
        );
    }

    #[test]
    fn everything_but_names_is_preserved_byte_for_byte() {
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE d>\n<?pi some data?>\n<!-- <x:p> stays -->\n\
             <x:d xmlns:x=\"{WML}\" x:a=\"&amp;\" x:b=\"&quot;\" x:c='say \"hi\"' x:d=\"it's\">\
             <![CDATA[ <x:p> & ]]>  <x:t>a &lt; b</x:t>\n</x:d>"
        );
        assert_eq!(
            canon(&xml),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE d>\n<?pi some data?>\n<!-- <x:p> stays -->\n\
                 <w:d xmlns:x=\"{WML}\" w:a=\"&amp;\" w:b=\"&quot;\" w:c='say \"hi\"' w:d=\"it's\">\
                 <![CDATA[ <x:p> & ]]>  <w:t>a &lt; b</w:t>\n</w:d>"
            )
        );
    }

    #[test]
    fn two_foreign_namespaces_on_one_reserved_prefix_are_both_renamed() {
        let xml =
            format!(r#"<x:p xmlns:x="{WML}" xmlns:w="urn:1"><w:a/><w:b xmlns:w="urn:2"><w:c/></w:b><w:d/></x:p>"#);
        assert_eq!(
            canon(&xml),
            format!(
                r#"<w:p xmlns:x="{WML}" xmlns:w="urn:1"><xberg_ns_w:a/><xberg_ns_w:b xmlns:w="urn:2"><xberg_ns_w:c/></xberg_ns_w:b><xberg_ns_w:d/></w:p>"#
            )
        );
    }

    #[test]
    fn giving_up_is_reported_only_when_wordprocessingml_is_bound_to_another_prefix() {
        let alias = format!(r#"<x:d xmlns:x="{WML}"><x:p></y:q>"#);
        assert!(canonicalize_prefixes(&alias).gave_up);
        let conventional = format!(r#"<w:d xmlns:w="{WML}" xmlns:w2="urn:o"><w:p></y:q>"#);
        assert!(!canonicalize_prefixes(&conventional).gave_up);
        let unrelated = r#"<d xmlns:w="urn:o"><p></y:q>"#;
        assert!(!canonicalize_prefixes(unrelated).gave_up);
    }

    #[test]
    fn unreadable_alias_part_surfaces_a_warning() {
        use crate::extractors::security::{SecurityBudget, SecurityLimits};
        let document = format!(
            r#"<x:document xmlns:x="{WML}"><x:body><x:p><x:r><x:t a=b>lost</x:t></x:r></x:p></x:body></x:document>"#
        );
        let bytes = zip_of(&[("word/document.xml", document)]);
        let mut budget = SecurityBudget::with_defaults();
        let parsed =
            crate::extraction::docx::parser::parse_document(&bytes, &mut budget, &SecurityLimits::default()).unwrap();
        assert!(
            parsed.warnings.iter().any(|w| w.message.contains("word/document.xml")),
            "{:?}",
            parsed.warnings
        );
    }

    #[test]
    fn too_deep_part_has_the_same_outcome_whatever_the_prefix() {
        use crate::extractors::security::{SecurityBudget, SecurityLimits};
        let outcome = |prefix: &str| {
            let open = format!("<{prefix}:p xmlns:n=\"urn:n\">").repeat(5000);
            let close = format!("</{prefix}:p>").repeat(5000);
            let footnotes = format!(
                r#"<{prefix}:footnotes xmlns:{prefix}="{WML}"><{prefix}:footnote {prefix}:id="2">{open}{close}</{prefix}:footnote></{prefix}:footnotes>"#
            );
            let document = format!(r#"<{prefix}:document xmlns:{prefix}="{WML}"><{prefix}:body/></{prefix}:document>"#);
            let bytes = zip_of(&[("word/document.xml", document), ("word/footnotes.xml", footnotes)]);
            let mut budget = SecurityBudget::with_defaults();
            crate::extraction::docx::parser::parse_document(&bytes, &mut budget, &SecurityLimits::default())
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        let conventional = outcome("w");
        assert!(conventional.is_err(), "control: the body parser must reject this");
        assert_eq!(outcome("x"), conventional);
    }

    #[test]
    fn strict_relationships_resolve_the_same_targets() {
        let strict_rels = RELS.replace(REL, STRICT_REL);
        assert_ne!(strict_rels, RELS, "control: the fixture must actually change");
        assert_eq!(
            parse_with_rels([Flavor::Strict; 5], &strict_rels),
            parse([Flavor::Conventional; 5])
        );
    }

    #[test]
    fn strict_drawingml_theme_is_read() {
        let strict = r#"<a:theme xmlns:a="http://purl.oclc.org/ooxml/drawingml/main" name="T"><a:themeElements><a:clrScheme name="c"><a:dk1><a:srgbClr val="000000"/></a:dk1></a:clrScheme></a:themeElements></a:theme>"#;
        let theme = crate::extraction::docx::theme::parse_theme_xml(&strict_to_transitional(strict)).unwrap();
        assert!(theme.color_scheme.is_some());
        let unconverted = crate::extraction::docx::theme::parse_theme_xml(strict).unwrap();
        assert!(
            unconverted.color_scheme.is_none(),
            "control: strict URI alone must not match"
        );
    }

    #[test]
    fn strict_conversion_edits_declarations_only() {
        let styles = format!(
            r#"<w:styles xmlns:w="http://purl.oclc.org/ooxml/wordprocessingml/ma&#105;n"><w:style w:type="paragraph" w:styleId="{STRICT_WML}"/></w:styles>"#
        );
        let converted = strict_to_transitional(&styles);
        assert!(converted.contains(&format!(r#"xmlns:w="{WML}""#)));
        assert!(
            converted.contains(&format!(r#"w:styleId="{STRICT_WML}""#)),
            "{converted}"
        );
        let catalog = crate::extraction::docx::styles::parse_styles_xml(&converted).unwrap();
        assert!(catalog.styles.contains_key(STRICT_WML));
        assert!(matches!(strict_to_transitional("<a/>"), Cow::Borrowed(_)));
    }

    #[test]
    fn repeated_xmlns_text_is_scanned_in_linear_time() {
        let hostile = format!(r#"<w:d xmlns:w="{WML}"><w:t>{}</w:t></w:d>"#, "xmlns:".repeat(200_000));
        let started = std::time::Instant::now();
        let _ = canonicalize_prefixes(&hostile);
        let _ = strict_to_transitional(&hostile);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn declaration_cut_by_the_window_inside_a_multibyte_name_is_unreadable_not_a_panic() {
        let xml = format!("<d xmlns:{}=\"u\"/>", "é".repeat(1000));
        assert!(read_declaration(&xml, "<d xmlns".len()).is_none());
    }
}
