use super::super::*;
use super::common::*;

#[test]
fn test_run_is_signed_number() {
    // Signed numeric exponents (unit notation) are detected for every
    // common minus/hyphen sign and left to the caller to skip. ~keep
    assert!(PdfDocument::run_is_signed_number("-1")); // U+002D hyphen-minus ~keep
    assert!(PdfDocument::run_is_signed_number("\u{2212}2")); // U+2212 minus sign ~keep
    assert!(PdfDocument::run_is_signed_number("\u{2010}3")); // U+2010 hyphen ~keep
    assert!(PdfDocument::run_is_signed_number("\u{2011}45"));
    // ~keep
    // A bare sign with no digit is not a signed number. ~keep
    assert!(!PdfDocument::run_is_signed_number("-"));
    // Unsigned digit runs (chemistry subscripts, ordinals, exponents the
    // plaintext convention DOES want as Unicode) are not affected. ~keep
    assert!(!PdfDocument::run_is_signed_number("2"));
    assert!(!PdfDocument::run_is_signed_number("th"));
    assert!(!PdfDocument::run_is_signed_number(""));
    // The sign must lead: an interior hyphen is not a signed exponent. ~keep
    assert!(!PdfDocument::run_is_signed_number("1-"));
}

#[test]
fn test_rotate_span_bbox_identity_and_180() {
    let r = crate::geometry::Rect::new(10.0, 20.0, 30.0, 5.0);
    let (w, h) = (200.0, 100.0);

    // rot == 0 is the identity (byte-identical, unrotated pages untouched). ~keep
    let id = PdfDocument::rotate_span_bbox(r, 0, w, h);
    assert!((id.x - r.x).abs() < 1e-4 && (id.y - r.y).abs() < 1e-4);
    assert!((id.width - r.width).abs() < 1e-4 && (id.height - r.height).abs() < 1e-4);

    // rot == 180 matches the legacy mirror: x' = w-(x+width), y' = h-(y+height). ~keep
    let m = PdfDocument::rotate_span_bbox(r, 180, w, h);
    assert!((m.x - (w - (r.x + r.width))).abs() < 1e-4, "180 x: {}", m.x);
    assert!((m.y - (h - (r.y + r.height))).abs() < 1e-4, "180 y: {}", m.y);
    assert!((m.width - r.width).abs() < 1e-4 && (m.height - r.height).abs() < 1e-4);
}

#[test]
fn test_rotate_span_bbox_90_270_roundtrip_and_swap() {
    let r = crate::geometry::Rect::new(10.0, 20.0, 30.0, 5.0);
    // 90° / 270° swap width and height of the AABB. ~keep
    let r90 = PdfDocument::rotate_span_bbox(r, 90, 200.0, 100.0);
    assert!((r90.width - r.height).abs() < 1e-4, "w/h swap: {}", r90.width);
    assert!((r90.height - r.width).abs() < 1e-4, "w/h swap: {}", r90.height);

    // Applying 90° four times around a square page returns to the start. ~keep
    let s = crate::geometry::Rect::new(12.0, 34.0, 6.0, 8.0);
    let p = 100.0;
    let a = PdfDocument::rotate_span_bbox(s, 90, p, p);
    let b = PdfDocument::rotate_span_bbox(a, 90, p, p);
    let c = PdfDocument::rotate_span_bbox(b, 90, p, p);
    let d = PdfDocument::rotate_span_bbox(c, 90, p, p);
    assert!((d.x - s.x).abs() < 1e-3, "roundtrip x: {} vs {}", d.x, s.x);
    assert!((d.y - s.y).abs() < 1e-3, "roundtrip y: {} vs {}", d.y, s.y);
    assert!((d.width - s.width).abs() < 1e-3 && (d.height - s.height).abs() < 1e-3);
}

