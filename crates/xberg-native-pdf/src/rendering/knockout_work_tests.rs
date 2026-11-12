use super::*;
use crate::document::PdfDocument;
use crate::rendering::{PageRenderer, RenderOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};

fn document(content: &str, knockout: bool, form: Option<&str>) -> PdfDocument {
    let group = format!("/Group << /S /Transparency /I true /K {knockout} >>");
    let resources = "/ExtGState << /Half << /ca 0.5 >> >> /XObject << /Fm 5 0 R >> /Properties << /L 6 0 R /V 7 0 R >> \
        /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> \
        /F3 << /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [1 0 0 1 0 0] /CharProcs << >> \
        /Encoding << /Type /Encoding /Differences [] >> /FirstChar 0 /LastChar 0 /Widths [0] >> >>";
    let stream = |dict: &str, data: &str| format!("<< {dict} /Length {} >>\nstream\n{data}\nendstream", data.len());
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R 7 0 R] /D << /OFF [6 0 R] >> >> >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 72 72] /Contents 4 0 R /Resources << {resources} >> {} >>",
            if form.is_none() { group.as_str() } else { "" }
        ),
        stream("", content),
        stream(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 72 72] /Resources << {resources} >> {group}"),
            form.unwrap_or(""),
        ),
        "<< /Type /OCG /Name (L) >>".to_string(),
        "<< /Type /OCG /Name (V) >>".to_string(),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(bytes).expect("valid synthetic PDF")
}

#[test]
fn sequential_small_knockout_forms_retain_their_planned_surface_work() {
    let forms = 64;
    let doc = document(&"/Fm Do ".repeat(forms), true, Some("0.2 g 0 0 72 72 re f"));
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let image = renderer.render_page(&doc, 0).expect("sequential sibling forms");
    assert_eq!(&image.data[..4], &[51, 51, 51, 255]);
    assert_eq!(
        renderer.paint_operator_calls,
        forms * 2,
        "each Do and real fill executed"
    );
    assert_eq!(
        renderer.knockout_resources.surface_work,
        2 * 72 * 72 * forms as u64,
        "each sibling charges both planned knockout work and its actual path raster"
    );
}

#[derive(Clone)]
struct WarningLog(Arc<Mutex<Vec<u8>>>);

impl Write for WarningLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("warning log").extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn oversized_knockout_pages_warn_and_match_normal_compositing_at_300_and_600_dpi() {
    for (dpi, tiny_paints) in [(300, 12_000), (600, 3_000)] {
        let body = format!(
            "{} /Half gs 0 g 0 0 72 72 re f 1 g 0 0 72 72 re f",
            "60 60 0.1 0.1 re f ".repeat(tiny_paints)
        );
        let normal = document(&body, false, None);
        let knockout = document(&body, true, None);
        let render = |doc: &PdfDocument| PageRenderer::new(RenderOptions::with_dpi(dpi).as_raw()).render_page(doc, 0);
        let expected = render(&normal).expect("ordinary compositing control");
        assert_eq!((expected.width, expected.height), (dpi, dpi));
        assert!(expected.data[0].abs_diff(191) <= 1, "real translucent overlap control");
        let log = WarningLog(Arc::new(Mutex::new(Vec::new())));
        let writer = log.clone();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_max_level(tracing::Level::WARN)
            .with_writer(move || writer.clone())
            .finish();
        // tracing-core caches a callsite's Interest from the live dispatchers, and with exactly one
        // registered it consults only the registering thread's default. A parallel test that first hits
        // the degradation warning without a subscriber would cache "never" for this test's capture too.
        // A second live dispatcher makes registration read every dispatcher, and the rebuild replaces an
        // Interest cached earlier. ~keep
        let keepalive = tracing::Dispatch::new(tracing_subscriber::fmt().with_writer(std::io::sink).finish());
        let actual = tracing::subscriber::with_default(subscriber, || {
            tracing::callsite::rebuild_interest_cache();
            render(&knockout)
        })
        .expect("oversized legitimate group degrades instead of failing the page");
        assert_eq!(
            actual.data, expected.data,
            "fallback follows ordinary paint order at {dpi} dpi"
        );
        drop(keepalive);
        let warnings = String::from_utf8(log.0.lock().expect("warning log").clone()).expect("UTF8 warning");
        assert!(
            warnings.contains("knockout") && warnings.contains("non-knockout"),
            "degradation warning at {dpi} dpi: {warnings}"
        );
    }
}

