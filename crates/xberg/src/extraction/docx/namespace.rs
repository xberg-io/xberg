//! Namespace-prefix canonicalization for WordprocessingML parts.
//!
//! The streaming parsers match literal qualified names (`w:p`, `w:val`, `r:id`, `m:oMath`,
//! `mc:Fallback`). XML lets a producer bind any prefix to a namespace, or none at all, so a part
//! declaring `xmlns:x="…/wordprocessingml/2006/main"` would match nothing and its text would be
//! dropped. [`canonicalize_prefixes`] rewrites element and attribute names whose namespace is one
//! of the known ones to the conventional prefix, once per part, before any parser sees it.
//!
//! Parts that already use the conventional prefixes (the overwhelming majority) take a
//! zero-allocation fast path and are returned borrowed.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::borrow::Cow;

/// Known namespace URIs and the prefix the parsers expect for each. Both the transitional and the
/// strict (ISO 29500 `purl.oclc.org`) URIs map to the same prefix.
const KNOWN_NAMESPACES: [(&str, &str); 7] = [
    ("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "w"),
    ("http://purl.oclc.org/ooxml/wordprocessingml/main", "w"),
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

/// Rewrite namespace prefixes in `xml` to the conventional `w`/`r`/`m`/`mc` set.
///
/// Returns the input borrowed when it needs no rewriting. Malformed XML is returned unchanged so
/// the downstream parser reports or tolerates it exactly as before.
pub(crate) fn canonicalize_prefixes(xml: &str) -> Cow<'_, str> {
    if !needs_rewrite(xml) {
        return Cow::Borrowed(xml);
    }
    rewrite(xml).map_or(Cow::Borrowed(xml), Cow::Owned)
}

/// Fast check: only the root element's declarations are inspected, plus a count of `xmlns`
/// occurrences to prove nothing is redeclared further down.
fn needs_rewrite(xml: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let root_declarations = loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let mut declarations = 0usize;
                for attr in e.attributes().flatten() {
                    let Some(prefix) = declared_prefix(attr.key.as_ref()) else {
                        continue;
                    };
                    declarations += 1;
                    if !declaration_is_canonical(prefix, &attr.value) {
                        return true;
                    }
                }
                break declarations;
            }
            Ok(Event::Eof) | Err(_) => return false,
            _ => {}
        }
    };
    xml.matches("xmlns").count() != root_declarations
}

/// `Some(prefix)` for an `xmlns` / `xmlns:prefix` attribute key, `""` for the default namespace.
fn declared_prefix(key: &str) -> Option<&str> {
    if key == "xmlns" {
        Some("")
    } else {
        key.strip_prefix("xmlns:")
    }
}

fn declaration_is_canonical(prefix: &str, uri: &str) -> bool {
    match canonical_prefix(uri) {
        Some(canonical) => prefix == canonical,
        None => !RESERVED_PREFIXES.contains(&prefix),
    }
}

struct Scope {
    declarations: Vec<(String, String)>,
    frames: Vec<usize>,
}

impl Scope {
    fn resolve(&self, prefix: &str) -> Option<&str> {
        self.declarations
            .iter()
            .rev()
            .find_map(|(p, uri)| (p == prefix).then_some(uri.as_str()))
    }

    /// The name to emit for `name`, or `None` when it is already canonical. Unprefixed attributes
    /// belong to no namespace and are never rewritten.
    fn canonical_name(&self, name: &str, is_attribute: bool) -> Option<String> {
        let (prefix, local) = match name.split_once(':') {
            Some((prefix, local)) => (prefix, local),
            None if is_attribute => return None,
            None => ("", name),
        };
        if prefix == "xmlns" || prefix == "xml" {
            return None;
        }
        let uri = self.resolve(prefix)?;
        let target = match canonical_prefix(uri) {
            Some(canonical) => canonical.to_string(),
            None if RESERVED_PREFIXES.contains(&prefix) => format!("xberg_ns_{prefix}"),
            None => return None,
        };
        (target != prefix).then(|| format!("{target}:{local}"))
    }
}

