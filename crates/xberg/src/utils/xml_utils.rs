#[cfg(any(feature = "xml", feature = "office"))]
use std::borrow::Cow;

/// Converts XML tag name bytes to a string, avoiding allocation when possible.
#[cfg(any(feature = "xml", feature = "office"))]
#[inline]
pub(crate) fn xml_tag_name(name: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(name)
}

/// Resolve a predefined XML entity name to its character.
#[cfg(any(feature = "xml", feature = "office"))]
fn resolve_entity(name: &str) -> Option<&'static str> {
    match name {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        "nbsp" => Some("\u{00A0}"),
        _ => None,
    }
}

/// Streaming XML reader that restores pre-quick-xml-0.37 text semantics.
///
/// quick-xml 0.37+ splits text nodes at entity and character references, emitting
/// `Event::GeneralRef` between `Event::Text` fragments. Consumers that only match
/// `Event::Text` silently drop the referenced characters (`&amp;` → nothing), and
/// trim-and-join accumulators corrupt spacing around the fragments. This wrapper
/// coalesces each run of `Text`/`GeneralRef` events into a single `Event::Text`
/// with references resolved, so consumers see whole text nodes again.
///
/// Reader-level `trim_text` must stay off (the default) — trimming individual
/// fragments before coalescing would destroy the whitespace around references.
#[cfg(any(feature = "xml", feature = "office"))]
pub(crate) struct EntityReader<'x> {
    reader: quick_xml::Reader<&'x [u8]>,
    pending: Option<quick_xml::events::Event<'x>>,
}

#[cfg(any(feature = "xml", feature = "office"))]
impl<'x> EntityReader<'x> {
    pub(crate) fn from_str(content: &'x str) -> Self {
        Self {
            reader: quick_xml::Reader::from_str(content),
            pending: None,
        }
    }

    pub(crate) fn from_bytes(content: &'x [u8]) -> Self {
        Self {
            reader: quick_xml::Reader::from_reader(content),
            pending: None,
        }
    }

    pub(crate) fn config_mut(&mut self) -> &mut quick_xml::reader::Config {
        self.reader.config_mut()
    }

    pub(crate) fn buffer_position(&self) -> u64 {
        self.reader.buffer_position()
    }

    /// Read the next event, merging consecutive `Text` / `GeneralRef` events into
    /// one owned `Event::Text` whose content has all references resolved.
    pub(crate) fn read_event(&mut self) -> quick_xml::Result<quick_xml::events::Event<'x>> {
        use quick_xml::events::{BytesText, Event};

        let first = match self.pending.take() {
            Some(event) => event,
            None => self.reader.read_event()?,
        };
        let mut text = match first {
            Event::Text(t) => String::from_utf8_lossy(t.as_ref()).into_owned(),
            Event::GeneralRef(r) => resolve_general_ref(r.as_ref()),
            other => return Ok(other),
        };
        loop {
            match self.reader.read_event()? {
                Event::Text(t) => text.push_str(&String::from_utf8_lossy(t.as_ref())),
                Event::GeneralRef(r) => text.push_str(&resolve_general_ref(r.as_ref())),
                other => {
                    self.pending = Some(other);
                    break;
                }
            }
        }
        Ok(Event::Text(BytesText::from_escaped(text)))
    }
}

/// Resolve an XML general reference (entity or character reference) to its text.
///
/// quick-xml 0.37+ emits `&amp;`-style references as `Event::GeneralRef` instead of
/// including them in `Event::Text`, so every streaming reader that assembles text
/// must append the resolved reference or the characters are silently dropped.
/// Unknown named entities (undefined DTD entities) resolve to an empty string.
#[cfg(any(feature = "xml", feature = "office"))]
pub(crate) fn resolve_general_ref(ref_bytes: &[u8]) -> String {
    let name = String::from_utf8_lossy(ref_bytes);
    if let Some(entity) = resolve_entity(&name) {
        return entity.to_string();
    }
    if let Some(num) = name.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix('x') {
            u32::from_str_radix(hex, 16).ok()
        } else {
            num.parse::<u32>().ok()
        };
        if let Some(ch) = code.and_then(char::from_u32) {
            return ch.to_string();
        }
    }
    String::new()
}

#[cfg(all(test, any(feature = "xml", feature = "office")))]
mod tests {
    use super::*;
    use quick_xml::events::Event;

    #[test]
    fn test_resolve_general_ref_predefined_entities() {
        assert_eq!(resolve_general_ref(b"amp"), "&");
        assert_eq!(resolve_general_ref(b"lt"), "<");
        assert_eq!(resolve_general_ref(b"gt"), ">");
        assert_eq!(resolve_general_ref(b"quot"), "\"");
        assert_eq!(resolve_general_ref(b"apos"), "'");
        assert_eq!(resolve_general_ref(b"nbsp"), "\u{00A0}");
    }

    #[test]
    fn test_resolve_general_ref_character_references() {
        assert_eq!(resolve_general_ref(b"#8212"), "\u{2014}");
        assert_eq!(resolve_general_ref(b"#x2014"), "\u{2014}");
        assert_eq!(resolve_general_ref(b"#65"), "A");
    }

    #[test]
    fn test_resolve_general_ref_unknown_entity_is_empty() {
        assert_eq!(resolve_general_ref(b"unknownentity"), "");
        assert_eq!(resolve_general_ref(b"#xZZ"), "");
        assert_eq!(resolve_general_ref(b"#1114112"), ""); // beyond char::MAX
    }

    /// The core contract: a text node split at references arrives as ONE Text
    /// event with the references resolved in place, spacing intact.
    #[test]
    fn test_entity_reader_coalesces_text_and_references() {
        let xml = "<root><a>Profits &amp; losses</a><b>5&gt;3</b><c/></root>";
        let mut reader = EntityReader::from_str(xml);
        let mut texts = Vec::new();
        loop {
            match reader.read_event().expect("valid XML") {
                Event::Text(t) => texts.push(String::from_utf8_lossy(t.as_ref()).into_owned()),
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(texts, vec!["Profits & losses", "5>3"]);
    }

    #[test]
    fn test_entity_reader_preserves_non_text_events() {
        let xml = "<root attr=\"v\">x&#65;y<child/></root>";
        let mut reader = EntityReader::from_str(xml);
        let mut summary = Vec::new();
        loop {
            match reader.read_event().expect("valid XML") {
                Event::Start(e) => summary.push(format!("start:{}", String::from_utf8_lossy(e.name().as_ref()))),
                Event::Empty(e) => summary.push(format!("empty:{}", String::from_utf8_lossy(e.name().as_ref()))),
                Event::End(e) => summary.push(format!("end:{}", String::from_utf8_lossy(e.name().as_ref()))),
                Event::Text(t) => summary.push(format!("text:{}", String::from_utf8_lossy(t.as_ref()))),
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(summary, vec!["start:root", "text:xAy", "empty:child", "end:root"]);
    }
}
