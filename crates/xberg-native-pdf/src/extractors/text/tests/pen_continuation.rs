//! A show that starts at the pen continues the word (GH#2664): a `Tm` that changes only the horizontal
//! scale (rule B), and a `Td` with no line move between two `TJ` arrays that mark their word spaces with
//! offsets (rule C). The fixtures follow the design lane's generator: each glyph position comes from the
//! Adobe standard-14 metrics, as a producer that lays out from the same metrics would write it. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

const Y: f32 = 600.0;

fn std_font(name: &str) -> FontInfo {
    let mut font = create_test_font();
    font.base_font = name.to_string();
    font
}

/// Times-Roman with a `/Widths` array that copies the Adobe metrics.
fn widths_font() -> FontInfo {
    let bare = create_test_font();
    let mut font = create_test_font();
    font.widths = Some((32u32..=126).map(|code| bare.get_glyph_width(code as u16)).collect());
    font.first_char = Some(32);
    font.last_char = Some(126);
    font
}

/// The advance of `text` in thousandths of an em.
fn units(font: &FontInfo, text: &str) -> f32 {
    text.bytes().map(|b| font.get_glyph_width(u16::from(b))).sum()
}

fn extract(font: FontInfo, stream: &str) -> Vec<TextSpan> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let spans = extractor.extract_text_spans(stream.as_bytes()).unwrap();
    assert!(!spans.is_empty(), "the stream produced no spans: {stream}");
    spans
}

/// The page text with a space between the runs and its whitespace collapsed: a word that two runs
/// hold is split here, as the word assembly splits it.
fn words(spans: &[TextSpan]) -> String {
    let text = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The spans separated by `|`, to show where the runs end.
fn runs(spans: &[TextSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("|")
}

/// One show of a [`Line`]: `Tm a 0 c d x y` under `1 Tf`. Without `x` the show starts `gap` points after
/// the pen; without `d` it keeps the line height of the first show, as the producer does.
#[derive(Clone)]
struct Show<'a> {
    text: &'a str,
    a: f32,
    x: Option<f32>,
    gap: f32,
    d: Option<f32>,
    tc: f32,
    c: f32,
    y: f32,
}

impl Default for Show<'_> {
    fn default() -> Self {
        Show {
            text: "",
            a: 9.1,
            x: None,
            gap: 0.0,
            d: None,
            tc: 0.0,
            c: 0.0,
            y: Y,
        }
    }
}

fn at(text: &str, a: f32, x: f32) -> Show<'_> {
    Show {
        text,
        a,
        x: Some(x),
        ..Show::default()
    }
}

fn after(text: &str, a: f32, gap: f32) -> Show<'_> {
    Show {
        text,
        a,
        gap,
        ..Show::default()
    }
}

/// A run of shows in one `BT`, each under its own text matrix.
struct Line {
    font: FontInfo,
    ops: Vec<String>,
    pen: f32,
    d0: Option<f32>,
}

impl Line {
    fn new(font: FontInfo) -> Self {
        Line {
            font,
            ops: vec!["BT /F1 1 Tf".to_string()],
            pen: 0.0,
            d0: None,
        }
    }

    fn show(mut self, show: Show<'_>) -> Self {
        let x = show.x.unwrap_or(self.pen + show.gap);
        let d0 = *self.d0.get_or_insert(show.a);
        let d = show.d.unwrap_or(d0);
        let Show { text, a, tc, c, y, .. } = show;
        self.ops.push(format!(
            "{tc:.4} Tc {a:.4} 0 {c:.4} {d:.4} {x:.4} {y:.4} Tm ({text}) Tj"
        ));
        // ISO 32000-1 9.4.4: tx = (w0 * Tfs + Tc) * Th per glyph, Tfs = 1, Th = 1, then scaled by a. ~keep
        self.pen = x + (units(&self.font, text) / 1000.0 + tc * text.len() as f32) * a;
        self
    }

    fn end(&self) -> String {
        format!("{} ET", self.ops.join(" "))
    }
}