#[test]
fn test_y_band_candidates_is_superset_of_tolerance() {
    let band = 4.0_f32;
    let spans: Vec<TextSpan> = [0.0, 1.5, 3.9, 4.0, 4.1, 8.0, 100.0, -3.0, -8.0]
        .iter()
        .map(|&y| make_test_span("x", 0.0, y, 5.0, 10.0))
        .collect();
    let idx = PdfDocument::build_y_band_index(&spans, band);
    for &cy in &[0.0_f32, 4.0, 4.05, 100.0, -3.0] {
        let got: std::collections::HashSet<usize> = PdfDocument::y_band_candidates(&idx, cy, band).collect();
        for (j, s) in spans.iter().enumerate() {
            if (s.bbox.y - cy).abs() <= band {
                assert!(
                    got.contains(&j),
                    "index missed span {j} (y={}) within band of cy={cy}",
                    s.bbox.y
                );
            }
        }
    }
}

#[test]
fn test_merge_drop_cap_initial() {
    let mut spans = vec![
        make_test_span("T", 0.0, 100.0, 14.0, 20.0),
        make_test_span("ABLE 102.3", 15.0, 100.0, 60.0, 12.0),
    ];
    PdfDocument::merge_drop_cap_initials(&mut spans);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].text, "TABLE 102.3");
    assert_eq!(spans[0].bbox.x, 0.0);
}

#[test]
fn test_merge_drop_cap_skips_same_size_capital() {
    let mut spans = vec![
        make_test_span("A", 0.0, 100.0, 8.0, 12.0),
        make_test_span("word", 9.0, 100.0, 30.0, 12.0),
    ];
    PdfDocument::merge_drop_cap_initials(&mut spans);
    assert_eq!(spans.len(), 2, "same-size capital is not a drop-cap initial");
}

#[test]
fn test_merge_drop_cap_skips_math_subscript_base() {
    let mut spans = vec![
        make_test_span("the shuffle algebra", 0.0, 100.0, 90.0, 10.0),
        make_test_span("A", 92.0, 100.0, 7.0, 10.0),
        make_test_span("st", 99.0, 98.0, 6.0, 6.5),
        make_test_span("of a statistic", 106.0, 100.0, 70.0, 10.0),
    ];
    PdfDocument::merge_drop_cap_initials(&mut spans);
    assert_eq!(spans.len(), 4, "inline math base letter is not a drop-cap initial");
    assert_eq!(spans[1].text, "A");
    assert_eq!(spans[2].text, "st");
}

#[test]
fn test_merge_drop_cap_skips_word_spaced_standalone_capital() {
    let mut spans = vec![
        make_test_span("ordinary body sentence one", 0.0, 200.0, 120.0, 10.0),
        make_test_span("ordinary body sentence two", 0.0, 188.0, 120.0, 10.0),
        make_test_span("A", 0.0, 100.0, 12.0, 18.0),
        make_test_span("Perspective", 17.0, 100.0, 90.0, 18.0),
    ];
    PdfDocument::merge_drop_cap_initials(&mut spans);
    assert_eq!(spans.len(), 4, "word-spaced standalone capital is not a drop cap");
    assert_eq!(spans[2].text, "A");
    assert_eq!(spans[3].text, "Perspective");
}

#[test]
fn test_order_rotated_blocks_groups_by_rotation() {
    let mk = |t: &str, x: f32, y: f32, rot: f32| {
        let mut s = make_test_span(t, x, y, 10.0, 10.0);
        s.rotation_degrees = rot;
        s
    };
    // Two 90° runs (seen first) then one -90° run. ~keep
    let spans = vec![
        mk("A", 10.0, 50.0, 90.0),
        mk("B", 10.0, 80.0, 90.0),
        mk("C", 200.0, 50.0, -90.0),
    ];
    let out = PdfDocument::order_rotated_blocks(spans);
    assert_eq!(out.len(), 3, "no spans dropped");
    // Groups stay contiguous in first-seen order; 90° block before -90°. ~keep
    let rots: Vec<f32> = out.iter().map(|s| s.rotation_degrees).collect();
    assert_eq!(rots, vec![90.0, 90.0, -90.0]);
    // Within the 90° block, upright-frame order keeps A before B. ~keep
    assert_eq!(out[0].text, "A");
    assert_eq!(out[1].text, "B");
}