#[test]
fn below_work_limit_knockout_pages_keep_the_initial_backdrop_semantics() {
    let doc = document("/Half gs 0 g 0 0 72 72 re f 1 g 0 0 72 72 re f", true, None);
    let image = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
        .render_page(&doc, 0)
        .expect("exact knockout control");
    assert_eq!(&image.data[..4], &[255, 255, 255, 255]);
}

#[test]
fn runtime_work_exhaustion_restarts_the_group_from_its_initial_backdrop() {
    let body = "/Half gs 0 g 0 0 72 72 re f 1 g 0 0 72 72 re f";
    let doc = document(body, true, None);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let resources = doc.get_page_resources(0).expect("page resources");
    renderer.load_resources(&doc, &resources).expect("load resources");
    let operators = crate::content::parse_content_stream(body.as_bytes()).expect("real paint operators");
    let mut pixmap = tiny_skia::Pixmap::new(72, 72).expect("initial backdrop");
    pixmap.fill(tiny_skia::Color::WHITE);
    renderer.knockout_resources.canvas(pixmap.data().len());
    renderer.knockout_resources.surface_work = MAX_SURFACE_WORK - 2 * 72 * 72;
    renderer
        .execute_knockout_group(
            &mut pixmap,
            tiny_skia::Transform::identity(),
            &operators,
            &doc,
            0,
            &resources,
            Some(0),
        )
        .expect("runtime work exhaustion degrades transactionally");
    assert!(
        pixmap.data()[0].abs_diff(191) <= 1,
        "retry must discard all partial knockout paint: {:?}",
        &pixmap.data()[..4]
    );
    renderer
        .knockout_resources
        .check()
        .expect("handled work refusal cannot poison the page");
}

#[test]
fn ordinary_group_after_work_fallback_can_still_render() {
    let body = "/Half gs 0 g 0 0 72 72 re f 1 g 0 0 72 72 re f";
    let doc = document(body, true, None);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let resources = doc.get_page_resources(0).expect("page resources");
    renderer.load_resources(&doc, &resources).expect("load resources");
    let operators = crate::content::parse_content_stream(body.as_bytes()).expect("real paint operators");
    let mut pixmap = tiny_skia::Pixmap::new(72, 72).expect("initial backdrop");
    pixmap.fill(tiny_skia::Color::WHITE);
    renderer.knockout_resources.canvas(pixmap.data().len());
    renderer.knockout_resources.surface_work = MAX_SURFACE_WORK - 2 * 72 * 72;
    drop(
        renderer
            .execute_knockout_group(
                &mut pixmap,
                tiny_skia::Transform::identity(),
                &operators,
                &doc,
                0,
                &resources,
                Some(0),
            )
            .expect("first group degrades"),
    );
    let ordinary = renderer
        .knockout_resources
        .group(pixmap.data().len(), Some(0))
        .expect("ordinary sibling workspace");
    pixmap.fill(tiny_skia::Color::WHITE);
    renderer
        .execute_operators(
            &mut pixmap,
            tiny_skia::Transform::identity(),
            &operators,
            &doc,
            0,
            &resources,
        )
        .expect("ordinary sibling can render after knockout budget exhaustion");
    assert!(pixmap.data()[0].abs_diff(191) <= 1);
    drop(ordinary);
}

#[test]
fn degraded_groups_keep_memory_depth_and_operation_failures_fatal() {
    let mut resources = KnockoutResources::default();
    resources.reset(Some(16));
    resources.canvas(8);
    let fallback = resources.fallback().expect("degraded scope");
    assert!(
        resources
            .knockout(1, 0, 0, 0, 1, 1)
            .expect("inherited degradation")
            .is_none()
    );
    assert!(resources.reserve(Some(9)).is_err(), "memory admission remains active");
    drop(fallback);
    assert!(resources.check().is_err(), "fatal memory refusal remains latched");

    resources.reset(None);
    let fallback = resources.fallback().expect("operation scope");
    resources.paint_operations = MAX_PAINT_OPERATIONS;
    assert!(resources.paint(0).is_err(), "paint operation limit remains fatal");
    drop(fallback);
    assert!(resources.check().is_err());

    resources.reset(None);
    let fallback = resources.fallback().expect("depth scope");
    let mut frames = Vec::new();
    for _ in 1..MAX_NESTING_DEPTH {
        frames.push(resources.reserve(Some(0)).expect("bounded depth"));
    }
    assert!(resources.reserve(Some(0)).is_err(), "depth admission remains active");
    drop(frames);
    drop(fallback);
}

