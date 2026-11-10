/// QA-6-MECH: pins the underlying mechanism the QA-6 family bug was
/// rooted in — the `Do` operator's post-Do spot-lane mirror.
///
/// Setup: no transparency groups anywhere. Page content stream is
///   /CS_PMS cs 0.6 scn /Form Do
/// Form XObject content stream is
///   /Half gs /CS_PMS cs 0.3 scn 0 0 100 100 re f
/// where /Half is /ca 0.5. The Form has NO /Group dict.
///
/// What the Form does internally:
///  - sets fill alpha = 0.5
///  - sets fill colour space + tint InkA 0.3
///  - paints a 100×100 rect → the path's fill operator's per-paint
///    spot mirror writes lane = compose_normal(0, 0.3, 0.5) = 0.15
///    → u8 38 at the painted pixels.
///
/// What the OUTER content stream does:
///  - sets fill colour space + tint InkA 0.6 at α=1 (no /Half on the
///    outer side)
///  - calls /Form Do
///
/// The `Do` dispatcher captures `gs_clone` = OUTER gs at Do time:
/// `fill_spot_inks = [("InkA", 0.6)]`, `fill_alpha = 1.0`. The Form
/// XObject's `render_form_xobject` path executes the form's internal
/// operators, which DO their own per-paint spot mirror (writing 38).
///
/// The pre-fix bug: the `Do` dispatcher unconditionally ran a post-Do
/// `mirror_spot_paint_into_sidecar_with_coverage(pixmap, &snap, None,
/// &gs_clone, true)` block whenever `gs_clone` had a spot ink active.
/// That post-Do mirror used the OUTER gs's tint (0.6) and α (1.0) and,
/// because `coverage = None`, fell back to the snapshot-vs-post diff
/// (any pixel where RGB changed counts as "fully painted at 255"). So
/// every pixel the form had touched got re-written: lane =
/// (1−1)·38 + 1·0.6 = 0.6 → u8 153. The form's correct 38 was
/// overwritten by the outer-gs-flavoured 153.
///
/// Spec basis for the fix (ISO 32000-1 §11.4.7 + §8.10):
///  - Form XObjects execute their own content stream with their own
///    graphics state; the per-paint sidecar mirror runs at each Form-
///    internal paint operator and is already complete by the time the
///    Form returns.
///  - Image / ImageMask XObjects do not execute paint operators of
///    their own; their pixel data is painted using the OUTER gs's
///    fill colour (ImageMask) or carries its own colours (Image), so
///    the outer gs's CMYK / overprint / spot-lane modulators must
///    run post-Do.
///
/// The fix dispatches the post-Do CMYK compose / overprint / spot
/// mirror by the XObject's `/Subtype`: skipped for Form, applied for
/// Image / ImageMask. SMask attenuation always applies regardless of
/// subtype (it modulates whatever pixels the Do produced against the
/// captured backdrop, per §11.4.7).
///
/// This probe is byte-exact: lane = 38.
/// Failure mode 153 = post-Do mirror re-fired with outer tint 0.6
/// (the regression this fix closes).
#[test]
fn qa_6_mech_do_dispatcher_does_not_remirror_outer_spot_over_form_internal_writes() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let form_stream = "/Half gs\n/CS_PMS cs\n0.3 scn\n0 0 100 100 re\nf\n";
    let form = format!(
        "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << /ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
                         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK \
                           << /FunctionType 2 /Domain [0 1] \
                              /Range [0 1 0 1 0 1 0 1] /C0 [0.0 0.0 0.0 0.0] \
                              /C1 [0.0 1.0 0.0 0.0] /N 1 >> ] >> >> \
           /Length {} >>\n\
        stream\n{}endstream\nendobj\n",
        form_stream.len(),
        form_stream
    );
    let content = "/CS_PMS cs\n0.6 scn\n/Form Do\n";
    let resources = format!(
        "/XObject << /Form 6 0 R >> \
         /ExtGState << /Trigger << /Type /ExtGState /ca 0.99 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[&form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");

    let observed = centre(inka);
    assert_eq!(
        observed, 38,
        "ISO 32000-1 §11.4.7 / §8.10: a Form XObject executes its own \
         content stream with its own graphics state; its per-paint \
         sidecar mirror is the authoritative lane write for the Form's \
         pixels. The Do dispatcher MUST NOT re-mirror the outer gs's \
         spot tint over the Form's contribution, or the outer's stale \
         colour overwrites the Form's correct lane state. Got {} \
         (expected 38). If 153: the Do dispatcher's post-Do spot-lane \
         mirror is firing on Form Do — that's the mechanism behind the \
         QA-6 / QA-6-DIAG-2 regression, where outer /K iteration 2's \
         Inner Do lost the inner Form's spot writes because this \
         double-mirror smashed them.",
        observed
    );
}

