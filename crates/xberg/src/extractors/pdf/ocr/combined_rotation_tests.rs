mod tests {
    use std::sync::Arc;

    use image::{DynamicImage, Rgb, RgbImage};

    use super::super::document::{build_mixed_ocr_page_document, formula_bbox_to_page_points};
    use super::super::pipeline::{undo_upright_raster_correction, upright_raster_for_backend};
    use super::super::rendering::{open_pdf_for_full_ocr, page_dimensions_pt};
    use crate::ocr_metadata_keys::{
        OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY, OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
    };
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};
    use crate::types::{BoundingBox, ExtractedDocument, Formula, Table};

    #[derive(Clone, Copy)]
    struct Case {
        correction: u16,
        scale: u32,
        dimensions: [u32; 2],
        word: [u32; 4],
        table: [u32; 4],
        formula: [u32; 4],
        quad: [[u32; 2]; 4],
    }

    // Raw 400x200 raster: word(40,20)-(100,50), table(160,80)-(300,140), formula(280,30)-(360,60).
    // Page90 plus backend90/180/270 gives180/270/0 respectively; these are independently fixed edges. ~keep
    const CASES: [Case; 3] = [
        Case {
            correction: 90,
            scale: 1,
            dimensions: [400, 200],
            word: [300, 150, 360, 180],
            table: [100, 60, 240, 120],
            formula: [40, 140, 120, 170],
            quad: [[255, 155], [285, 160], [280, 180], [250, 175]],
        },
        Case {
            correction: 180,
            scale: 2,
            dimensions: [200, 400],
            word: [20, 300, 50, 360],
            table: [80, 100, 140, 240],
            formula: [30, 40, 60, 120],
            quad: [[25, 250], [45, 255], [40, 285], [20, 280]],
        },
        Case {
            correction: 270,
            scale: 2,
            dimensions: [400, 200],
            word: [40, 20, 100, 50],
            table: [160, 80, 300, 140],
            formula: [280, 30, 360, 60],
            quad: [[120, 20], [150, 25], [145, 45], [115, 40]],
        },
    ];

    fn page_pdf() -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages = document.new_object_id();
        let content = document.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Rotate" => 90,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {}, "Contents" => content,
        });
        document.objects.insert(
            pages,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
            }),
        );
        let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        document.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("valid source PDF");
        bytes
    }

    fn raw_raster() -> Arc<Vec<u8>> {
        let mut pixels = RgbImage::from_pixel(400, 200, Rgb([255, 255, 255]));
        for (edges, color) in [
            ([40, 20, 100, 50], [0, 0, 0]),
            ([160, 80, 300, 140], [0, 0, 255]),
            ([280, 30, 360, 60], [255, 0, 0]),
        ] {
            for y in edges[1]..edges[3] {
                for x in edges[0]..edges[2] {
                    pixels.put_pixel(x, y, Rgb(color));
                }
            }
        }
        let mut png = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(pixels)
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("valid colored raster");
        Arc::new(png.into_inner())
    }

    fn backend_raster(case: Case) -> (DynamicImage, u32, u32, u32) {
        let (png, width, height, correction) = upright_raster_for_backend(
            &raw_raster(),
            400,
            200,
            90,
            crate::plugins::PageOrientationHandling::RequiresUpright,
            None,
        )
        .expect("real page-upright raster preparation");
        assert_eq!((width, height, correction), (200, 400, 90));
        let upright = image::load_from_memory(&png).expect("real upright PNG");
        assert_eq!((upright.width(), upright.height()), (200, 400));
        let scaled = upright.resize_exact(
            width * case.scale,
            height * case.scale,
            image::imageops::FilterType::Nearest,
        );
        let backend = match case.correction {
            90 => scaled.rotate90(),
            180 => scaled.rotate180(),
            270 => scaled.rotate270(),
            _ => unreachable!("fixed quarter turns"),
        };
        assert_eq!(
            (backend.width(), backend.height()),
            (case.dimensions[0] * case.scale, case.dimensions[1] * case.scale)
        );
        for (edges, color) in [
            (case.word, [0, 0, 0]),
            (case.table, [0, 0, 255]),
            (case.formula, [255, 0, 0]),
        ] {
            let x = (edges[0] + edges[2]) * case.scale / 2;
            let y = (edges[1] + edges[3]) * case.scale / 2;
            assert_eq!(
                backend.to_rgb8().get_pixel(x, y).0,
                color,
                "independent backend box lies on its source marker"
            );
        }
        (backend, width, height, correction)
    }

    fn bbox(edges: [u32; 4], scale: u32) -> BoundingBox {
        BoundingBox {
            x0: f64::from(edges[0] * scale),
            y0: f64::from(edges[1] * scale),
            x1: f64::from(edges[2] * scale),
            y1: f64::from(edges[3] * scale),
        }
    }

    fn word_document(case: Case) -> InternalDocument {
        let mut document = InternalDocument::new("pdf");
        let mut word = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Word,
            },
            "word",
            0,
        );
        word.bbox = Some(bbox(case.word, case.scale));
        document.push_element(word);
        document
    }

    fn backend_table(case: Case) -> Table {
        Table {
            cells: vec![vec!["Code".into(), "Count".into()], vec!["a".into(), "2".into()]],
            columns: Some(vec!["Code".into(), "Count".into()]),
            markdown: "| Code | Count |\n|---|---|\n| a | 2 |".into(),
            bounding_box: Some(bbox(case.table, case.scale)),
            page_number: 1,
            ..Default::default()
        }
    }

    fn backend_result(case: Case, backend: &DynamicImage) -> ExtractedDocument {
        let document = word_document(case);
        let mut result = crate::extraction::derive::derive_extraction_result(
            document.clone(),
            true,
            crate::core::config::OutputFormat::Plain,
        );
        result.tables = vec![backend_table(case)];
        result.formulas = vec![Formula {
            latex: "x^2".into(),
            bbox: Some(bbox(case.formula, case.scale)),
            page: Some(1),
        }];
        assert!(
            result
                .document
                .as_ref()
                .is_some_and(|structure| !structure.nodes.is_empty())
        );
        result.ocr_internal_document = Some(document);
        result.ocr_elements = Some(public_words(case));
        for (key, value) in [
            (
                OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
                serde_json::json!(backend.width()),
            ),
            (
                OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
                serde_json::json!(backend.height()),
            ),
            (OCR_AUTO_ROTATED_METADATA_KEY, serde_json::json!(true)),
            (
                OCR_ORIENTATION_DEGREES_METADATA_KEY,
                serde_json::json!(360 - case.correction),
            ),
        ] {
            result.metadata.additional.insert(key.into(), value);
        }
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.formulas.len(), 1);
        assert_eq!(result.ocr_elements.as_ref().unwrap().len(), 2);
        result
    }

    fn public_words(case: Case) -> Vec<OcrElement> {
        vec![
            OcrElement {
                text: "word".into(),
                level: OcrElementLevel::Word,
                geometry: OcrBoundingGeometry::Rectangle {
                    left: case.word[0] * case.scale,
                    top: case.word[1] * case.scale,
                    width: (case.word[2] - case.word[0]) * case.scale,
                    height: (case.word[3] - case.word[1]) * case.scale,
                },
                confidence: OcrConfidence {
                    detection: None,
                    recognition: 0.99,
                },
                ..Default::default()
            },
            OcrElement {
                text: "quad".into(),
                level: OcrElementLevel::Word,
                geometry: OcrBoundingGeometry::Quadrilateral {
                    points: case
                        .quad
                        .iter()
                        .map(|p| (p[0] * case.scale, p[1] * case.scale).into())
                        .collect(),
                },
                confidence: OcrConfidence {
                    detection: None,
                    recognition: 0.98,
                },
                ..Default::default()
            },
        ]
    }

    fn coordinates(bbox: BoundingBox) -> [f64; 4] {
        [bbox.x0, bbox.y0, bbox.x1, bbox.y1]
    }

    fn observed(document: &InternalDocument, formula: &Formula) -> serde_json::Value {
        assert_eq!(document.tables.len(), 1, "actual mixed assembly retains the table");
        let word = document
            .elements
            .iter()
            .find(|element| element.text == "word")
            .expect("actual internal word");
        let elements = document
            .prebuilt_ocr_elements
            .as_ref()
            .expect("actual mixed public word payload");
        assert_eq!(elements.len(), 2);
        let rectangle = elements.iter().find(|element| element.text == "word").unwrap();
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = &rectangle.geometry
        else {
            panic!("rectangle source shape")
        };
        let quad = elements.iter().find(|element| element.text == "quad").unwrap();
        let OcrBoundingGeometry::Quadrilateral { points } = &quad.geometry else {
            panic!("quadrilateral source shape")
        };
        let mut points: Vec<_> = points.iter().map(|p| [p.x, p.y]).collect();
        points.sort_unstable();
        let frame = document.ocr_coordinate_frame.expect("actual page-local frame");
        serde_json::json!({
            "internal_word": coordinates(word.bbox.unwrap()), "table": coordinates(document.tables[0].bounding_box.unwrap()),
            "formula": coordinates(formula.bbox.unwrap()), "word_pixels": [left, top, width, height],
            "quad_pixels": points, "frame": [frame.width, frame.height],
        })
    }

    fn exercise(case: Case) {
        let pdf = page_pdf();
        let (source, count, rotations) = open_pdf_for_full_ocr(&pdf).expect("actual rotated PDF source");
        assert_eq!(count, 1);
        assert_eq!(rotations, [90]);
        assert_eq!(page_dimensions_pt(&source, 0), (200.0, 100.0));
        let (backend, upright_width, upright_height, correction) = backend_raster(case);
        let mut result = backend_result(case, &backend);
        undo_upright_raster_correction(&mut result, correction, upright_width, upright_height);
        let mut formula = result.formulas.pop().expect("real backend formula payload");
        formula_bbox_to_page_points(&mut formula, &source, 0, Some(&result.metadata), 400, 200);
        let config = crate::core::config::OcrConfig {
            auto_rotate: true,
            element_config: Some(crate::types::OcrElementConfig {
                include_elements: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (document, _) = build_mixed_ocr_page_document(
            &mut result,
            &config,
            1,
            400,
            200,
            200.0,
            100.0,
            crate::pdf::native::text::PageMarginFractions { top: 0.0, bottom: 0.0 },
        )
        .expect("actual mixed page assembly must run");
        let mut quad = [[120, 20], [150, 25], [145, 45], [115, 40]].map(|p| [p[0] * case.scale, p[1] * case.scale]);
        quad.sort_unstable();
        let expected = serde_json::json!({
            "internal_word": [20.0, 75.0, 50.0, 90.0], "table": [80.0, 30.0, 150.0, 60.0],
            "formula": [140.0, 70.0, 180.0, 85.0],
            "word_pixels": [40 * case.scale, 20 * case.scale, 60 * case.scale, 30 * case.scale],
            "quad_pixels": quad, "frame": [400 * case.scale, 200 * case.scale],
        });
        assert_eq!(
            observed(&document, &formula),
            expected,
            "combined page90/backend{} scale{} must return every payload to its independent page frame",
            case.correction,
            case.scale
        );
    }

    #[test]
    fn should_compose_page90_and_backend90_for_all_mixed_geometry_2356() {
        exercise(CASES[0]);
    }

    #[test]
    fn should_compose_page90_and_resampled_backend180_for_all_mixed_geometry_2356() {
        exercise(CASES[1]);
    }

    #[test]
    fn should_compose_page90_and_resampled_backend270_for_all_mixed_geometry_2356() {
        exercise(CASES[2]);
    }
}