fn times() -> Line {
    Line::new(create_test_font())
}

// ---- Rule B: only `a` changes and the show starts within the band before the pen. ~keep

#[test]
fn b01_stretched_show_that_overlaps_the_pen_continues_the_word() {
    let line = times()
        .show(at("City/Neare", 9.1, 100.0))
        .show(after("st ", 9.381, -0.28))
        .show(after("Community", 9.1, 0.0));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "City/Nearest Community", "{}", runs(&spans));
}

#[test]
fn b02_stretched_show_exactly_at_the_pen_continues_the_word() {
    let line = times()
        .show(at("Coastal ", 9.1, 100.0))
        .show(after("Perm", 9.1, 0.0))
        .show(after("it ", 9.186, 0.0));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Coastal Permit", "{}", runs(&spans));
}

#[test]
fn b03_character_spacing_counts_in_the_pen() {
    let line = times()
        .show(Show {
            tc: 0.027,
            ..at("Perm", 9.1, 100.0)
        })
        .show(Show {
            tc: 0.027,
            ..after("it ", 9.186, -0.43)
        });
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Permit", "{}", runs(&spans));
}

#[test]
fn b04_shrunk_show_continues_the_word() {
    let line = times()
        .show(at("terr", 9.381, 100.0))
        .show(after("ace, second", 9.1, -0.16));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "terrace, second", "{}", runs(&spans));
}

#[test]
fn b05_overlap_at_the_band_edge_continues_the_word() {
    let line = times()
        .show(at("Water Sup", 9.1, 100.0))
        .show(after("ply", 9.605, -0.49));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Water Supply", "{}", runs(&spans));
}

/// The later advances must scale by the new `a`, or the third pen reads 4 points off. ~keep
#[test]
fn b06_three_stretched_shows_make_one_word() {
    let line = times()
        .show(at("Char", 9.1, 100.0))
        .show(after("acteristical", 10.01, -0.2))
        .show(after("ly", 9.6, -0.2));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Characteristically", "{}", runs(&spans));
}

#[test]
fn b07_real_gap_after_a_shrunk_show_stays_a_space() {
    let line = times()
        .show(at("Pre", 10.01, 100.0))
        .show(after("determined", 9.1, -0.2))
        .show(after("rules", 9.6, 2.5));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Predetermined rules", "{}", runs(&spans));
}

/// The flushed run is as wide as its glyphs: a 1.6 point gap to the next `BT` is a space. ~keep
#[test]
fn b08_run_restated_in_the_new_unit_keeps_its_width() {
    let line = times()
        .show(at("Pre", 10.01, 100.0))
        .show(after("determined", 9.1, -0.2));
    let stream = format!(
        "{} BT /F1 9.1 Tf 1 0 0 1 {:.4} {Y} Tm (rules) Tj ET",
        line.end(),
        line.pen + 1.6
    );
    let spans = extract(create_test_font(), &stream);
    assert_eq!(words(&spans), "Predetermined rules", "{}", runs(&spans));
    let run = spans
        .iter()
        .find(|s| s.text.starts_with("Predetermined"))
        .expect("the run");
    let right = run.bbox.x + run.bbox.width;
    assert!(
        (right - line.pen).abs() < 0.05,
        "run ends at {right}, the pen is at {}",
        line.pen
    );
}

#[test]
fn b09_pen_jump_inside_the_run_then_a_stretched_show() {
    let line = times()
        .show(at("Alpha:Bravo ", 9.1, 120.0))
        .show(at("Char", 9.1, 290.0))
        .show(after("lie", 9.373, -0.28));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Alpha:Bravo Charlie", "{}", runs(&spans));
}

#[test]
fn b10_font_size_in_tf_under_a_unit_matrix() {
    let font = create_test_font();
    let tail_x = 290.0 + units(&font, "Char") / 1000.0 * 9.1 - 0.28;
    let stream = format!("BT /F1 9.1 Tf 1 0 0 1 290 {Y} Tm (Char) Tj 1.03 0 0 1 {tail_x:.4} {Y} Tm (lie) Tj ET");
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "Charlie", "{}", runs(&spans));
}