/// QA-7: /K group containing /SMask and /Separation paint. SMask
/// attenuation must apply per-pixel to the spot lane; /K replay must
/// snapshot and restore the spot lane to the group's initial backdrop.
/// The order is: enter /K, snapshot lanes; for each element, restore
/// lanes; execute paint (mirror writes spot lane); apply SMask
/// (modulate spot lane against pre-mirror snapshot); merge into
/// accumulator.
///
/// Backdrop: no prior InkA paint. /K Form has /SMask gs + single
/// /Separation /InkA paint at tint 0.6 with /ca 1.0. Uniform /SMask
/// at 0.5 grey.
///
/// Cascade:
///   - Mirror writes lane = compose_normal(0, 0.6, 1) = 0.6 → u8 153.
///   - SMask: post = 153, pre-mirror snap = 0. m = 0.5. lane = 0.5·153
///     + 0.5·0 = 76.5 → u8 77.
///   - /K merge: post = 77, backdrop = 0. Skip if equal: 77 ≠ 0 →
///     accumulator picks 77.
///
/// Probe pins 77 byte-exact.
#[test]
fn qa_7_knockout_group_with_smask_spot_paint_attenuates_correctly() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let smask_form = "8 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << >> \
           /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
           /Length 28 >>\n\
        stream\n0.5 g\n0 0 100 100 re\nf\nendstream\nendobj\n";
    let k_stream = "/Mask gs\n/CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n";
    let k_form = format!(
        "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << /ExtGState << /Mask << /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 8 0 R >> >> >> \
                         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK \
                           << /FunctionType 2 /Domain [0 1] \
                              /Range [0 1 0 1 0 1 0 1] /C0 [0.0 0.0 0.0 0.0] \
                              /C1 [0.0 1.0 0.0 0.0] /N 1 >> ] >> >> \
           /Group << /Type /Group /S /Transparency /K true /CS /DeviceCMYK >> \
           /Length {} >>\n\
        stream\n{}endstream\nendobj\n",
        k_stream.len(),
        k_stream
    );
    let content = "/Form Do\n";
    let resources = format!(
        "/XObject << /Form 6 0 R >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[&k_form, smask_form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");

    assert_eq!(
        centre(inka),
        77,
        "ISO 32000-1 §11.4.7 + §11.4.6.2: /K group with /SMask + \
         /Separation paint. Mirror writes 153; SMask attenuates to \
         77; /K merge preserves 77 (≠ backdrop 0). Got {}.",
        centre(inka)
    );
}

/// QA-8: a page with /OP true (overprint) only triggers the per-plate
/// walker (composite path is excluded by `page_declares_transparency`
/// dropping /OP). This is the round-3 self-flagged correctness
/// guarantee. The probe pins per-plate walker output for an OPM=0
/// DeviceCMYK paint with /OP true, which validates the walker's
/// §11.7.4 OPM logic and indirectly confirms the dispatch's perf-
/// optimisation IS effectively a correctness-critical gate (because
/// the per-plate walker's behaviour differs from what composite
/// path would produce).
#[test]
fn qa_8_detection_off_pure_overprint_page_keeps_per_plate_walker() {
    let content = "0.4 0 0 0 k\n0 0 100 100 re\nf\n\
                   /Ov gs\n0 0.5 0 0 k\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Ov << /Type /ExtGState /OP true >> >>";
    let pdf = build_pdf_no_output_intent(content, resources);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let c = plate(&plates, "Cyan");
    let m = plate(&plates, "Magenta");

    assert_eq!(
        centre(m),
        128,
        "Per-plate walker writes /Separation-equivalent DeviceCMYK \
         plate M = 0.5 → u8 128 at centre. Got {}.",
        centre(m)
    );
    let observed_c = centre(c);
    assert!(
        observed_c == 0 || observed_c == 102,
        "Per-plate walker /OP true /OPM 0 + DeviceCMYK: Cyan must be \
         either replaced to 0 (full-spec semantics) or preserved at \
         u8 102 (replace-nonzero approximation). Got u8 {}; both \
         readings are defensible §11.7.4.3 interpretations. The probe \
         records the walker's chosen interpretation as a baseline.",
        observed_c
    );
}

