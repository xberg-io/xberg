use super::*;
use crate::document::PdfDocument;
use crate::rendering::{PageRenderer, RenderOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};

fn document(content: &str, knockout: bool, form: Option<&str>) -> PdfDocument {
    sized_document(content, knockout, form, "72 72")
}

fn sized_document(content: &str, knockout: bool, form: Option<&str>, size: &str) -> PdfDocument {
    let group = format!("/Group << /S /Transparency /I true /K {knockout} >>");
    let resources = "/ExtGState << /Half << /ca 0.5 >> >> /XObject << /Fm 5 0 R >> \
        /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {size}] /Contents 4 0 R /Resources << {resources} >> {} >>",
            if form.is_none() { group.as_str() } else { "" }
        ),
        stream("", content),
        stream(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 {size}] /Resources << {resources} >> {group}"),
            form.unwrap_or(""),
        ),
    ];
    pdf(&objects)
}

fn stream(dict: &str, data: &str) -> String {
    format!("<< {dict} /Length {} >>\nstream\n{data}\nendstream", data.len())
}

fn pdf(objects: &[String]) -> PdfDocument {
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
        let actual = tracing::subscriber::with_default(subscriber, || render(&knockout))
            .expect("oversized legitimate group degrades instead of failing the page");
        assert_eq!(
            actual.data, expected.data,
            "fallback follows ordinary paint order at {dpi} dpi"
        );
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
            .knockout(1, 0, 0, 0, 1)
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
    resources.surface_work = MAX_SURFACE_WORK - 10;
    let admitted = resources
        .knockout(10, 0, 0, 0, 1)
        .expect("exact threshold")
        .expect("native");
    assert_eq!(resources.surface_work, MAX_SURFACE_WORK);
    drop(admitted);
    assert!(resources.knockout(1, 0, 0, 0, 1).expect("graceful excess").is_none());
    let fallback = resources.fallback().expect("work-only fallback");
    resources
        .raster(usize::MAX)
        .expect("normal raster is not charged as knockout work");
    assert_eq!(resources.surface_work, MAX_SURFACE_WORK, "no refund or reset");
    drop(fallback);
    resources.raster(1).expect("ordinary siblings remain renderable");
    assert_eq!(
        resources.surface_work, MAX_SURFACE_WORK,
        "work outside a native knockout pass is not charged"
    );
    resources.reset(Some(1));
    assert!(resources.knockout(1, 1, 0, 0, 1).is_err(), "failed memory admission");
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

struct Form<'a> {
    name: &'a str,
    group: Option<bool>,
    content: String,
}

fn form(name: &str, group: Option<bool>, content: impl Into<String>) -> Form<'_> {
    Form {
        name,
        group,
        content: content.into(),
    }
}

/// A page whose resources name every form; a form called `Mask` is the soft mask of `/SM gs`.
fn forms_document(size: &str, page: &str, forms: &[Form<'_>]) -> PdfDocument {
    let object = |name: &str| forms.iter().position(|form| form.name == name).map(|index| index + 5);
    let xobjects: String = forms
        .iter()
        .map(|form| format!("/{} {} 0 R ", form.name, object(form.name).unwrap_or(0)))
        .collect();
    let mask = object("Mask")
        .map(|mask| format!("/SM << /SMask << /S /Luminosity /G {mask} 0 R >> >>"))
        .unwrap_or_default();
    let resources = format!(
        "/ExtGState << /Half << /ca 0.5 >> {mask} >> /XObject << {xobjects}>> \
         /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>"
    );
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {size}] /Contents 4 0 R /Resources << {resources} >> >>"),
        stream("", page),
    ];
    for form in forms {
        let group = form
            .group
            .map(|knockout| format!("/Group << /S /Transparency /I true /K {knockout} >>"))
            .unwrap_or_default();
        objects.push(stream(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 {size}] /Resources << {resources} >> {group}"),
            &form.content,
        ));
    }
    pdf(&objects)
}