/// Registers the element's namespace declarations in `scope`, then returns the rewritten tag text,
/// `Ok(None)` when nothing in it needs renaming, or `None` when its attributes cannot be read.
fn rewrite_start_tag(scope: &mut Scope, e: &BytesStart, is_empty: bool) -> Option<Option<String>> {
    let mut attributes = Vec::new();
    for attr in e.attributes() {
        let attr = attr.ok()?;
        let key = attr.key.as_ref();
        if let Some(prefix) = declared_prefix(key) {
            scope.declarations.push((prefix.to_string(), attr.value.to_string()));
        }
        attributes.push((key.to_string(), attr.value.into_owned()));
    }
    let original_name = e.name().as_ref().to_string();
    let new_name = scope.canonical_name(&original_name, false);
    let mut changed = new_name.is_some();
    for (key, _) in &mut attributes {
        if let Some(renamed) = scope.canonical_name(key, true) {
            *key = renamed;
            changed = true;
        }
    }
    if !changed {
        return Some(None);
    }
    let mut tag = format!("<{}", new_name.as_deref().unwrap_or(&original_name));
    for (key, value) in &attributes {
        let quote = if value.contains('"') { '\'' } else { '"' };
        tag.push_str(&format!(" {key}={quote}{value}{quote}"));
    }
    tag.push_str(if is_empty { "/>" } else { ">" });
    Some(Some(tag))
}