#[test]
fn test_merge_drop_cap_does_not_reach_line_above() {
    // A tall oversized "A" (16.8pt, baseline y=328) whose bbox top reaches
    // up into the previous line ("Or if", y~342). It must NOT merge with
    // "if" on the line above — only with same-baseline words on its own line
    // (which here are word-spaced and so also stay separate). Reproduces the
    // alice_old "OrAif" corruption. ~keep
    let mut spans = vec![
        make_test_span("Or", 44.0, 344.0, 14.4, 12.0),
        make_test_span("if", 62.0, 342.5, 10.7, 8.9),
        make_test_span("Idrop upon my toe", 74.0, 343.9, 90.0, 12.2),
        make_test_span("A", 54.7, 328.1, 10.1, 16.8),
        make_test_span("very heavy weight", 69.8, 327.8, 90.0, 8.4),
    ];
    PdfDocument::merge_drop_cap_initials(&mut spans);
    assert!(
        spans.iter().all(|s| s.text != "Aif" && !s.text.contains("OrA")),
        "tall initial must not steal a word from the line above"
    );
    assert!(
        spans.iter().any(|s| s.text == "A"),
        "initial left intact on its own line"
    );
}

#[test]
fn test_fix_digit_logicalnot_decimal() {
    // `¬` between digits → `.`; spaced or non-digit-flanked `¬` is left alone. ~keep
    assert_eq!(PdfDocument::fix_digit_logicalnot_decimal("1\u{00AC}00"), "1.00");
    assert_eq!(
        PdfDocument::fix_digit_logicalnot_decimal("0\u{00AC}75 1\u{00AC}00"),
        "0.75 1.00"
    );
    assert_eq!(
        PdfDocument::fix_digit_logicalnot_decimal("A \u{00AC} B"),
        "A \u{00AC} B"
    );
    assert_eq!(
        PdfDocument::fix_digit_logicalnot_decimal("5 \u{00AC} 3"),
        "5 \u{00AC} 3"
    );
    assert_eq!(PdfDocument::fix_digit_logicalnot_decimal("\u{00AC}5"), "\u{00AC}5");
    // Spaced decimal: a subset that emits a single space between the decimal
    // glyph and the fractional digits → drop the lone space, recover `.`. ~keep
    assert_eq!(PdfDocument::fix_digit_logicalnot_decimal("1\u{00AC} 00"), "1.00");
    assert_eq!(
        PdfDocument::fix_digit_logicalnot_decimal("0\u{00AC} 75 1\u{00AC} 00"),
        "0.75 1.00"
    );
    // Still NOT a decimal when the leading digit does not abut `¬`
    // (genuine spaced negation): `5 ¬ 3` stays untouched even though a
    // digit follows the space. ~keep
    assert_eq!(
        PdfDocument::fix_digit_logicalnot_decimal("5 \u{00AC} 3"),
        "5 \u{00AC} 3"
    );
    assert_eq!(
        PdfDocument::fix_digit_logicalnot_decimal("1\u{00AC}  00"),
        "1\u{00AC}  00"
    );
}

fn stamp_decimal_with_nearby_dot(extra_dot_x: u32) -> Vec<f32> {
    let stream = format!(
        "BT /F1 4 Tf 1 0 0 1 13 60 Tm (1) Tj 1 0 0 1 14 60 Tm (2) Tj \
         1 0 0 1 16 60 Tm (3) Tj 1 0 0 1 25 60 Tm (.) Tj 1 0 0 1 29 60 Tm (4) Tj \
         1 0 0 1 {extra_dot_x} 60 Tm (.) Tj ET\n"
    );
    let content = format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len());
    let pdf = super::pdf_fixtures::build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"),
            (4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
            (5, content.as_bytes()),
        ],
    );
    let document = PdfDocument::from_bytes(pdf).expect("parse decimal glyph fixture");
    let mut dots: Vec<_> = document
        .extract_chars(0)
        .unwrap()
        .into_iter()
        .filter(|glyph| glyph.char == '.')
        .map(|glyph| glyph.origin_x)
        .collect();
    dots.sort_by(f32::total_cmp);
    let mut expected_dots = [25.0, extra_dot_x as f32];
    expected_dots.sort_by(f32::total_cmp);
    assert_eq!(dots, expected_dots);
    let mut span = crate::layout::TextSpan {
        text: "123.4".to_owned(),
        bbox: crate::geometry::Rect::new(13.0, 60.0, 20.0, 4.0),
        font_size: 4.0,
        ..Default::default()
    };
    document.stamp_table_decimal_glyph_positions(0, std::slice::from_mut(&mut span));
    span.char_x_offsets
}