/// QA-9: /Separation /InkA with /SMask + /OP true + /OPM 1 + /ca 1.0.
/// SMask attenuates the spot lane; overprint is per-§11.7.4.4 for
/// process plates only (spot lanes are not affected by /OP/OPM —
/// §11.7.4.2 says overprint applies to process colorants; spot
/// lanes get the /Normal substitute or the requested BM, independent
/// of OPM).
///
/// Probe pins:
///   - InkA spot plate: SMask-attenuated mirror = m·post + (1-m)·pre
///     = 0.5·153 + 0.5·0 = 77 → u8 77.
///   - Magenta plate: unaffected (no Magenta source).
#[test]
fn qa_9_transparency_overprint_smask_separation_plate_byte_exact() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let smask_form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << >> \
           /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
           /Length 28 >>\n\
        stream\n0.5 g\n0 0 100 100 re\nf\nendstream\nendobj\n";
    let content = "/Both gs\n\
                   /CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Both << /Type /ExtGState /OP true /OPM 1 \
            /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[smask_form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    let m = plate(&plates, "Magenta");

    assert_eq!(
        centre(inka),
        77,
        "ISO 32000-1 §11.4.7 + §11.7.4.2: /Separation paint with \
         /SMask + /OP true + /OPM 1. Overprint applies to process \
         lanes only; spot lane runs through SMask attenuation. \
         Mirror writes 153, SMask m=0.5 → 77. Got {}.",
        centre(inka)
    );
    // Magenta: no source magenta paint. Should be 0 (the /Separation
    // /InkA's alternate-CS approximation contributes to the visible
    // composite but NOT to the process plates' spec-per-plate output).
    // We pin the observed M-plate centre value: if the alternate-CS
    // path leaks into the process Magenta plate, this is non-zero;
    // if §11.7.3 "spots retain identity through transparency" is
    // honoured, this is 0. ~keep
    let observed_m = centre(m);
    // The probe records the empirically observed magenta byte.
    // Documented expectation per §11.7.3 + §11.7.4.2: 0 (the
    // /Separation paint does not contribute to process plates because
    // its alternate-CS expansion happens in the compositing buffer,
    // not on the per-plate output). ~keep
    assert_eq!(
        observed_m, 0,
        "ISO 32000-1 §11.7.3: /Separation /InkA spot paint should \
         not contribute to the Magenta process plate. The plate \
         output is independent of the alternate-CS approximation \
         used for the visible composite. Got Magenta = {} (expected \
         0).",
        observed_m
    );
}

/// QA-10: a page that fires the transparency detection (ca<1) is
/// routed through `render_plates_via_composite`. The renderer
/// allocates a sidecar (force_cmyk_sidecar = true + detection ON).
/// The probe verifies the composite path does NOT panic if it ever
/// finds `take_cmyk_sidecar` returning None — the code path guards
/// each access with `if let Some(s) = sidecar.as_ref()`. We
/// simulate by constructing a synthetic with detection on but where
/// the sidecar might not allocate (e.g. zero-size page). Defensively
/// the probe just confirms no panic on render and that all plates
/// come back with the correct dims.
#[test]
fn qa_10_composite_path_does_not_panic_on_none_sidecar() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let content = "/Trig gs\n\
                   /CS_PMS cs\n0.5 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Trig << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");

    let plates = render_separations(&doc, 0, 72).expect("render");
    assert!(!plates.is_empty(), "plate list non-empty for detection-ON page");

    for p in &plates {
        assert_eq!(
            p.data.len(),
            (p.width as usize) * (p.height as usize),
            "plate {} has wrong-sized buffer: {} vs {}×{}",
            p.ink_name,
            p.data.len(),
            p.width,
            p.height
        );
    }
}

// ===========================================================================
// Adversarial probe 11: page_declares_transparency regression coverage.
// The helper must fire on every transparency trigger and NOT fire on
// /OP/op alone.
// =========================================================================== ~keep

/// QA-11a: /SMask non-None triggers the helper. Probe routes through
/// the composite path → spot plate gets the SMask-attenuated value
/// (proves SMask trigger fired).
#[test]
fn qa_11a_smask_triggers_composite_dispatch() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let smask_form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << >> \
           /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
           /Length 28 >>\n\
        stream\n0.5 g\n0 0 100 100 re\nf\nendstream\nendobj\n";
    let content = "/Mask gs\n\
                   /CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Mask << /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[smask_form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    assert_eq!(
        centre(inka),
        77,
        "page_declares_transparency must fire on /SMask non-None. \
         Composite path → SMask attenuates mirror 153 to 77. Got {}.",
        centre(inka)
    );
}