#[test]
fn planned_work_is_committed_only_after_successful_native_admission() {
    let mut resources = KnockoutResources::default();
    resources.surface_work = MAX_SURFACE_WORK - 20;
    let admitted = resources
        .knockout(10, 0, 0, 0, 1, 1)
        .expect("exact threshold")
        .expect("native");
    assert_eq!(resources.surface_work, MAX_SURFACE_WORK - 10);
    resources.raster(10).expect("the pass spends its planned floor");
    assert_eq!(resources.surface_work, MAX_SURFACE_WORK);
    drop(admitted);
    assert!(resources.knockout(1, 0, 0, 0, 1, 1).expect("graceful excess").is_none());
    let fallback = resources.fallback().expect("work-only fallback");
    resources
        .raster(usize::MAX)
        .expect("normal raster is not charged as knockout work");
    assert_eq!(resources.surface_work, MAX_SURFACE_WORK, "no refund or reset");
    drop(fallback);
    resources.raster(1).expect("ordinary siblings remain renderable");
    assert_eq!(
        resources.surface_work,
        MAX_SURFACE_WORK + 1,
        "normal charging resumes after scope drop"
    );
    resources.reset(Some(1));
    assert!(resources.knockout(1, 1, 0, 0, 1, 1).is_err(), "failed memory admission");
    assert_eq!(resources.surface_work, 0, "a failed memory reservation charges no work");
}

fn blended_form_result(knockout: bool, nested: bool, headroom: u64) -> (Vec<u8>, Vec<u8>) {
    use crate::content::GraphicsState;
    use crate::object::{Object, ObjectRef};
    use crate::rendering::sidecar::CmykSidecar;
    let body = "/Half gs 0 0 0 1 k 0 0 72 72 re f 0 0 0 0 k 0 0 72 72 re f";
    let doc = document("/Fm Do", knockout, Some(body));
    let object = doc.load_object(ObjectRef::new(5, 0)).expect("real group stream");
    let mut dict = object.as_dict().expect("group dictionary").clone();
    let mut group = dict
        .get("Group")
        .and_then(Object::as_dict)
        .expect("transparency")
        .clone();
    group.insert("I".into(), Object::Boolean(false));
    if nested {
        group.insert("K".into(), Object::Boolean(false));
    }
    dict.insert("Group".into(), Object::Dictionary(group));
    let resources = doc.get_page_resources(0).expect("resources");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer.load_resources(&doc, &resources).expect("load resources");
    let mut pixmap = tiny_skia::Pixmap::new(72, 72).expect("parent backdrop");
    pixmap.fill(tiny_skia::Color::from_rgba8(204, 204, 204, 255));
    let mut sidecar = CmykSidecar::new(72, 72, Vec::new());
    for pixel in sidecar.cmyk_mut().as_chunks_mut::<4>().0 {
        pixel[3] = 51;
    }
    renderer.cmyk_sidecar = Some(sidecar);
    renderer.knockout_resources.canvas(pixmap.data().len() * 2);
    renderer.knockout_resources.surface_work = MAX_SURFACE_WORK - headroom * 72 * 72;
    let gs = GraphicsState {
        fill_alpha: 0.5,
        blend_mode: "Multiply".into(),
        ..Default::default()
    };
    renderer
        .render_form_xobject_with_invocation(
            &mut pixmap,
            &dict,
            if nested { b"/Fm Do" } else { body.as_bytes() },
            tiny_skia::Transform::identity(),
            &doc,
            0,
            &resources,
            Some(super::super::group_compositing::FormInvocation {
                gs: &gs,
                clip: None,
                mask_transform: tiny_skia::Transform::identity(),
                mask_resources: &resources,
            }),
        )
        .expect("blended group and alpha recovery");
    renderer.knockout_resources.check().expect("no swallowed work failure");
    (
        pixmap.data().to_vec(),
        renderer.cmyk_sidecar.expect("process sidecar").cmyk().to_vec(),
    )
}