/// The band is in user space: under a CTM of 0.5 a text-space overlap of 0.56 is 0.28 points. ~keep
#[test]
fn b11_band_is_measured_in_user_space() {
    let line = times().show(at("Neare", 18.2, 200.0)).show(after("st ", 18.762, -0.56));
    let stream = format!("q 0.5 0 0 0.5 0 300 cm {} Q", line.end());
    let spans = extract(create_test_font(), &stream);
    assert_eq!(words(&spans), "Nearest", "{}", runs(&spans));
}

// ---- Outside rule B: the behaviour of the PR base. ~keep

#[test]
fn n01_overlap_beyond_the_band_does_not_continue() {
    let line = times().show(at("Plann", 9.1, 100.0)).show(after("ed", 9.247, -0.92));
    let spans = extract(create_test_font(), &line.end());
    assert!(!words(&spans).contains("Planned"), "{}", runs(&spans));
}

#[test]
fn n02_word_gap_after_a_space_glyph_stays_a_space() {
    let line = times().show(at("Plan ", 9.1, 100.0)).show(after("area", 9.381, 2.5));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Plan area", "{}", runs(&spans));
}

#[test]
fn n03_word_gap_without_a_space_glyph_stays_a_space() {
    let line = times().show(at("Plan", 9.1, 100.0)).show(after("area", 9.381, 2.5));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Plan area", "{}", runs(&spans));
}

#[test]
fn n04_small_positive_gap_goes_to_the_merge_with_the_raw_gap() {
    let line = times().show(at("Econo", 9.1, 100.0)).show(after("mic", 9.167, 0.09));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(words(&spans), "Economic", "{}", runs(&spans));
}

#[test]
fn n05_line_height_change_does_not_continue() {
    let line = times().show(at("Neare", 9.1, 100.0)).show(Show {
        d: Some(9.4),
        ..after("st", 9.381, -0.28)
    });
    let spans = extract(create_test_font(), &line.end());
    assert!(!words(&spans).contains("Nearest"), "{}", runs(&spans));
}

#[test]
fn n06_baseline_move_does_not_continue() {
    let line = times().show(at("Neare", 9.1, 100.0)).show(Show {
        y: Y + 6.0,
        ..after("st", 9.381, -0.28)
    });
    let spans = extract(create_test_font(), &line.end());
    assert!(!words(&spans).contains("Nearest"), "{}", runs(&spans));
}

#[test]
fn n07_skew_change_does_not_continue() {
    let line = times().show(at("Neare", 9.1, 100.0)).show(Show {
        c: 1.0,
        ..after("st", 9.381, -0.28)
    });
    let spans = extract(create_test_font(), &line.end());
    assert!(!words(&spans).contains("Nearest"), "{}", runs(&spans));
}

#[test]
fn n08_font_with_widths_joins_at_the_merge() {
    let line = Line::new(widths_font())
        .show(at("Neare", 9.1, 100.0))
        .show(after("st", 9.381, -0.28));
    let spans = extract(widths_font(), &line.end());
    assert_eq!(words(&spans), "Nearest", "{}", runs(&spans));
}

/// A font with fallback widths has a run wider than the Adobe pen: the stretched show is far behind
/// the pen, outside the band. ~keep
#[test]
fn n09_fallback_width_font_is_outside_the_band() {
    let line = times().show(at("Char", 9.1, 100.0)).show(after("lie", 9.373, -0.28));
    let spans = extract(std_font("InventedSans"), &line.end());
    assert!(!words(&spans).contains("Charlie"), "{}", runs(&spans));
}

/// A `BT` per table cell, the next cell 0.44 points early (the round 4 regression). ~keep
#[test]
fn n10_cells_in_their_own_bt_keep_their_space() {
    let font = std_font("Helvetica");
    let first = "U.S. Plans";
    let second_x = 100.0 + units(&font, first) / 1000.0 * 9.64 - 0.44;
    let stream = format!(
        "BT /F1 9.64 Tf 1 0 0 1 100 {Y} Tm ({first}) Tj ET \
         BT /F1 9.64 Tf 1 0 0 1 {second_x:.4} {Y} Tm (Non-U.S. Plans) Tj ET"
    );
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "U.S. Plans Non-U.S. Plans", "{}", runs(&spans));
}

