use super::*;

pub(crate) fn create_test_pptx_bytes(slides: Vec<&str>) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

        let mut rels_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        );
        for (i, _) in slides.iter().enumerate() {
            use std::fmt::Write;
            let _ = write!(
                rels_xml,
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                i + 1,
                i + 1
            );
        }
        rels_xml.push_str("</Relationships>");
        zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
        zip.write_all(rels_xml.as_bytes()).unwrap();

        for (i, text) in slides.iter().enumerate() {
            let slide_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>{}</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#,
                text
            );
            zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), options)
                .unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();
        }

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/">
    <dc:title>Test Presentation</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:description>Test Description</dc:description>
    <dc:subject>Test Subject</dc:subject>
</cp:coreProperties>"#,
        )
        .unwrap();

        let app_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
    <Slides>{}</Slides>
    <Application>Microsoft Office PowerPoint</Application>
</Properties>"#,
            slides.len()
        );
        zip.start_file("docProps/app.xml", options).unwrap();
        zip.write_all(app_xml.as_bytes()).unwrap();

        let _ = zip.finish().unwrap();
    }
    buffer
}

#[test]
fn test_extract_pptx_from_bytes_single_slide() {
    let pptx_bytes = create_test_pptx_bytes(vec!["Hello World"]);
    let result = extract_pptx_from_bytes(
        &pptx_bytes,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert_eq!(result.slide_count, 1);
    assert!(
        result.content.contains("Hello World"),
        "Content was: {}",
        result.content
    );
    assert_eq!(result.image_count, 0);
    assert_eq!(result.table_count, 0);
}

#[test]
fn test_extract_pptx_from_bytes_multiple_slides() {
    let pptx_bytes = create_test_pptx_bytes(vec!["Slide 1", "Slide 2", "Slide 3"]);
    let internal = extract_pptx_from_bytes_with_slide_contents(
        &pptx_bytes,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    let result = internal.result;

    assert_eq!(result.slide_count, 3);
    assert!(result.content.contains("Slide 1"));
    assert!(result.content.contains("Slide 2"));
    assert!(result.content.contains("Slide 3"));
    assert!(!result.content.contains("<!-- Slide number:"));
    assert_eq!(
        internal
            .slide_contents
            .iter()
            .map(|slide| slide.slide_number)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn test_extract_pptx_metadata() {
    let pptx_bytes = create_test_pptx_bytes(vec!["Content"]);
    let result = extract_pptx_from_bytes(
        &pptx_bytes,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert_eq!(result.metadata.slide_count, 1);
}

#[test]
fn test_extract_pptx_empty_slides() {
    let pptx_bytes = create_test_pptx_bytes(vec!["", "", ""]);
    let result = extract_pptx_from_bytes(
        &pptx_bytes,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert_eq!(result.slide_count, 3);
}

#[test]
fn test_extract_pptx_from_bytes_invalid_data() {
    use crate::error::XbergError;

    let invalid_bytes = b"not a valid pptx file";
    let result = extract_pptx_from_bytes(
        invalid_bytes,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    );

    assert!(result.is_err());
    if let Err(XbergError::Parsing { message: msg, .. }) = result {
        assert!(msg.contains("Failed to read PPTX archive") || msg.contains("Failed to write temp PPTX file"));
    } else {
        panic!("Expected ParsingError");
    }
}

#[test]
fn test_extract_pptx_from_bytes_empty_data() {
    let empty_bytes: &[u8] = &[];
    let result = extract_pptx_from_bytes(
        empty_bytes,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    );

    assert!(result.is_err());
}

/// Build a PPTX bytes with sections and optional speaker notes for integration testing.
pub(crate) fn create_pptx_with_sections_and_notes(
    slides: &[(&str, Option<&str>)],
    sections: &[(&str, &[usize])],
) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

    let mut pres_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (i, _) in slides.iter().enumerate() {
        use std::fmt::Write as FmtWrite;
        let _ = write!(
            pres_rels,
            r#"<Relationship Id="rId{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{id}.xml"/>"#,
            id = i + 1
        );
    }
    pres_rels.push_str("</Relationships>");
    zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
    zip.write_all(pres_rels.as_bytes()).unwrap();

    let base_id: u32 = 256;
    let mut sld_id_lst = String::new();
    for (i, _) in slides.iter().enumerate() {
        use std::fmt::Write as FmtWrite;
        let _ = write!(
            sld_id_lst,
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            base_id + i as u32,
            i + 1
        );
    }

    let mut section_lst = String::new();
    for (name, positions) in sections {
        use std::fmt::Write as FmtWrite;
        let mut sld_ids = String::new();
        for &pos in *positions {
            let _ = write!(sld_ids, r#"<p14:sectionSldId id="{}"/>"#, base_id + (pos as u32 - 1));
        }
        let _ = write!(
            section_lst,
            r#"<p14:section name="{}"><p14:sectionSldIdLst>{}</p14:sectionSldIdLst></p14:section>"#,
            name, sld_ids
        );
    }

    let ext_lst = if section_lst.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p:extLst><p:ext uri="{{521415D9-36F7-43E2-AB2F-B90AF26B5E84}}"><p14:sectionLst xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main">{}</p14:sectionLst></p:ext></p:extLst>"#,
            section_lst
        )
    };

    let presentation_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>{}</p:sldIdLst>
  {}
</p:presentation>"#,
        sld_id_lst, ext_lst
    );
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(presentation_xml.as_bytes()).unwrap();

    for (i, (text, notes)) in slides.iter().enumerate() {
        let slide_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#,
            text
        );
        zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), opts).unwrap();
        zip.write_all(slide_xml.as_bytes()).unwrap();

        if let Some(note_text) = notes {
            let rels_xml = format!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="notes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide"
 Target="../notesSlides/notesSlide{}.xml"/></Relationships>"#,
                i + 1
            );
            zip.start_file(format!("ppt/slides/_rels/slide{}.xml.rels", i + 1), opts)
                .unwrap();
            zip.write_all(rels_xml.as_bytes()).unwrap();
            let notes_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:notes>"#,
                note_text
            );
            zip.start_file(format!("ppt/notesSlides/notesSlide{}.xml", i + 1), opts)
                .unwrap();
            zip.write_all(notes_xml.as_bytes()).unwrap();
        }
    }

    zip.start_file("docProps/core.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Test</dc:title></cp:coreProperties>"#,
    )
    .unwrap();

    let app_xml = format!(
        r#"<?xml version="1.0"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Slides>{}</Slides></Properties>"#,
        slides.len()
    );
    zip.start_file("docProps/app.xml", opts).unwrap();
    zip.write_all(app_xml.as_bytes()).unwrap();

    let _ = zip.finish().unwrap();
    buffer
}

