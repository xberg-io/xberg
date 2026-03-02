//! DOCX drawing object parsing.
//!
//! This module handles extraction and parsing of drawing objects (`<w:drawing>`)
//! from DOCX documents. Drawing objects can be inline or anchored and may contain
//! images or shapes.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};

/// A drawing object extracted from `<w:drawing>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Drawing {
    pub drawing_type: DrawingType,
    pub extent: Option<Extent>,
    pub doc_properties: Option<DocProperties>,
    pub image_ref: Option<String>, // r:embed rId value
}

/// Whether the drawing is inline or anchored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DrawingType {
    Inline,
    Anchored(AnchorProperties),
}

/// Size in EMUs (English Metric Units, 1 inch = 914400 EMU).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Extent {
    pub cx: i64, // width in EMU
    pub cy: i64, // height in EMU
}

impl Extent {
    /// Convert width to inches.
    pub fn width_inches(&self) -> f64 {
        self.cx as f64 / super::EMUS_PER_INCH as f64
    }

    /// Convert height to inches.
    pub fn height_inches(&self) -> f64 {
        self.cy as f64 / super::EMUS_PER_INCH as f64
    }
}

/// Document properties from `<wp:docPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DocProperties {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>, // alt text
}

/// Properties for anchored drawings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AnchorProperties {
    pub behind_doc: bool,
    pub layout_in_cell: bool,
    pub relative_height: Option<i64>,
    pub position_h: Option<Position>,
    pub position_v: Option<Position>,
    pub wrap_type: WrapType,
}

/// Horizontal or vertical position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub relative_from: String, // "page", "margin", "column", "paragraph", "character"
    pub offset: Option<i64>,   // EMUs
}

/// Text wrapping type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum WrapType {
    #[default]
    None,
    Square,
    Tight,
    TopAndBottom,
    Through,
}

/// Parse a drawing object starting after the `<w:drawing>` Start event.
///
/// This function reads events until it encounters the closing `</w:drawing>` tag,
/// parsing the drawing type (inline or anchored), extent, properties, and image references.
pub fn parse_drawing(reader: &mut Reader<&[u8]>) -> Drawing {
    let mut drawing = Drawing {
        drawing_type: DrawingType::Inline,
        extent: None,
        doc_properties: None,
        image_ref: None,
    };

    let mut depth = 1; // We've already consumed the <w:drawing> start
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                let local_name = local.as_ref();

                match local_name {
                    b"inline" => {
                        drawing.drawing_type = DrawingType::Inline;
                        depth += 1;
                    }
                    b"anchor" => {
                        let anchor = AnchorProperties {
                            behind_doc: get_attr_bool(e, b"behindDoc"),
                            layout_in_cell: get_attr_bool(e, b"layoutInCell"),
                            relative_height: get_attr_i64(e, b"relativeHeight"),
                            ..Default::default()
                        };
                        drawing.drawing_type = DrawingType::Anchored(anchor);
                        depth += 1;
                    }
                    b"positionH" => {
                        let relative_from = get_attr(e, b"relativeFrom").unwrap_or_else(|| "page".to_string());
                        let position = parse_position(reader, "positionH");
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.position_h = Some(Position {
                                relative_from,
                                offset: position,
                            });
                        }
                        // parse_position consumes through the end tag, so no depth change
                    }
                    b"positionV" => {
                        let relative_from = get_attr(e, b"relativeFrom").unwrap_or_else(|| "paragraph".to_string());
                        let position = parse_position(reader, "positionV");
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.position_v = Some(Position {
                                relative_from,
                                offset: position,
                            });
                        }
                        // parse_position consumes through the end tag, so no depth change
                    }
                    b"wrapSquare" | b"wrapTight" | b"wrapTopAndBottom" | b"wrapThrough" => {
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            match local_name {
                                b"wrapSquare" => anchor.wrap_type = WrapType::Square,
                                b"wrapTight" => anchor.wrap_type = WrapType::Tight,
                                b"wrapTopAndBottom" => anchor.wrap_type = WrapType::TopAndBottom,
                                b"wrapThrough" => anchor.wrap_type = WrapType::Through,
                                _ => {}
                            }
                        }
                        depth += 1;
                    }
                    _ => {
                        depth += 1;
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                let local_name = local.as_ref();

                match local_name {
                    b"extent" => {
                        if let (Some(cx), Some(cy)) = (get_attr_i64(e, b"cx"), get_attr_i64(e, b"cy")) {
                            drawing.extent = Some(Extent { cx, cy });
                        }
                    }
                    b"docPr" => {
                        drawing.doc_properties = Some(DocProperties {
                            id: get_attr(e, b"id"),
                            name: get_attr(e, b"name"),
                            description: get_attr(e, b"descr"),
                        });
                    }
                    b"blip" => {
                        drawing.image_ref = get_attr(e, b"embed").or_else(|| get_attr(e, b"link"));
                    }
                    b"wrapNone" => {
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.wrap_type = WrapType::None;
                        }
                    }
                    b"wrapSquare" => {
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.wrap_type = WrapType::Square;
                        }
                    }
                    b"wrapTight" => {
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.wrap_type = WrapType::Tight;
                        }
                    }
                    b"wrapTopAndBottom" => {
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.wrap_type = WrapType::TopAndBottom;
                        }
                    }
                    b"wrapThrough" => {
                        if let DrawingType::Anchored(ref mut anchor) = drawing.drawing_type {
                            anchor.wrap_type = WrapType::Through;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                depth -= 1;
                if e.local_name().as_ref() == b"drawing" && depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => {
                break;
            }
            Err(_) => {
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    drawing
}

/// Parse position offset from positionH or positionV element.
/// Consumes all events through the closing element_name end tag.
fn parse_position(reader: &mut Reader<&[u8]>, element_name: &str) -> Option<i64> {
    let mut buf = Vec::new();
    let element_bytes = element_name.as_bytes();
    let mut result = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"posOffset" {
                    let mut text_buf = Vec::new();
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut text_buf)
                        && let Ok(text) = t.decode()
                    {
                        result = text.parse::<i64>().ok();
                    }
                    // Consume the posOffset end tag
                    let mut end_buf = Vec::new();
                    let _ = reader.read_event_into(&mut end_buf);
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == element_bytes {
                    return result;
                }
            }
            Ok(Event::Eof) => {
                return result;
            }
            Err(_) => {
                return result;
            }
            _ => {}
        }
        buf.clear();
    }
}