#[test]
fn decimal_stamp_rejects_an_overprinted_dot_inside_the_span() {
    assert!(stamp_decimal_with_nearby_dot(17).is_empty());
}

#[test]
fn decimal_stamp_retains_the_unique_sequence_with_a_dot_outside_the_span() {
    assert_eq!(stamp_decimal_with_nearby_dot(34), [13.0, 14.0, 16.0, 25.0, 29.0]);
}

#[test]
fn dense_decimal_baseline_leaves_unsearched_words_without_offsets() {
    use std::fmt::Write as _;

    let mut stream = String::from("BT /F1 8 Tf\n");
    let mut spans = Vec::new();
    for word in 0..64 {
        let x = 10 + word * 20;
        writeln!(stream, "1 0 0 1 {x} 60 Tm (1.0) Tj").unwrap();
        spans.push(crate::layout::TextSpan {
            text: "1.0".to_owned(),
            bbox: crate::geometry::Rect::new(x as f32, 60.0, 15.0, 8.0),
            font_size: 8.0,
            char_x_offsets: vec![-1.0; 3],
            ..Default::default()
        });
    }
    stream.push_str("ET\n");
    let content = format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len());
    let pdf = super::pdf_fixtures::build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1320 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"),
            (4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
            (5, content.as_bytes()),
        ],
    );
    let document = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(document.extract_chars(0).unwrap().len(), 64 * 3);
    document.stamp_table_decimal_glyph_positions(0, &mut spans);
    assert_eq!(spans[0].char_x_offsets.len(), 3);
    assert_eq!(spans[0].char_x_offsets[0], 10.0);
    assert!(spans[63].char_x_offsets.is_empty(), "unsearched word retains offsets");
    assert_eq!(spans.iter().filter(|span| !span.char_x_offsets.is_empty()).count(), 32);
}