#[test]
fn ordinary_group_paints_spend_no_knockout_work_but_count_operations() {
    let mut resources = KnockoutResources::default();
    let native = resources
        .knockout(10, 40, 0, 0, 1)
        .expect("small knockout admission")
        .expect("native frame");
    resources
        .raster(10)
        .expect("one real surface pass under the native frame");
    drop(native);
    assert_eq!(resources.surface_work, 20, "the native pass is charged");
    let ordinary = resources.group(40, Some(0)).expect("ordinary sibling workspace");
    for _ in 0..3 {
        resources
            .paint(usize::MAX)
            .expect("an ordinary group has no work ceiling");
    }
    resources.raster(usize::MAX).expect("ordinary raster is not charged");
    assert_eq!(
        resources.surface_work, 20,
        "ordinary paints leave the knockout ledger alone"
    );
    assert_eq!(resources.paint_operations, 3, "ordinary paints are counted");
    resources.paint_operations = MAX_PAINT_OPERATIONS;
    assert!(
        resources.paint(0).is_err(),
        "the paint operation cap holds in an ordinary group"
    );
    drop(ordinary);
}

#[test]
fn native_knockout_keeps_its_operation_cap_and_a_degraded_child_charges_nothing() {
    let mut resources = KnockoutResources::default();
    let native = resources
        .knockout(10, 0, 0, 0, 1)
        .expect("small knockout admission")
        .expect("native frame");
    let child = resources.fallback().expect("degraded child under the native frame");
    resources
        .raster(usize::MAX)
        .expect("a degraded child renders without knockout work");
    assert_eq!(resources.surface_work, 10, "only the planned native work is charged");
    drop(child);
    resources.paint_operations = MAX_PAINT_OPERATIONS;
    assert!(
        resources.paint(0).is_err(),
        "the paint operation cap holds in a native pass"
    );
    drop(native);
}

#[test]
fn nested_ordinary_form_fan_out_ends_at_the_paint_operation_cap() {
    let doc = forms_document(
        "2 2",
        "/F1 Do",
        &[
            form("F1", Some(false), "/F2 Do ".repeat(1000)),
            form("F2", Some(false), "f ".repeat(1000)),
        ],
    );
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let Err(error) = renderer.render_page(&doc, 0) else {
        panic!("a million nested ordinary paints end in a bounded error");
    };
    assert!(
        error.to_string().contains("paint operations"),
        "the paint operation cap ends the fan-out: {error}"
    );
    assert_eq!(
        renderer.knockout_resources.surface_work, 0,
        "ordinary groups charge no work"
    );
}

#[test]
fn text_runs_charge_the_operator_loop_only_in_a_knockout_group() {
    let form = "BT /F1 4 Tf 10 10 Td (a) Tj (a) Tj ET";
    let charges = |knockout: bool| {
        let doc = document("/Fm Do", knockout, Some(form));
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        renderer.render_page(&doc, 0).expect("small group renders");
        let resources = &renderer.knockout_resources;
        (resources.paint_operations, resources.surface_work)
    };
    let page = 72 * 72;
    assert_eq!(
        charges(true),
        (2, 6 * page),
        "two planned paints, two coverage rasters and two operator loop charges of one page each"
    );
    assert_eq!(
        charges(false),
        (2, 0),
        "an ordinary group counts its paints and charges no work"
    );
}