#[test]
fn nonisolated_blend_replay_degrades_pixels_and_process_plates_together() {
    let normal = blended_form_result(false, false, 4);
    assert!(normal.0[0] < 204 && normal.1[3] > 51, "real color and plate control");
    assert!(
        blended_form_result(true, false, 4) == normal,
        "a degraded alpha pass requires ordinary source paint too"
    );
}

#[test]
fn nested_knockout_fallback_keeps_outer_color_and_alpha_replay_consistent() {
    let normal = blended_form_result(false, true, 6);
    assert!(
        normal.0[0] < 204 && normal.1[3] > 51,
        "real nested color and plate control"
    );
    assert!(
        blended_form_result(true, true, 6) == normal,
        "when a descendant degrades during replay, color and alpha must both use ordinary compositing"
    );
}

#[test]
fn ordinary_alpha_replay_can_start_after_its_knockout_child_fills_the_work_ledger() {
    let normal = blended_form_result(false, true, 6);
    assert!(
        blended_form_result(true, true, 5) == normal,
        "ordinary alpha replay must reach the nested work fallback after exact color exhausts the ledger"
    );
}

#[test]
fn successful_knockout_does_not_disable_the_ordinary_surface_work_ceiling() {
    let mut resources = KnockoutResources::default();
    let native = resources
        .knockout(10, 40, 0, 0, 1, 1)
        .expect("small knockout admission")
        .expect("native frame");
    resources
        .raster(10)
        .expect("one real surface pass under the native frame");
    drop(native);
    let ordinary = resources.group(40, Some(0)).expect("ordinary sibling workspace");
    resources
        .raster(MAX_SURFACE_WORK as usize)
        .expect("ordinary sibling has its own work allowance");
    assert!(
        resources.raster(1).is_err(),
        "ordinary work beyond its ceiling must still fail"
    );
    drop(ordinary);
}

#[test]
fn plan_is_refused_before_the_pass_when_its_in_pass_floor_passes_the_limit() {
    let pixels = 1 << 20;
    let mut resources = KnockoutResources::default();
    let fits = resources
        .knockout(pixels, 0, 0, 0, 500, 500)
        .expect("plan and floor of 500 surfaces fit")
        .expect("native");
    assert_eq!(resources.surface_work, 500 << 20);
    drop(fits);

    resources.reset(None);
    assert!(
        resources
            .knockout(pixels, 0, 0, 0, 600, 600)
            .expect("graceful degradation")
            .is_none(),
        "600 surfaces plan within the limit and charge 600 more while running"
    );
    assert_eq!(resources.surface_work, 0, "a refused plan commits no work");
    assert_eq!(resources.native.load(Ordering::Relaxed), 0);

    assert!(
        resources
            .knockout(pixels, 0, 0, 0, 300, 800)
            .expect("graceful degradation")
            .is_none(),
        "text that also charges a coverage surface raises the floor above the plan"
    );
    assert!(resources.knockout(pixels, 0, 0, 0, 300, 600).expect("fits").is_some());
}

fn text_shows(content: &str, initial: Option<u8>) -> usize {
    let doc = document(content, true, None);
    let renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let resources = doc.get_page_resources(0).expect("page resources");
    let operators = crate::content::parse_content_stream(content.as_bytes()).expect("real text operators");
    renderer.painting_text_shows(&operators, &doc, &resources, initial)
}

#[test]
fn only_text_that_certainly_paints_a_side_raises_the_floor() {
    let show = "(M) Tj (M) Tj";
    let cases: [(&str, Option<u8>, usize); 14] = [
        ("BT /F1 8 Tf 0 Tr (M) Tj (M) Tj ET", Some(0), 2),
        ("BT /F1 8 Tf (M) Tj ET", Some(0), 1),
        ("BT /F1 8 Tf (M) Tj ET", None, 0),
        ("BT /F1 8 Tf 3 Tr (M) Tj (M) Tj ET", Some(0), 0),
        ("BT /F1 8 Tf 7 Tr (M) Tj ET", Some(0), 0),
        ("BT /F1 8 Tf 2 Tr (M) Tj ET", None, 1),
        ("BT /F1 8 Tf 1 Tr (M) Tj ET", None, 1),
        ("q 3 Tr Q BT /F1 8 Tf (M) Tj ET", Some(0), 1),
        ("q BT /F1 8 Tf 3 Tr (M) Tj ET Q BT /F1 8 Tf (M) Tj ET", Some(0), 1),
        ("/OC /Layer BDC BT /F1 8 Tf (M) Tj ET EMC", Some(0), 0),
        ("/Span BMC BT /F1 8 Tf (M) Tj ET EMC", Some(0), 1),
        ("BT /F3 8 Tf (M) Tj ET", Some(0), 0),
        ("BT /F9 8 Tf (M) Tj ET", Some(0), 0),
        ("BT (M) Tj ET", Some(0), 0),
    ];
    for (content, initial, expected) in cases {
        assert_eq!(
            text_shows(content, initial),
            expected,
            "{content} starting in mode {initial:?}"
        );
    }
    assert_eq!(text_shows(&format!("BT /F1 8 Tf 3 Tr {show} 0 Tr {show} ET"), None), 2);
}