#[test]
fn test_speaker_notes_in_page_contents() {
    use crate::core::config::PageConfig;

    let pptx = create_pptx_with_sections_and_notes(
        &[
            ("Intro Slide", Some("These are intro notes.")),
            ("Content Slide", None),
            ("Outro Slide", Some("Final notes here.")),
        ],
        &[],
    );

    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            page_config: Some(PageConfig::default()),
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let pages = result
        .page_contents
        .as_ref()
        .expect("page_contents should be populated");
    assert_eq!(pages.len(), 3);

    assert_eq!(pages[0].speaker_notes.as_deref(), Some("These are intro notes."));
    assert!(pages[1].speaker_notes.is_none(), "slide 2 has no notes");
    assert_eq!(pages[2].speaker_notes.as_deref(), Some("Final notes here."));
}

#[test]
fn test_section_name_in_page_contents() {
    use crate::core::config::PageConfig;

    let pptx = create_pptx_with_sections_and_notes(
        &[("Slide 1", None), ("Slide 2", None), ("Slide 3", None)],
        &[("Introduction", &[1]), ("Deep Dive", &[2, 3])],
    );

    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            page_config: Some(PageConfig::default()),
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let pages = result
        .page_contents
        .as_ref()
        .expect("page_contents should be populated");
    assert_eq!(pages.len(), 3);

    assert_eq!(pages[0].section_name.as_deref(), Some("Introduction"));
    assert_eq!(pages[1].section_name.as_deref(), Some("Deep Dive"));
    assert_eq!(pages[2].section_name.as_deref(), Some("Deep Dive"));
}