fn rewrite(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    let mut scope = Scope {
        declarations: Vec::new(),
        frames: Vec::new(),
    };
    let mut out = String::with_capacity(xml.len() + xml.len() / 16);
    let mut copied = 0usize;

    loop {
        let before = usize::try_from(reader.buffer_position()).ok()?;
        let event = reader.read_event().ok()?;
        let after = usize::try_from(reader.buffer_position()).ok()?;
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let is_empty = xml.as_bytes().get(after.wrapping_sub(2)) == Some(&b'/');
                let frame_start = scope.declarations.len();
                if let Some(tag) = rewrite_start_tag(&mut scope, &e, is_empty)? {
                    out.push_str(&xml[copied..before]);
                    out.push_str(&tag);
                    copied = after;
                }
                if is_empty {
                    scope.declarations.truncate(frame_start);
                } else {
                    scope.frames.push(frame_start);
                }
            }
            Event::End(e) => {
                if let Some(new_name) = scope.canonical_name(e.name().as_ref(), false) {
                    out.push_str(&xml[copied..before]);
                    out.push_str("</");
                    out.push_str(&new_name);
                    out.push('>');
                    copied = after;
                }
                if let Some(frame_start) = scope.frames.pop() {
                    scope.declarations.truncate(frame_start);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    out.push_str(&xml[copied..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WML: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    #[test]
    fn conventional_prefixes_are_returned_borrowed() {
        let xml = format!(r#"<w:document xmlns:w="{WML}"><w:body><w:p/></w:body></w:document>"#);
        assert!(matches!(canonicalize_prefixes(&xml), Cow::Borrowed(_)));
    }

    #[test]
    fn alternate_prefix_is_rewritten_for_elements_and_attributes() {
        let xml = format!(r#"<x:document xmlns:x="{WML}"><x:b x:val='0'/><x:t>a &amp; b</x:t></x:document>"#);
        assert_eq!(
            canonicalize_prefixes(&xml),
            format!(r#"<w:document xmlns:x="{WML}"><w:b w:val="0"/><w:t>a &amp; b</w:t></w:document>"#)
        );
    }

    #[test]
    fn default_namespace_elements_gain_the_conventional_prefix() {
        let xml = format!(r#"<document xmlns="{WML}" xmlns:w="{WML}"><p><r w:val="1"/></p></document>"#);
        assert_eq!(
            canonicalize_prefixes(&xml),
            format!(r#"<w:document xmlns="{WML}" xmlns:w="{WML}"><w:p><w:r w:val="1"/></w:p></w:document>"#)
        );
    }

    #[test]
    fn strict_namespace_maps_to_the_conventional_prefix() {
        let xml = r#"<s:p xmlns:s="http://purl.oclc.org/ooxml/wordprocessingml/main"/>"#;
        assert_eq!(
            canonicalize_prefixes(xml),
            r#"<w:p xmlns:s="http://purl.oclc.org/ooxml/wordprocessingml/main"/>"#
        );
    }

    #[test]
    fn foreign_namespace_bound_to_a_reserved_prefix_is_renamed() {
        let xml = format!(r#"<x:p xmlns:x="{WML}" xmlns:w="urn:other"><w:t>no</w:t></x:p>"#);
        assert_eq!(
            canonicalize_prefixes(&xml),
            format!(r#"<w:p xmlns:x="{WML}" xmlns:w="urn:other"><xberg_ns_w:t>no</xberg_ns_w:t></w:p>"#)
        );
    }

    #[test]
    fn nested_redeclaration_is_resolved_in_scope() {
        let xml = format!(r#"<w:body xmlns:w="{WML}"><q:p xmlns:q="{WML}"><q:t/></q:p><w:p/></w:body>"#);
        assert_eq!(
            canonicalize_prefixes(&xml),
            format!(r#"<w:body xmlns:w="{WML}"><w:p xmlns:q="{WML}"><w:t/></w:p><w:p/></w:body>"#)
        );
    }

    #[test]
    fn malformed_xml_is_returned_unchanged() {
        let xml = format!(r#"<x:document xmlns:x="{WML}"><x:p></y:q>"#);
        assert_eq!(canonicalize_prefixes(&xml), xml);
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

    /// Turn a template written with `w:`/`r:` into `flavor`, declaring the namespaces on the root.
    fn render(template: &str, flavor: Flavor) -> String {
        let (element, attribute, root_decl) = match flavor {
            Flavor::Conventional => ("w:", "w:", format!(r#"xmlns:w="{WML}""#)),
            Flavor::Alias => ("x:", "x:", format!(r#"xmlns:x="{WML}""#)),
            Flavor::DefaultNamespace => ("", "w:", format!(r#"xmlns="{WML}" xmlns:w="{WML}""#)),
            Flavor::Strict => ("s:", "s:", format!(r#"xmlns:s="{STRICT_WML}""#)),
        };
        let relationship_prefix = if matches!(flavor, Flavor::Conventional) {
            "r"
        } else {
            "rel"
        };
        template
            .replace("@NS@", &format!(r#"{root_decl} xmlns:{relationship_prefix}="{REL}""#))
            .replace("<w:", &format!("<{element}"))
            .replace("</w:", &format!("</{element}"))
            .replace(" w:", &format!(" {attribute}"))
            .replace(" r:id", &format!(" {relationship_prefix}:id"))
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

    fn build_package(flavors: [Flavor; 5]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        let parts = [
            ("word/document.xml", DOCUMENT),
            ("word/numbering.xml", NUMBERING),
            ("word/footnotes.xml", FOOTNOTES),
            ("word/comments.xml", COMMENTS),
            ("word/header1.xml", HEADER),
        ];
        for ((path, template), flavor) in parts.into_iter().zip(flavors) {
            zip.start_file(path, options).unwrap();
            zip.write_all(render(template, flavor).as_bytes()).unwrap();
        }
        zip.start_file("word/_rels/document.xml.rels", options).unwrap();
        zip.write_all(RELS.as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }

    fn parse(flavors: [Flavor; 5]) -> String {
        use crate::extractors::security::{SecurityBudget, SecurityLimits};
        let mut budget = SecurityBudget::with_defaults();
        let document = crate::extraction::docx::parser::parse_document(
            &build_package(flavors),
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
            canonicalize_prefixes(&xml),
            format!(
                r#"<w:document xmlns:w="{WML}" xmlns:x="{WML}"><w:body><w:p><w:r><w:t>mixed</w:t></w:r></w:p></w:body></w:document>"#
            )
        );
    }
}