/// `stamp_char_x_offsets`'s span loop before its baseline rows were shared across spans.
fn stamp_char_x_offsets_oracle(
    accurate: &[crate::layout::TextChar],
    spans: &mut [crate::layout::TextSpan],
    exact_decimals: bool,
) {
    let is_decimal_word = |span: &crate::layout::TextSpan| {
        span.text.contains('.') && span.text.chars().any(|glyph| glyph.is_ascii_digit())
    };
    let mut by_y: Vec<u32> = (0..accurate.len() as u32).collect();
    by_y.sort_by(|&a, &b| crate::utils::safe_float_cmp(accurate[a as usize].origin_y, accurate[b as usize].origin_y));
    let ys: Vec<f32> = by_y.iter().map(|&i| accurate[i as usize].origin_y).collect();
    let mut decimal_candidates_left = accurate.len().saturating_mul(32);
    for span in spans.iter_mut() {
        if span.rotation_degrees != 0.0 || (exact_decimals && !is_decimal_word(span)) {
            continue;
        }
        span.char_x_offsets.clear();
        if exact_decimals && decimal_candidates_left == 0 {
            continue;
        }
        let glyphs: Vec<char> = span.text.chars().collect();
        let n = glyphs.len();
        if n == 0 {
            continue;
        }
        let baseline_tol = 0.6 * span.font_size.max(1.0);
        let bracket = baseline_tol + baseline_tol.abs() * 1e-6 + f32::EPSILON;
        let lo = ys.partition_point(|&y| y < span.bbox.y - bracket);
        let hi = ys.partition_point(|&y| y <= span.bbox.y + bracket);
        if exact_decimals {
            let Some(remaining) = decimal_candidates_left.checked_sub(hi - lo) else {
                decimal_candidates_left = 0;
                continue;
            };
            decimal_candidates_left = remaining;
        }
        let horizontal = span.bbox.left()..span.bbox.right();
        let mut idx: Vec<u32> = by_y[lo..hi]
            .iter()
            .copied()
            .filter(|&i| {
                let glyph = &accurate[i as usize];
                (glyph.origin_y - span.bbox.y).abs() <= baseline_tol
                    && (!exact_decimals || horizontal.contains(&glyph.origin_x))
            })
            .collect();
        if idx.is_empty() {
            continue;
        }
        idx.sort_by(|&a, &b| {
            crate::utils::safe_float_cmp(accurate[a as usize].origin_x, accurate[b as usize].origin_x).then(a.cmp(&b))
        });
        let line: Vec<&crate::layout::TextChar> = idx.iter().map(|&i| &accurate[i as usize]).collect();
        if exact_decimals {
            let mut source = line.iter();
            let baseline = line[0].origin_y;
            let positions = glyphs
                .iter()
                .map(|glyph| {
                    source
                        .next()
                        .filter(|item| {
                            item.char == *glyph
                                && item.rotation_degrees == 0.0
                                && (item.origin_y - baseline).abs() <= 0.05
                        })
                        .map(|item| item.origin_x)
                })
                .collect::<Option<Vec<_>>>();
            if let Some(positions) = positions
                && source.next().is_none()
                && positions.windows(2).all(|pair| pair[0] < pair[1])
            {
                span.char_x_offsets = positions;
            }
            continue;
        }
        let start = line.iter().position(|c| c.origin_x >= span.bbox.x - 0.5).unwrap_or(0);
        let mut assigned: Vec<Option<f32>> = vec![None; n];
        let mut li = start;
        for (k, &g) in glyphs.iter().enumerate() {
            let mut j = li;
            let mut steps = 0;
            while j < line.len() && steps < 6 {
                if line[j].char == g {
                    assigned[k] = Some(line[j].origin_x);
                    li = j + 1;
                    break;
                }
                j += 1;
                steps += 1;
            }
        }
        let anchors = assigned.iter().filter(|a| a.is_some()).count();
        if anchors * 5 < n * 3 {
            continue;
        }
        let cw = &span.char_widths;
        let width_at = |i: usize| -> f32 {
            if cw.len() == n {
                cw[i]
            } else {
                span.bbox.width / n as f32
            }
        };
        let mut offs = vec![0.0f32; n];
        let mut last: Option<(usize, f32)> = None;
        for k in 0..n {
            if let Some(x) = assigned[k] {
                offs[k] = x;
                last = Some((k, x));
            } else if let Some((lk, lx)) = last {
                let acc: f32 = (lk..k).map(width_at).sum();
                offs[k] = lx + acc;
            }
        }
        if assigned[0].is_none()
            && let Some(fk) = assigned.iter().position(|a| a.is_some())
        {
            let fx = assigned[fk].unwrap();
            for k in 0..fk {
                let acc: f32 = (k..fk).map(width_at).sum();
                offs[k] = fx - acc;
            }
        }
        span.char_x_offsets = offs;
    }
}

/// A deterministic xorshift stream, so a failing seed reproduces.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