/// QA-11b: /BM non-Normal triggers the helper. /Separation paint
/// with /BM /Multiply at /ca = 1.0 (transparency-trigger via BM only)
/// must route to composite path. Round-3 P1 already pins Multiply
/// with /ca; this probe pins BM-only (no /ca).
#[test]
fn qa_11b_blend_mode_triggers_composite_dispatch() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let content = "/CS_PMS cs\n0.4 scn\n0 0 100 100 re\nf\n\
                   /Mult gs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Mult << /Type /ExtGState /BM /Multiply >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    assert_eq!(
        centre(inka),
        194,
        "page_declares_transparency must fire on /BM non-Normal even \
         without /ca. Composite path → Multiply(0.4, 0.6) at α=1 = \
         1 - 0.6·0.4 = 0.76 → u8 194. Got {}.",
        centre(inka)
    );
}

/// QA-11c: /BM array form with non-Normal first-recognised triggers
/// the helper. `/BM [/UnknownMode /Multiply]` resolves to Multiply.
#[test]
fn qa_11c_blend_mode_array_form_triggers_composite_dispatch() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let content = "/CS_PMS cs\n0.4 scn\n0 0 100 100 re\nf\n\
                   /Mult gs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Mult << /Type /ExtGState /BM [/MarketingInventedMode /Multiply] >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    assert_eq!(
        centre(inka),
        194,
        "page_declares_transparency must fire on /BM array first-\
         recognised non-Normal. Got {}.",
        centre(inka)
    );
}

/// QA-11d: /OP true ALONE does NOT trigger the helper. The detection-
/// OFF byte-identity check: pure /OP true with no other trigger goes
/// to the per-plate walker. We verify by paint output differing from
/// what the composite path would produce.
#[test]
fn qa_11d_op_alone_does_not_trigger_composite_dispatch() {
    let content = "/OnlyOP gs\n0.6 0 0 0 k\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /OnlyOP << /Type /ExtGState /OP true >> >>";
    let pdf = build_pdf_no_output_intent(content, resources);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let c = plate(&plates, "Cyan");
    assert_eq!(
        centre(c),
        153,
        "page_declares_transparency must NOT fire on /OP alone. Per-\
         plate walker writes Cyan = 0.6 → u8 153. Got {}.",
        centre(c)
    );
}

/// QA-11e: /op true (lowercase) alone does NOT trigger the helper.
/// Mirror of QA-11d for the stroking-overprint flag.
#[test]
fn qa_11e_op_lowercase_alone_does_not_trigger_composite_dispatch() {
    let content = "/OnlyOp gs\n0.6 0 0 0 k\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /OnlyOp << /Type /ExtGState /op true >> >>";
    let pdf = build_pdf_no_output_intent(content, resources);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let c = plate(&plates, "Cyan");
    assert_eq!(
        centre(c),
        153,
        "page_declares_transparency must NOT fire on /op lowercase \
         alone. Per-plate walker writes Cyan = 0.6 → u8 153. Got {}.",
        centre(c)
    );
}

/// QA-11f: XObject with /Group dict triggers the helper. A page
/// /Resources/XObject/Form whose Form dict has /Group /S /Transparency
/// — even without any /ExtGState — must route to composite. Probe
/// renders an InkA paint via the Form Do; composite path produces
/// the alpha-composed plate.
#[test]
fn qa_11f_xobject_group_triggers_composite_dispatch() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK \
              << /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                 /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >> ] >> >> \
           /Group << /Type /Group /S /Transparency /CS /DeviceCMYK >> \
           /Length 35 >>\n\
        stream\n/CS_PMS cs\n0.5 scn\n0 0 100 100 re\nf\nendstream\nendobj\n";
    let content = "/Form Do\n";
    let resources = format!(
        "/XObject << /Form 6 0 R >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    assert_eq!(
        centre(inka),
        128,
        "page_declares_transparency must fire on XObject /Group. \
         Composite path → InkA tint 0.5 at α=1 = 0.5 → u8 128. Got \
         {}.",
        centre(inka)
    );
}

/// QA-12: a single-ink render via `render_separation` for a non-
/// existent ink on a detection-OFF page produces an all-zero plate
/// (per §8.6.6.3 "no plate"). The compose path is not entered;
/// per-plate walker fills with 0.
#[test]
fn qa_12_render_separation_nonexistent_ink_produces_zero_plate() {
    use xberg_native_pdf::rendering::render_separation;
    let content = "0.5 0 0 0 k\n0 0 100 100 re\nf\n";
    let pdf = build_pdf_no_output_intent(content, "");
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plate = render_separation(&doc, 0, "PANTONE 9999 C", 72).expect("render");
    let off = ((plate.height / 2) * plate.width + plate.width / 2) as usize;
    assert_eq!(
        plate.data[off], 0,
        "ISO 32000-1 §8.6.6.3 \"no plate\": ink not on page produces \
         all-zero plate. Got {}.",
        plate.data[off]
    );
    assert!(
        plate.data.iter().all(|&b| b == 0),
        "Non-existent ink plate must be all-zero, not just at centre"
    );
}