#[test]
fn knockout_and_ordinary_groups_nested_either_way_charge_only_the_native_pass() {
    let fill = "0 g 0 0 10 10 re f ";
    let charges = |outer: bool| {
        let doc = forms_document(
            "72 72",
            "/Outer Do",
            &[
                form("Outer", Some(outer), format!("{fill}/Inner Do")),
                form("Inner", Some(!outer), fill.repeat(2)),
            ],
        );
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        renderer.render_page(&doc, 0).expect("nested groups render");
        let resources = &renderer.knockout_resources;
        assert_eq!(resources.degradations(), 0, "small groups stay native");
        (resources.paint_operations, resources.surface_work)
    };
    let page = 72 * 72;
    assert_eq!(
        charges(false),
        (4, 4 * page),
        "a knockout form inside an ordinary group: only its plan and its two fills are charged, a page each"
    );
    assert_eq!(
        charges(true),
        (4, 6 * page),
        "an ordinary form inside a knockout group: the whole native pass is charged"
    );
}

#[test]
fn heavy_ordinary_work_before_a_knockout_group_keeps_native_knockout() {
    let runs = 1000;
    let tiny = "60 60 0.1 0.1 re f ".repeat(78);
    let doc = forms_document(
        "612 792",
        "/Ordinary Do /Knockout Do",
        &[
            form(
                "Ordinary",
                Some(false),
                format!("BT /F1 4 Tf 10 400 Td {} ET", "(a) Tj ".repeat(runs)),
            ),
            form(
                "Knockout",
                Some(true),
                format!("/Half gs 1 0 0 rg 0 0 612 792 re f {tiny}0 0 1 rg 0 0 612 792 re f"),
            ),
        ],
    );
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let image = renderer.render_page(&doc, 0).expect("both groups render");
    assert_eq!(
        renderer.knockout_resources.degradations(),
        0,
        "ordinary work leaves the knockout allowance alone"
    );
    let page = 612 * 792;
    assert_eq!(
        renderer.knockout_resources.surface_work,
        2 * 80 * page,
        "only the knockout group is charged: its plan and its 80 fills, a page each"
    );
    let pixel = &image.data[..4];
    assert!(
        pixel[0].abs_diff(127) <= 1 && pixel[1].abs_diff(127) <= 1 && pixel[2] == 255,
        "the last knockout paint replaces the first: {pixel:?}"
    );
}

#[test]
fn ordinary_group_with_a_thousand_text_runs_renders_at_300_dpi() {
    let runs = 1000;
    let form = format!("BT /F1 4 Tf 10 400 Td {} ET 0 g 0 0 20 20 re f", "(a) Tj ".repeat(runs));
    let doc = sized_document("/Fm Do", false, Some(&form), "612 792");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(300).as_raw());
    let image = renderer
        .render_page(&doc, 0)
        .expect("an ordinary transparency group renders at any size");
    assert_eq!((image.width, image.height), (2550, 3300));
    assert_eq!(
        renderer.paint_operator_calls,
        runs + 2,
        "the Do, every text run and the fill ran"
    );
    let bottom_left = (image.height as usize - 1) * image.width as usize * 4;
    assert_eq!(
        &image.data[bottom_left..bottom_left + 4],
        &[0, 0, 0, 255],
        "the fill after the text runs reached the page"
    );
}

#[test]
fn form_without_a_group_under_a_soft_mask_renders_200_text_runs_at_300_dpi() {
    let runs = 200;
    let doc = forms_document(
        "612 792",
        "/SM gs /Fm Do",
        &[
            form(
                "Fm",
                None,
                format!("BT /F1 4 Tf 10 400 Td {} ET 0 g 0 0 20 20 re f", "(a) Tj ".repeat(runs)),
            ),
            form("Mask", Some(false), "1 g 0 0 612 792 re f"),
        ],
    );
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(300).as_raw());
    let image = renderer
        .render_page(&doc, 0)
        .expect("a masked form renders at any size");
    assert_eq!(
        renderer.knockout_resources.surface_work, 0,
        "a masked form charges no work"
    );
    let bottom_left = (image.height as usize - 1) * image.width as usize * 4;
    assert_eq!(
        &image.data[bottom_left..bottom_left + 4],
        &[0, 0, 0, 255],
        "the fill after the text runs reached the page through a white mask"
    );
}