/// Integer `TD` moves after plain shows (the round 4 regression): `table` is 26.68 wide, the move 26. ~keep
#[test]
fn n11_integer_td_moves_after_plain_shows_keep_their_spaces() {
    let font = std_font("Helvetica");
    let the = (units(&font, "the") / 1000.0 * 12.0 + 1.32).round();
    let table = (units(&font, "table") / 1000.0 * 12.0) as i32;
    let stream =
        format!("BT /F1 12 Tf 100 {Y} Td (the) Tj {the} 0 TD (aidof) Tj 28 0 TD (table) Tj {table} 0 TD (II) Tj ET");
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "the aidof table II", "{}", runs(&spans));
}

#[test]
fn n12_origin_before_the_run_start_does_not_continue() {
    // A run narrower than the band (a period at 1 pt) puts an origin before its start inside the band
    // of its pen. The run continues only forward of its start, as the unchanged-matrix case does. ~keep
    let line = times().show(at(".", 1.0, 100.0)).show(at("x", 1.5, 99.9));
    let spans = extract(create_test_font(), &line.end());
    assert_eq!(spans.len(), 2, "{}", runs(&spans));
}

// ---- Rule C: a `TJ` with word offsets, a `Td` with ty 0 back to the pen, the next `TJ`. ~keep