/// Runs `content` as a knockout group on a 72x72 page whose work ledger leaves room for exactly the
/// plan and the in-pass floor of `shows` paints, and returns whether it stayed native, with the
/// paint operators executed.
fn native_with_room_for_floor(content: &str, shows: usize, initial: Option<u8>) -> (bool, usize) {
    let doc = document(content, true, None);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let resources = doc.get_page_resources(0).expect("page resources");
    renderer.load_resources(&doc, &resources).expect("load resources");
    let operators = crate::content::parse_content_stream(content.as_bytes()).expect("real text operators");
    let mut pixmap = tiny_skia::Pixmap::new(72, 72).expect("initial backdrop");
    pixmap.fill(tiny_skia::Color::WHITE);
    renderer.knockout_resources.canvas(pixmap.data().len());
    renderer.knockout_resources.surface_work = MAX_SURFACE_WORK - 2 * (shows * 72 * 72) as u64;
    let fallback = renderer
        .execute_knockout_group(
            &mut pixmap,
            tiny_skia::Transform::identity(),
            &operators,
            &doc,
            0,
            &resources,
            initial,
        )
        .expect("group renders");
    (fallback.is_none(), renderer.paint_operator_calls)
}

#[test]
fn invisible_text_stays_native_where_its_plan_and_floor_fit() {
    let shows = 40;
    let invisible = format!("BT /F1 8 Tf 3 Tr {} ET", "(M) Tj ".repeat(shows));
    assert_eq!(
        native_with_room_for_floor(&invisible, shows, Some(0)),
        (true, shows),
        "mode 3 charges the paint only, so its plan and floor of {shows} surfaces each fit"
    );
    let visible = format!("BT /F1 8 Tf 0 Tr {} ET", "(M) Tj ".repeat(shows));
    assert!(
        !native_with_room_for_floor(&visible, shows, Some(0)).0,
        "visible text also charges its coverage raster, which the same room does not hold"
    );
}

#[test]
fn a_knockout_form_with_invisible_text_stays_native_where_its_plan_and_floor_fit() {
    let shows = 40;
    let invisible = format!("BT /F1 8 Tf 3 Tr {} ET", "(M) Tj ".repeat(shows));
    assert_eq!(native_with_room_for_floor(&invisible, shows, None), (true, shows));
    let doc = document("/Fm Do", true, Some(&invisible));
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer
        .render_page(&doc, 0)
        .expect("knockout form with invisible text renders");
    assert_eq!(
        renderer.paint_operator_calls,
        shows + 1,
        "the Do and every show execute once"
    );
}

fn path_paints(content: &str, hidden_layer: bool) -> usize {
    let doc = document(content, true, None);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let resources = doc.get_page_resources(0).expect("page resources");
    if hidden_layer {
        renderer.excluded_layers_snapshot = Some(Arc::new(std::iter::once("L".to_string()).collect()));
    }
    let operators = crate::content::parse_content_stream(content.as_bytes()).expect("real path operators");
    renderer.certain_path_paints(&operators, &doc, &resources)
}

const HUGE: &str = "300000000000000000000000000000000000000.0";
const BEYOND_F32: &str = "1000000000000000000000000000000000000000.0";