fn random_stamp_page(rng: &mut Xorshift) -> (Vec<crate::layout::TextChar>, Vec<crate::layout::TextSpan>) {
    const GLYPHS: &[char] = &['1', '2', '.', '3', 'a', 'b', ' '];
    const BASELINES: &[f32] = &[100.0, 100.4, 103.0, 110.0, 110.05, 125.0];
    let mut chars = Vec::new();
    for _ in 0..rng.below(90) + 1 {
        let x = match rng.below(20) {
            0 => f32::NAN,
            1 => -0.0,
            _ => (rng.below(120) as f32) * 0.5,
        };
        chars.push(crate::layout::TextChar {
            char: rng.pick(GLYPHS),
            bbox: crate::geometry::Rect::new(x, 0.0, 1.0, 4.0),
            font_name: "F1".into(),
            font_size: 4.0,
            font_weight: crate::layout::FontWeight::Normal,
            color: crate::layout::Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: x,
            origin_y: rng.pick(BASELINES) + rng.pick(&[0.0f32, 0.02, -0.3, 2.4, 2.41]),
            rotation_degrees: rng.pick(&[0.0, 0.0, 0.0, 90.0]),
            advance_width: 1.0,
            rendered_advance: 1.0,
            ascent: 3.8,
            descent: -1.4,
            matrix: None,
        });
    }
    let mut spans = Vec::new();
    for _ in 0..rng.below(80) + 1 {
        if rng.below(2) == 0 {
            let y = rng.pick(BASELINES);
            let mut row: Vec<&crate::layout::TextChar> = chars
                .iter()
                .filter(|glyph| glyph.origin_y == y && !glyph.origin_x.is_nan())
                .collect();
            row.sort_by(|a, b| a.origin_x.total_cmp(&b.origin_x));
            if !row.is_empty() {
                let start = rng.below(row.len());
                let end = (start + 1 + rng.below(6)).min(row.len());
                let left = row[start].origin_x;
                let width = row[end - 1].origin_x + 0.25 - left;
                spans.push(crate::layout::TextSpan {
                    text: row[start..end].iter().map(|glyph| glyph.char).collect(),
                    bbox: crate::geometry::Rect::new(left, y, width, 4.0),
                    font_size: 4.0,
                    ..Default::default()
                });
                continue;
            }
        }
        let length = rng.below(7);
        let text: String = (0..length).map(|_| rng.pick(GLYPHS)).collect();
        let char_widths = if rng.below(2) == 0 {
            (0..length).map(|_| rng.below(4) as f32 * 0.5).collect()
        } else {
            vec![1.0; rng.below(3)]
        };
        spans.push(crate::layout::TextSpan {
            text,
            bbox: crate::geometry::Rect::new(
                rng.pick(&[f32::NAN, 0.0, 3.5, 10.0, 25.0, 40.0]),
                rng.pick(BASELINES) + rng.pick(&[0.0f32, 0.0, 0.3, f32::NAN]),
                rng.pick(&[0.0, 2.5, 8.0, 30.0]),
                4.0,
            ),
            font_size: rng.pick(&[0.5, 4.0, 4.0, 8.0, f32::NAN]),
            rotation_degrees: rng.pick(&[0.0, 0.0, 0.0, 0.0, 90.0]),
            char_widths,
            char_x_offsets: vec![-1.0; rng.below(3)],
            ..Default::default()
        });
    }
    (chars, spans)
}

fn offset_bits(spans: &[crate::layout::TextSpan]) -> Vec<Vec<u32>> {
    spans
        .iter()
        .map(|span| span.char_x_offsets.iter().map(|x| x.to_bits()).collect())
        .collect()
}

#[test]
fn stamp_with_shared_baseline_rows_matches_per_span_selection() {
    let mut rng = Xorshift(0x9e37_79b9_7f4a_7c15);
    let mut stamped = [0usize; 2];
    for case in 0..3000 {
        let (chars, spans) = random_stamp_page(&mut rng);
        for (mode, exact_decimals) in [false, true].into_iter().enumerate() {
            let mut expected = spans.clone();
            stamp_char_x_offsets_oracle(&chars, &mut expected, exact_decimals);
            let mut actual = spans.clone();
            PdfDocument::stamp_char_x_offsets_from_chars(&chars, &mut actual, exact_decimals);
            assert_eq!(
                offset_bits(&actual),
                offset_bits(&expected),
                "case {case}, exact_decimals {exact_decimals}"
            );
            stamped[mode] += expected.iter().filter(|span| !span.char_x_offsets.is_empty()).count();
        }
    }
    assert!(
        stamped.iter().all(|&count| count > 100),
        "the oracle stamped too few spans: {stamped:?}"
    );
}