#[derive(Clone, Copy)]
enum El {
    S(&'static str),
    O(i32),
}
use El::{O, S};

struct Seam {
    gap: f32,
    mv: &'static str,
    ty: f32,
    size: f32,
    new_bt: bool,
}

impl Default for Seam {
    fn default() -> Self {
        Seam {
            gap: -0.01,
            mv: "Td",
            ty: 0.0,
            size: 10.0,
            new_bt: false,
        }
    }
}

fn array(els: &[El]) -> String {
    let body: Vec<String> = els
        .iter()
        .map(|e| match e {
            S(s) => format!("({s})"),
            O(o) => o.to_string(),
        })
        .collect();
    format!("[{}] TJ", body.join(" "))
}

/// The pen after the array `els` under `size`, in points from the start of the line.
fn advance(font: &FontInfo, els: &[El], size: f32) -> f32 {
    let units: f32 = els
        .iter()
        .map(|e| match e {
            S(s) => units(font, s),
            O(o) => -(*o as f32),
        })
        .sum();
    units / 1000.0 * size
}

/// Two `TJ` arrays; the second starts `gap` points after the pen the first one leaves.
fn tj_seam(font: &FontInfo, first: &[El], second: &[El], seam: Seam) -> String {
    let Seam {
        gap,
        mv,
        ty,
        size,
        new_bt,
    } = seam;
    let adv = advance(font, first, size);
    let head = format!("BT /F1 {size} Tf 100 {Y} Td {}", array(first));
    if new_bt {
        // A new `BT` starts at the identity, so the move is from the page origin. ~keep
        let x = 100.0 + adv + gap;
        return format!("{head} ET BT /F1 {size} Tf {x:.4} {Y} Td {} ET", array(second));
    }
    // `Td` moves from the start of the line (ISO 32000-1 9.4.2). ~keep
    let tx = adv + gap;
    format!("{head} {tx:.4} {ty} {mv} {} ET", array(second))
}

const QUICK: [El; 5] = [S("The"), O(-278), S("quick"), O(-278), S("bro")];
const WN_FOX: [El; 3] = [S("wn"), O(-278), S("fox")];

fn helvetica_seam(first: &[El], second: &[El], seam: Seam) -> Vec<TextSpan> {
    let font = std_font("Helvetica");
    let stream = tj_seam(&font, first, second, seam);
    extract(font, &stream)
}

#[test]
fn c01_td_back_to_the_pen_between_two_arrays_continues_the_word() {
    let spans = helvetica_seam(&QUICK, &WN_FOX, Seam::default());
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

#[test]
fn c01_under_a_half_ctm() {
    let font = std_font("Helvetica");
    let stream = format!(
        "q 0.5 0 0 0.5 0 300 cm {} Q",
        tj_seam(&font, &QUICK, &WN_FOX, Seam::default())
    );
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

#[test]
fn c02_overlap_inside_the_band_continues_the_word() {
    let spans = helvetica_seam(
        &QUICK,
        &WN_FOX,
        Seam {
            gap: -0.22,
            ..Seam::default()
        },
    );
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

#[test]
fn c03_td_with_leading_moves_the_pen_as_td_does() {
    let spans = helvetica_seam(
        &QUICK,
        &WN_FOX,
        Seam {
            mv: "TD",
            ..Seam::default()
        },
    );
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

#[test]
fn c04_word_offset_that_ends_the_first_array_is_a_space() {
    let first = [S("The"), O(-278), S("quick"), O(-278), S("brown"), O(-278)];
    let spans = helvetica_seam(&first, &[S("fox")], Seam::default());
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

#[test]
fn c05_word_offset_that_starts_the_second_array_is_a_space() {
    let first = [S("The"), O(-278), S("quick"), O(-278), S("brown")];
    let spans = helvetica_seam(&first, &[O(-278), S("fox")], Seam::default());
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

#[test]
fn c06_positive_gap_at_the_seam_is_a_space() {
    let spans = helvetica_seam(
        &QUICK,
        &WN_FOX,
        Seam {
            gap: 2.5,
            ..Seam::default()
        },
    );
    assert_eq!(words(&spans), "The quick bro wn fox", "{}", runs(&spans));
}

/// A `TJ` with no word offset: the class of a page that moves the line with `TD` after plain shows. ~keep
#[test]
fn c07_array_without_a_word_offset_does_not_continue() {
    let spans = helvetica_seam(&[S("Thequick"), O(-20), S("bro")], &[S("wn")], Seam::default());
    assert_eq!(words(&spans), "Thequickbro wn", "{}", runs(&spans));
}

/// The next array in a `BT` of its own: the class of a table whose cells overlap by a hair. ~keep
#[test]
fn c08_next_array_in_its_own_bt_does_not_continue() {
    let spans = helvetica_seam(
        &QUICK,
        &WN_FOX,
        Seam {
            new_bt: true,
            ..Seam::default()
        },
    );
    assert_eq!(words(&spans), "The quick bro wn fox", "{}", runs(&spans));
}

/// The next array in a `BT` of its own that first moves to the line and then along it: the seam ends
/// at the `ET`, so the second `Td` with no line move finds none. ~keep
#[test]
fn c08_second_move_in_a_new_bt_does_not_continue() {
    let font = std_font("Helvetica");
    let tx = advance(&font, &QUICK, 10.0) - 0.01;
    let stream = format!(
        "BT /F1 10 Tf 100 {Y} Td {} ET BT /F1 10 Tf 100 {Y} Td {tx:.4} 0 Td {} ET",
        array(&QUICK),
        array(&WN_FOX)
    );
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "The quick bro wn fox", "{}", runs(&spans));
}

#[test]
fn c09_td_to_the_next_line_does_not_continue() {
    let spans = helvetica_seam(
        &QUICK,
        &WN_FOX,
        Seam {
            ty: -12.0,
            ..Seam::default()
        },
    );
    assert!(runs(&spans).contains("bro|wn"), "{}", runs(&spans));
}

#[test]
fn c09_td_with_a_small_line_move_does_not_continue() {
    let spans = helvetica_seam(
        &QUICK,
        &WN_FOX,
        Seam {
            ty: 0.1,
            ..Seam::default()
        },
    );
    assert_eq!(words(&spans), "The quick bro wn fox", "{}", runs(&spans));
}

/// No `/Widths` and no Adobe metrics: the extractor's pen is the fallback's, not the producer's, so a
/// `Td` back to the extractor's own pen is not read as the producer's seam. ~keep
#[test]
fn c10_fallback_width_font_does_not_continue() {
    let font = std_font("InventedSans");
    let stream = tj_seam(&font, &QUICK, &WN_FOX, Seam::default());
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "The quick bro wn fox", "{}", runs(&spans));
}

#[test]
fn c11_font_with_widths_joins_at_the_merge() {
    let font = widths_font();
    let stream = tj_seam(&font, &QUICK, &WN_FOX, Seam::default());
    let spans = extract(font, &stream);
    assert_eq!(words(&spans), "The quick brown fox", "{}", runs(&spans));
}

// ---- The round 3 cases: a stretched show in a font with no `/Widths`, and the fallback-width font. ~keep

/// One line in a standard-14 font with no `/Widths`, set the way some form producers write it: a `1 Tf`
/// font under a text matrix that scales it to `size`, a label, a second show far to the right, and the
/// rest of that word in a show of its own that starts 0.28 points before the second show ends.
fn standard_font_split_word_stream(font: &FontInfo, scale: f32, size: f32, far_label: bool) -> String {
    let width = |text: &str| -> f32 {
        let units: f32 = text.bytes().map(|b| font.get_glyph_width(u16::from(b))).sum();
        units / 1000.0 * size * scale
    };
    let start = 290.0;
    let tail_x = start + width("Char") - 0.28;
    let label = if far_label {
        format!("{scale} 0 0 {scale} 120 600 Tm (Alpha:Bravo ) Tj ")
    } else {
        String::new()
    };
    format!(
        "BT /F1 {size} Tf {label}{scale} 0 0 {scale} {start} 600 Tm (Char) Tj {} 0 0 {scale} {tail_x:.3} 600 Tm \
         (lie) Tj ET",
        scale * 1.03
    )
}

/// A show under a text matrix that changes only the horizontal scale and starts 0.28 points before the
/// pen continues the word (GH#2664), also when the run holds a pen jump and the font has no `/Widths`.
#[test]
fn word_split_across_shows_in_a_standard_font_without_widths_stays_whole() {
    for (scale, size, far_label) in [(9.1, 1.0, true), (9.1, 1.0, false), (1.0, 9.1, true), (1.0, 9.1, false)] {
        let font = create_test_font();
        let stream = standard_font_split_word_stream(&font, scale, size, far_label);
        let mut extractor = TextExtractor::new();
        extractor.add_font("F1".to_string(), font);
        let spans = extractor.extract_text_spans(stream.as_bytes()).unwrap();
        let text = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("|");
        assert!(
            text.ends_with("Charlie"),
            "scale {scale}, size {size}, label {far_label}: {text:?}"
        );
    }
}

/// A font with no `/Widths` and no standard metrics reports the fallback advance, which is wider than
/// its glyphs: an overlap of 0.3 points still holds a word gap, and the merge keeps the space.
#[test]
fn overlap_in_a_font_with_fallback_widths_still_gets_its_space() {
    let mut font = create_test_font();
    font.base_font = "InventedSans".to_string();
    assert!(!font.has_explicit_widths() && !font.has_standard_font_metrics());
    let stream = b"BT /F1 10 Tf 1 0 0 1 72 700 Tm (Alpha) Tj ET BT /F1 10 Tf 1 0 0 1 121.7 700 Tm (Bravo) Tj ET";
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let spans = extractor.extract_text_spans(stream).unwrap();
    let text = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("|");
    assert_eq!(text, "Alpha Bravo");
}

#[test]
fn standard_font_metrics_name_the_times_helvetica_and_courier_families() {
    for (name, expected) in [
        ("Times-Roman", true),
        ("ABCDEF+Helvetica-Bold", true),
        ("Courier", true),
        ("Symbol", false),
        ("InventedSans", false),
        ("HelveticaCorp-Custom", false),
    ] {
        let mut font = create_test_font();
        font.base_font = name.to_string();
        assert_eq!(font.has_standard_font_metrics(), expected, "{name}");
    }
}