#[test]
fn only_a_path_paint_that_certainly_builds_a_path_raises_the_floor() {
    let overflowing_span = format!("-{HUGE} 0 m {HUGE} 0 l f");
    let overflowing_curve = format!("-{HUGE} 0 m 0 0 {HUGE} 0 {HUGE} 0 c f");
    let overflowing_y = format!("-{HUGE} 0 m 0 0 {HUGE} 0 y f");
    let overflowing_v = format!("0 0 m 1 1 l 0 0 {HUGE} 0 v 0 0 -{HUGE} 0 v f");
    let overflowing_from_origin = format!("{HUGE} 0 l -{HUGE} 0 l f");
    let fitting_span = format!("0 0 m {HUGE} 0 l f");
    let infinite_line = format!("0 0 m {BEYOND_F32} 5 l f");
    let infinite_after_rect = format!("0 0 10 10 re {BEYOND_F32} 5 l f");
    let infinite_start = format!("{BEYOND_F32} 0 m 5 5 l f");
    let overflowing_rect = format!("{HUGE} 0 {HUGE} 10 re f");
    let cases: Vec<(&str, usize)> = vec![
        ("f", 0),
        ("n f", 0),
        ("0 0 10 10 re n f", 0),
        ("0 0 10 10 re f", 1),
        ("0 0 10 10 re f f", 1),
        ("0 0 10 10 re B 5 5 m 9 9 l S", 2),
        ("0 0 10 10 re s", 1),
        ("5 5 m 9 9 l s", 1),
        ("0 0 0 10 re f", 1),
        ("0 0 100 0 re S", 1),
        (&overflowing_rect, 0),
        ("/OC /L BDC 0 0 10 10 re f EMC", 0),
        ("/OC /L BDC EMC 0 0 10 10 re f", 1),
        ("/OC /L BDC /Span BMC EMC 0 0 10 10 re f EMC 0 0 10 10 re f", 1),
        ("/Span BMC 0 0 10 10 re f EMC", 1),
        ("/Span /L BDC 0 0 10 10 re f EMC", 1),
        ("0 0 10 10 re /OC /L BDC f EMC f", 0),
        ("5 5 m f", 0),
        ("0 0 m 5 5 l f 0 0 m 5 5 l f", 2),
        ("0 0 m 1 1 2 2 3 3 c f", 1),
        ("0 0 m 1 1 3 3 y f", 1),
        ("1 1 2 2 3 3 c f", 1),
        ("5 5 l f", 1),
        (&fitting_span, 1),
        (&overflowing_span, 0),
        (&overflowing_curve, 0),
        (&overflowing_y, 0),
        (&overflowing_v, 0),
        (&overflowing_from_origin, 0),
        (&infinite_line, 0),
        (&infinite_after_rect, 0),
        (&infinite_start, 0),
    ];
    for (content, expected) in cases {
        assert_eq!(path_paints(content, true), expected, "{content}");
    }
}

#[test]
fn the_floor_counts_a_paint_only_where_the_renderers_path_builder_builds_a_path() {
    let stale_then_line = format!("-{HUGE} 0 m {HUGE} 0 m 5 5 l f");
    let stale_then_rect = format!("-{HUGE} 0 m {HUGE} 0 m 0 0 10 10 re f");
    let stale_after_close = format!("-{HUGE} 0 m h {HUGE} 0 m 5 5 l f");
    let stale_before_close_stroke = format!("-{HUGE} 0 m {HUGE} 0 m 5 5 l s");
    let two_moves_only = format!("-{HUGE} 0 m {HUGE} 0 m f");
    let cases: Vec<(&str, usize)> = vec![
        (&stale_then_line, 1),
        (&stale_then_rect, 1),
        (&stale_before_close_stroke, 1),
        (&stale_after_close, 0),
        (&two_moves_only, 0),
        ("0 0 m 0 0 m f", 0),
        ("0 0 m h f", 1),
        ("0 0 m s", 1),
        ("0 0 m b", 1),
        ("0 0 m b*", 1),
        ("0 0 m S", 0),
        ("0 0 m B", 0),
        ("0 0 m 1 1 5 5 v f", 1),
        ("1 1 5 5 v f", 0),
        ("0 0 -10 -10 re f", 1),
    ];
    for (content, expected) in cases {
        assert_eq!(path_paints(content, true), expected, "{content}");
    }
}