#[test]
fn test_section_name_and_notes_combined() {
    use crate::core::config::PageConfig;

    let pptx = create_pptx_with_sections_and_notes(
        &[
            ("Title Slide", Some("Welcome notes.")),
            ("Methods", Some("Methodology notes.")),
        ],
        &[("Part One", &[1, 2])],
    );

    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            page_config: Some(PageConfig::default()),
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let pages = result
        .page_contents
        .as_ref()
        .expect("page_contents should be populated");
    assert_eq!(pages[0].section_name.as_deref(), Some("Part One"));
    assert_eq!(pages[0].speaker_notes.as_deref(), Some("Welcome notes."));
    assert_eq!(pages[1].section_name.as_deref(), Some("Part One"));
    assert_eq!(pages[1].speaker_notes.as_deref(), Some("Methodology notes."));
}

#[test]
fn test_no_page_contents_without_page_config() {
    let pptx = create_pptx_with_sections_and_notes(&[("Slide 1", Some("Notes."))], &[("Section A", &[1])]);

    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert!(
        result.page_contents.is_none(),
        "page_contents should be None when page_config is not set"
    );
}

#[test]
fn test_detect_image_format_jpeg() {
    let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0];
    assert_eq!(detect_image_format(&jpeg_header), "jpeg");
}

#[test]
fn test_detect_image_format_png() {
    let png_header = vec![0x89, 0x50, 0x4E, 0x47];
    assert_eq!(detect_image_format(&png_header), "png");
}

#[test]
fn test_detect_image_format_gif() {
    let gif_header = b"GIF89a";
    assert_eq!(detect_image_format(gif_header), "gif");
}

#[test]
fn test_detect_image_format_bmp() {
    let bmp_header = b"BM";
    assert_eq!(detect_image_format(bmp_header), "bmp");
}

#[test]
fn test_detect_image_format_svg() {
    let svg_header = b"<svg xmlns=\"http://www.w3.org/2000/svg\">";
    assert_eq!(detect_image_format(svg_header), "svg");
}

#[test]
fn test_detect_image_format_tiff_little_endian() {
    let tiff_header = vec![0x49, 0x49, 0x2A, 0x00];
    assert_eq!(detect_image_format(&tiff_header), "tiff");
}

#[test]
fn test_detect_image_format_tiff_big_endian() {
    let tiff_header = vec![0x4D, 0x4D, 0x00, 0x2A];
    assert_eq!(detect_image_format(&tiff_header), "tiff");
}

#[test]
fn test_detect_image_format_unknown() {
    let unknown_data = b"unknown format";
    assert_eq!(detect_image_format(unknown_data), "unknown");
}

#[test]
fn test_get_slide_rels_path() {
    assert_eq!(
        image_handling::get_slide_rels_path("ppt/slides/slide1.xml"),
        "ppt/slides/_rels/slide1.xml.rels"
    );
    assert_eq!(
        image_handling::get_slide_rels_path("ppt/slides/slide10.xml"),
        "ppt/slides/_rels/slide10.xml.rels"
    );
}

#[test]
fn test_get_full_image_path_relative() {
    assert_eq!(
        image_handling::get_full_image_path("ppt/slides/slide1.xml", "../media/image1.png"),
        "ppt/media/image1.png"
    );
}

#[test]
fn test_get_full_image_path_direct() {
    assert_eq!(
        image_handling::get_full_image_path("ppt/slides/slide1.xml", "image1.png"),
        "ppt/slides/image1.png"
    );
}