/// Extract a string attribute by local name.
fn get_attr(e: &BytesStart, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|attr| attr.key.local_name().as_ref() == key)
        .and_then(|attr| {
            let raw = std::str::from_utf8(&attr.value).ok()?;
            quick_xml::escape::unescape(raw).ok().map(|s| s.into_owned())
        })
}

/// Extract an i64 attribute by local name.
fn get_attr_i64(e: &BytesStart, key: &[u8]) -> Option<i64> {
    get_attr(e, key).and_then(|s| s.parse().ok())
}

/// Extract a boolean attribute by local name (value "1" = true).
fn get_attr_bool(e: &BytesStart, key: &[u8]) -> bool {
    get_attr(e, key).as_deref() == Some("1")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to parse drawing XML and return the Drawing object.
    fn parse_drawing_from_xml(xml: &[u8]) -> Drawing {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();

        // Consume opening tag and any events before <w:drawing>
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"drawing" => {
                    break;
                }
                Ok(Event::Eof) => {
                    return Drawing {
                        drawing_type: DrawingType::Inline,
                        extent: None,
                        doc_properties: None,
                        image_ref: None,
                    };
                }
                Err(_) => {
                    return Drawing {
                        drawing_type: DrawingType::Inline,
                        extent: None,
                        doc_properties: None,
                        image_ref: None,
                    };
                }
                _ => {}
            }
            buf.clear();
        }

        parse_drawing(&mut reader)
    }

    #[test]
    fn test_parse_inline_drawing() {
        let xml = br#"<w:drawing xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                        xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                        xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <wp:inline>
            <wp:extent cx="914400" cy="457200"/>
            <wp:docPr id="1" name="Picture 1" descr="A test image"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic>
                  <pic:blipFill>
                    <a:blip r:embed="rId5"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>"#;

        let drawing = parse_drawing_from_xml(xml);

        assert_eq!(drawing.drawing_type, DrawingType::Inline);
        assert_eq!(drawing.extent, Some(Extent { cx: 914400, cy: 457200 }));
        assert_eq!(
            drawing.doc_properties,
            Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Picture 1".to_string()),
                description: Some("A test image".to_string()),
            })
        );
        assert_eq!(drawing.image_ref, Some("rId5".to_string()));
    }

    #[test]
    fn test_parse_anchored_drawing() {
        let xml = br#"<w:drawing xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                        xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                        xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <wp:anchor behindDoc="0" layoutInCell="1" relativeHeight="251573248">
            <wp:positionH relativeFrom="page">
              <wp:posOffset>621792</wp:posOffset>
            </wp:positionH>
            <wp:positionV relativeFrom="paragraph">
              <wp:posOffset>274320</wp:posOffset>
            </wp:positionV>
            <wp:extent cx="209550" cy="209550"/>
            <wp:wrapSquare/>
            <wp:docPr id="2" name="Picture 2"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic>
                  <pic:blipFill>
                    <a:blip r:embed="rId6"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:anchor>
        </w:drawing>"#;

        let drawing = parse_drawing_from_xml(xml);

        match drawing.drawing_type {
            DrawingType::Anchored(anchor) => {
                assert!(!anchor.behind_doc);
                assert!(anchor.layout_in_cell);
                assert_eq!(anchor.relative_height, Some(251573248));
                assert_eq!(anchor.wrap_type, WrapType::Square);

                assert!(anchor.position_h.is_some());
                if let Some(pos_h) = anchor.position_h {
                    assert_eq!(pos_h.relative_from, "page");
                    assert_eq!(pos_h.offset, Some(621792));
                }

                assert!(anchor.position_v.is_some());
                if let Some(pos_v) = anchor.position_v {
                    assert_eq!(pos_v.relative_from, "paragraph");
                    assert_eq!(pos_v.offset, Some(274320));
                }
            }
            _ => panic!("Expected DrawingType::Anchored"),
        }

        assert_eq!(drawing.extent, Some(Extent { cx: 209550, cy: 209550 }));
        assert_eq!(drawing.image_ref, Some("rId6".to_string()));
    }

    #[test]
    fn test_parse_drawing_wrap_none() {
        let xml = br#"<w:drawing xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                        xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                        xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <wp:anchor behindDoc="0" layoutInCell="0" relativeHeight="0">
            <wp:wrapNone/>
            <wp:extent cx="100000" cy="100000"/>
            <wp:docPr id="3" name="Picture 3"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic>
                  <pic:blipFill>
                    <a:blip r:embed="rId7"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:anchor>
        </w:drawing>"#;

        let drawing = parse_drawing_from_xml(xml);

        match drawing.drawing_type {
            DrawingType::Anchored(anchor) => {
                assert_eq!(anchor.wrap_type, WrapType::None);
            }
            _ => panic!("Expected DrawingType::Anchored"),
        }
    }

    #[test]
    fn test_parse_drawing_wrap_top_and_bottom() {
        let xml = br#"<w:drawing xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                        xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                        xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <wp:anchor behindDoc="0" layoutInCell="0" relativeHeight="0">
            <wp:wrapTopAndBottom/>
            <wp:extent cx="100000" cy="100000"/>
            <wp:docPr id="4" name="Picture 4"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic>
                  <pic:blipFill>
                    <a:blip r:embed="rId8"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:anchor>
        </w:drawing>"#;

        let drawing = parse_drawing_from_xml(xml);

        match drawing.drawing_type {
            DrawingType::Anchored(anchor) => {
                assert_eq!(anchor.wrap_type, WrapType::TopAndBottom);
            }
            _ => panic!("Expected DrawingType::Anchored"),
        }
    }

    #[test]
    fn test_extent_conversion() {
        let extent = Extent { cx: 914400, cy: 914400 };

        assert_eq!(extent.width_inches(), 1.0);
        assert_eq!(extent.height_inches(), 1.0);

        let extent2 = Extent {
            cx: 1828800,
            cy: 914400,
        };

        assert_eq!(extent2.width_inches(), 2.0);
        assert_eq!(extent2.height_inches(), 1.0);
    }

    #[test]
    fn test_parse_drawing_no_image() {
        let xml = br#"<w:drawing xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                        xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
          <wp:inline>
            <wp:extent cx="100000" cy="100000"/>
            <wp:docPr id="5" name="Shape 5"/>
            <a:graphic>
              <a:graphicData>
                <!-- No blip element, just a shape -->
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>"#;

        let drawing = parse_drawing_from_xml(xml);

        assert_eq!(drawing.drawing_type, DrawingType::Inline);
        assert_eq!(drawing.extent, Some(Extent { cx: 100000, cy: 100000 }));
        assert_eq!(drawing.image_ref, None);
    }

    #[test]
    fn test_parse_drawing_empty_extent() {
        let xml = br#"<w:drawing xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                        xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                        xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <wp:inline>
            <wp:docPr id="6" name="Picture 6"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic>
                  <pic:blipFill>
                    <a:blip r:embed="rId9"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>"#;

        let drawing = parse_drawing_from_xml(xml);

        assert_eq!(drawing.drawing_type, DrawingType::Inline);
        assert_eq!(drawing.extent, None);
        assert_eq!(drawing.image_ref, Some("rId9".to_string()));
    }

    #[test]
    fn test_drawing_serialization() {
        let drawing = Drawing {
            drawing_type: DrawingType::Inline,
            extent: Some(Extent { cx: 914400, cy: 457200 }),
            doc_properties: Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Test".to_string()),
                description: Some("Test description".to_string()),
            }),
            image_ref: Some("rId5".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&drawing).expect("Failed to serialize");

        // Deserialize back
        let deserialized: Drawing = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify round-trip
        assert_eq!(drawing, deserialized);
    }
}