#[test]
fn stale_move_path_paints_that_cannot_fit_degrade_before_painting() {
    let paints = 40;
    let body = format!("-{HUGE} 0 m {HUGE} 0 m 5 5 l f ").repeat(paints);
    let room = 2 * paints - 10;
    assert_eq!(
        run_with_room_for(&format!("0 g {body}"), room, false),
        (false, paints),
        "the floor counts each stale-move paint, refuses the plan, and nothing is painted twice"
    );
}

#[test]
fn a_path_paint_in_a_visible_layer_raises_the_floor_and_one_in_a_hidden_layer_does_not() {
    let cases: [(&str, usize, usize); 6] = [
        ("/OC /L BDC 0 0 10 10 re f EMC", 1, 0),
        ("/OC /L BDC 0 0 10 10 re f EMC 0 0 10 10 re f", 2, 1),
        ("/OC /Gone BDC 0 0 10 10 re f EMC", 1, 1),
        ("/OC /L BDC /OC /Gone BDC 0 0 10 10 re f EMC EMC", 1, 0),
        ("/OC /Gone BDC /OC /L BDC 0 0 10 10 re f EMC EMC", 1, 0),
        ("/OC /Gone BDC 0 0 10 10 re EMC f", 1, 1),
    ];
    for (content, visible, hidden) in cases {
        assert_eq!(path_paints(content, false), visible, "visible: {content}");
        assert_eq!(path_paints(content, true), hidden, "hidden: {content}");
    }
}

fn native_with_plan_and_certain_floor(content: &str, certain: usize) -> (bool, usize) {
    let operators = crate::content::parse_content_stream(content.as_bytes()).expect("real operators");
    let paints = operators
        .iter()
        .filter(|operator| crate::rendering::page_renderer::is_paint_operator(operator))
        .count();
    run_with_room_for(content, paints + certain, true)
}

fn run_with_room_for(content: &str, surfaces: usize, hidden_layer: bool) -> (bool, usize) {
    let doc = document(content, true, None);
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let resources = doc.get_page_resources(0).expect("page resources");
    renderer.load_resources(&doc, &resources).expect("load resources");
    if hidden_layer {
        renderer.excluded_layers_snapshot = Some(Arc::new(std::iter::once("L".to_string()).collect()));
    }
    let operators = crate::content::parse_content_stream(content.as_bytes()).expect("real operators");
    let mut pixmap = tiny_skia::Pixmap::new(72, 72).expect("initial backdrop");
    pixmap.fill(tiny_skia::Color::WHITE);
    renderer.knockout_resources.canvas(pixmap.data().len());
    renderer.knockout_resources.surface_work = MAX_SURFACE_WORK - (surfaces * 72 * 72) as u64;
    let fallback = renderer
        .execute_knockout_group(
            &mut pixmap,
            tiny_skia::Transform::identity(),
            &operators,
            &doc,
            0,
            &resources,
            None,
        )
        .expect("group renders");
    (fallback.is_none(), renderer.paint_operator_calls)
}

#[test]
fn paints_that_charge_no_path_work_stay_native_where_plan_and_floor_fit() {
    let bare = format!("/Half gs 0 g 10 10 40 40 re f 20 20 40 40 re f {}", "f ".repeat(40));
    assert_eq!(native_with_plan_and_certain_floor(&bare, 2), (true, 42));
    let hidden = format!(
        "/Half gs 0 g 10 10 40 40 re f 20 20 40 40 re f /OC /L BDC {}EMC",
        "0 0 10 10 re f ".repeat(40)
    );
    assert_eq!(native_with_plan_and_certain_floor(&hidden, 2), (true, 42));
    let visible = format!("0 g {}", "0 0 10 10 re f ".repeat(42));
    assert!(
        !native_with_plan_and_certain_floor(&visible, 2).0,
        "visible path paints charge a surface each, which room for two does not hold"
    );
}

#[test]
fn a_visible_layer_of_path_paints_that_cannot_fit_degrades_before_painting() {
    let paints = 40;
    let body = "0 0 10 10 re f ".repeat(paints);
    let room = 2 * paints - 10;
    assert_eq!(
        run_with_room_for(&format!("0 g /OC /V BDC {body}EMC"), room, false),
        (false, paints),
        "the floor counts the layer's paints, refuses the plan, and nothing is painted twice"
    );
    assert_eq!(
        run_with_room_for(&format!("0 g /OC /L BDC {body}EMC"), room, true),
        (true, paints),
        "a hidden layer charges nothing, so the same room holds it"
    );
}