/// Build a minimal PPTX ZIP with one slide that contains an image (<p:pic>) element.
///
/// The slide XML includes a picture shape referencing rel id "rId2", and the
/// slide rels file maps that id to "media/image1.png".
fn create_pptx_with_image_slide(slide_text: &str) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let opts = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="png" ContentType="image/png"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

        zip.start_file("ppt/presentation.xml", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#).unwrap();

        let slide_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p><a:r><a:t>{slide_text}</a:t></a:r></a:p>
                </p:txBody>
            </p:sp>
            <p:pic>
                <p:nvPicPr>
                    <p:nvPr><p:ph type="pic"/></p:nvPr>
                    <p:cNvPicPr/>
                    <p:nvPr><a:hlinkClick r:id=""/></p:nvPr>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId2" descr="Test chart"/>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
                </p:spPr>
            </p:pic>
        </p:spTree>
    </p:cSld>
</p:sld>"#
        );
        zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
        zip.write_all(slide_xml.as_bytes()).unwrap();

        zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#).unwrap();

        zip.start_file("ppt/media/image1.png", opts).unwrap();
        zip.write_all(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82").unwrap();

        zip.start_file("docProps/core.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Image Test</dc:title>
</cp:coreProperties>"#,
        )
        .unwrap();

        zip.start_file("docProps/app.xml", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Slides>1</Slides></Properties>").unwrap();

        let _ = zip.finish().unwrap();
    }
    buffer
}

#[test]
fn test_inject_placeholders_true_emits_image_reference() {
    let pptx = create_pptx_with_image_slide("Hello");
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        result.content.contains("!["),
        "inject_placeholders=true must emit image reference, got: {:?}",
        result.content
    );
}

#[test]
fn test_inject_placeholders_false_suppresses_image_reference() {
    let pptx = create_pptx_with_image_slide("Hello");
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            inject_placeholders: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        !result.content.contains("!["),
        "inject_placeholders=false must NOT emit image reference, got: {:?}",
        result.content
    );
}

#[test]
fn test_inject_placeholders_false_preserves_text_content() {
    let pptx = create_pptx_with_image_slide("Quarterly Review");
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            inject_placeholders: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        result.content.contains("Quarterly Review"),
        "Text content must survive when inject_placeholders=false, got: {:?}",
        result.content
    );
}

#[test]
fn test_default_inject_placeholders_preserves_existing_behaviour() {
    let pptx = create_pptx_with_image_slide("Slide Title");
    let result_true = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    let result_false = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            inject_placeholders: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        result_true.content.len() >= result_false.content.len(),
        "inject_placeholders=true should produce >= content length vs false"
    );
}

/// Build a minimal PPTX with optional comment XML parts.
///
/// `slides` is a list of slide text strings.
/// `comments_per_slide` is indexed by slide (0-based); each entry is a list
/// of `(idx, author_id, datetime, comment_text)` tuples.
/// `authors` is a list of `(id, name)` tuples written to `commentAuthors.xml`.
fn create_pptx_with_comments(
    slides: &[&str],
    comments_per_slide: &[Vec<(u32, u32, &str, &str)>],
    authors: &[(u32, &str)],
) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels"
      ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1"
      Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
      Target="ppt/presentation.xml"/>
</Relationships>"#,
    )
    .unwrap();

    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (i, _) in slides.iter().enumerate() {
        use std::fmt::Write as FmtWrite;
        let _ = write!(
            rels,
            r#"<Relationship Id="rId{id}"
  Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide"
  Target="slides/slide{id}.xml"/>"#,
            id = i + 1
        );
    }
    rels.push_str("</Relationships>");
    zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
    zip.write_all(rels.as_bytes()).unwrap();

    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0"?><p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst/></p:presentation>"#).unwrap();

    for (i, text) in slides.iter().enumerate() {
        let slide_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody>
        <a:p><a:r><a:t>{text}</a:t></a:r></a:p>
    </p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#
        );
        zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), opts).unwrap();
        zip.write_all(slide_xml.as_bytes()).unwrap();
    }

    if !authors.is_empty() {
        let mut authors_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:cmAuthorLst xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">"#,
        );
        for (id, name) in authors {
            use std::fmt::Write as FmtWrite;
            let _ = write!(
                authors_xml,
                r#"<p:cmAuthor id="{id}" name="{name}" initials="A" lastIdx="0" clrIdx="0"/>"#
            );
        }
        authors_xml.push_str("</p:cmAuthorLst>");
        zip.start_file("ppt/commentAuthors.xml", opts).unwrap();
        zip.write_all(authors_xml.as_bytes()).unwrap();
    }

    for (slide_idx, slide_comments) in comments_per_slide.iter().enumerate() {
        if slide_comments.is_empty() {
            continue;
        }
        let mut cm_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:cmLst xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">"#,
        );
        for (idx, author_id, dt, text) in slide_comments {
            use std::fmt::Write as FmtWrite;
            let _ = write!(
                cm_xml,
                r#"<p:cm authorId="{author_id}" dt="{dt}" idx="{idx}"><p:text>{text}</p:text></p:cm>"#
            );
        }
        cm_xml.push_str("</p:cmLst>");
        zip.start_file(format!("ppt/comments/comment{}.xml", slide_idx + 1), opts)
            .unwrap();
        zip.write_all(cm_xml.as_bytes()).unwrap();
    }

    zip.start_file("docProps/core.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Test</dc:title></cp:coreProperties>"#,
    )
    .unwrap();
    zip.start_file("docProps/app.xml", opts).unwrap();
    let app = format!(
        r#"<?xml version="1.0"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Slides>{}</Slides></Properties>"#,
        slides.len()
    );
    zip.write_all(app.as_bytes()).unwrap();

    let _ = zip.finish().unwrap();
    buffer
}

#[test]
fn should_return_none_revisions_when_no_comment_files_exist() {
    let pptx = create_test_pptx_bytes(vec!["Slide with no comments"]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        result.revisions.is_none(),
        "revisions should be None when no ppt/comments/ files exist"
    );
}

#[test]
fn should_surface_single_comment_as_revision_with_correct_fields() {
    let pptx = create_pptx_with_comments(
        &["Slide One"],
        &[vec![(1, 0, "2024-03-15T10:30:00Z", "Please revise this slide")]],
        &[(0, "Alice")],
    );
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let revisions = result
        .revisions
        .as_ref()
        .expect("revisions should be Some when comment files exist");
    assert_eq!(revisions.len(), 1, "expected 1 revision for 1 comment");

    let rev = &revisions[0];
    assert_eq!(rev.revision_id, "1");
    assert_eq!(rev.author.as_deref(), Some("Alice"));
    assert_eq!(rev.timestamp.as_deref(), Some("2024-03-15T10:30:00Z"));
    use crate::types::revisions::{DiffLine, RevisionAnchor, RevisionKind};
    assert!(matches!(rev.kind, RevisionKind::Comment));
    assert!(
        matches!(&rev.anchor, Some(RevisionAnchor::Slide { index: 0 })),
        "slide anchor should be 0 (0-indexed) for the first slide"
    );
    assert_eq!(rev.delta.content.len(), 1);
    assert!(matches!(&rev.delta.content[0], DiffLine::Context(t) if t == "Please revise this slide"));
}

#[test]
fn should_surface_comments_from_multiple_slides_with_correct_anchors() {
    let pptx = create_pptx_with_comments(
        &["Slide 1", "Slide 2", "Slide 3"],
        &[
            vec![(1, 0, "2024-03-15T09:00:00Z", "Comment on slide 1")],
            vec![],
            vec![
                (1, 0, "2024-03-15T11:00:00Z", "Comment A on slide 3"),
                (2, 1, "2024-03-15T11:05:00Z", "Comment B on slide 3"),
            ],
        ],
        &[(0, "Alice"), (1, "Bob")],
    );
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let revisions = result.revisions.as_ref().expect("revisions should be Some");
    assert_eq!(revisions.len(), 3);

    use crate::types::revisions::RevisionAnchor;
    assert!(matches!(&revisions[0].anchor, Some(RevisionAnchor::Slide { index: 0 })));
    assert!(matches!(&revisions[1].anchor, Some(RevisionAnchor::Slide { index: 2 })));
    assert!(matches!(&revisions[2].anchor, Some(RevisionAnchor::Slide { index: 2 })));
    assert_eq!(revisions[1].author.as_deref(), Some("Alice"));
    assert_eq!(revisions[2].author.as_deref(), Some("Bob"));
}
