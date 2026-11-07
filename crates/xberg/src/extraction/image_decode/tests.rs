use std::collections::BTreeMap;
use std::path::Path;

use image::ImageEncoder;

use super::*;

fn grayscale_png(width: u32, height: u32) -> Vec<u8> {
    let pixels = vec![127_u8; usize::try_from(u64::from(width) * u64::from(height)).unwrap()];
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&pixels, width, height, image::ExtendedColorType::L8)
        .expect("encode grayscale PNG");
    bytes
}

fn decode_for_encode_under_test(
    bytes: &[u8],
    limits: &SecurityLimits,
    additional_bytes_per_pixel: u64,
) -> Result<image::DynamicImage> {
    let image = decode_standard_image_with_security_limits(bytes, limits)?;
    validate_dynamic_image_additional_live_bytes(&image, limits, additional_bytes_per_pixel, 0)?;
    Ok(image)
}

#[test]
fn encode_decode_rejects_peak_when_output_buffer_exceeds_budget() {
    let bytes = grayscale_png(10, 10);
    let limits = SecurityLimits {
        max_content_size: 399,
        ..Default::default()
    };

    let error = decode_for_encode_under_test(&bytes, &limits, 3)
        .expect_err("100-byte luma plus 300-byte encoded-output budget must exceed 399 bytes");

    assert!(matches!(error, XbergError::Validation { .. }));
}

#[test]
fn rgb_decode_rejects_peak_when_source_alone_fits() {
    let bytes = grayscale_png(10, 10);
    let limits = SecurityLimits {
        max_content_size: 399,
        ..Default::default()
    };

    let error = decode_standard_rgb8_with_security_limits(&bytes, &limits)
        .expect_err("100-byte luma plus 300-byte RGB conversion must exceed 399 bytes");

    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("live image-processing bytes"));
}

#[test]
fn rgb_decode_accepts_peak_at_exact_budget() {
    let bytes = grayscale_png(10, 10);
    let exact_peak = bytes.len() + 400;
    let limits = SecurityLimits {
        max_content_size: exact_peak,
        ..Default::default()
    };

    let rgb =
        decode_standard_rgb8_with_security_limits(&bytes, &limits).expect("the exact source-plus-RGB peak must fit");

    assert_eq!(rgb.dimensions(), (10, 10));
    assert_eq!(rgb.as_raw().len(), 300);
}

#[test]
fn rgb_decode_counts_still_live_encoded_input() {
    let bytes = grayscale_png(10, 10);
    let limits = SecurityLimits {
        max_content_size: bytes.len() + 399,
        ..Default::default()
    };

    let error = decode_standard_rgb8_with_security_limits(&bytes, &limits)
        .expect_err("encoded input must remain in the live peak during pixel conversion");

    assert!(matches!(error, XbergError::Validation { .. }));
}

#[test]
fn luma_decode_rejects_peak_when_rgba_source_alone_fits() {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[0_u8; 400], 10, 10, image::ExtendedColorType::Rgba8)
        .expect("encode RGBA PNG");
    let limits = SecurityLimits {
        max_content_size: 499,
        ..Default::default()
    };

    let error = decode_standard_luma8_with_security_limits(&bytes, &limits)
        .expect_err("400-byte RGBA plus 100-byte luma conversion must exceed 499 bytes");

    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("live image-processing bytes"));
}

fn budget_of(max_content_size: usize) -> ImageDecodeBudget {
    ImageDecodeBudget::from_security_limits(&SecurityLimits {
        max_content_size,
        ..Default::default()
    })
}

#[test]
fn budget_admits_a_pixel_count_over_budget_when_the_byte_count_fits() {
    // GH#1761 crossing point: the budget is a number of BYTES, so a
    // 10_000-pixel image whose decoded form is 500 bytes is within a
    // 1_000-byte budget. Comparing the pixel count against it rejected
    // this. Every caller today also passes a byte count >= the pixel
    // count, so only a test at the crossing point can see the unit
    // mismatch. ~keep
    budget_of(1_000)
        .validate(100, 100, 500)
        .expect("10_000 pixels decoding to 500 bytes fits a 1_000-byte budget");
}

#[test]
fn budget_still_rejects_a_byte_count_over_budget() {
    let error = budget_of(1_000)
        .validate(10, 10, 1_001)
        .expect_err("1_001 bytes must not fit a 1_000-byte budget");

    assert!(matches!(error, XbergError::Validation { .. }));
    assert!(error.to_string().contains("live image-processing bytes"));
}

#[test]
fn budget_accepts_a_byte_count_exactly_at_the_budget() {
    budget_of(1_000)
        .validate(10, 10, 1_000)
        .expect("a byte count exactly at the budget must fit");
}

#[test]
fn budget_rejects_a_zero_dimension() {
    // `validate` doubles as the zero-dimension guard; a zero extent is
    // malformed input regardless of how few bytes it claims. ~keep
    assert!(budget_of(1_000).validate(0, 10, 1).is_err());
    assert!(budget_of(1_000).validate(10, 0, 1).is_err());
    assert!(budget_of(1_000).validate(0, 0, 0).is_err());
}

#[test]
fn budget_handles_the_largest_representable_dimension_product() {
    // u32::MAX squared is 18_446_744_065_119_617_025, just under
    // u64::MAX, so widening both extents to u64 makes the product
    // unrepresentable-by-construction impossible. The `checked_mul` in
    // `validate` is therefore a backstop for a future widening of the
    // dimension types, not a branch reachable from u32 input — this test
    // pins the arithmetic that makes it unreachable, and shows such an
    // image is still refused on its byte count. ~keep
    assert!(u64::from(u32::MAX).checked_mul(u64::from(u32::MAX)).is_some());

    let error = budget_of(1_000)
        .validate(u32::MAX, u32::MAX, u64::MAX)
        .expect_err("a maximal image's byte count must exceed the budget");

    assert!(matches!(error, XbergError::Validation { .. }));
}

#[test]
fn source_audit_detects_aliases_multiline_calls_and_decoder_families() {
    let source = r#"
        use image::load_from_memory as decode_pixels;
        fn decode_all(bytes: &[u8], cursor: std::io::Cursor<&[u8]>) {
            let _ = image::
                load_from_memory(bytes);
            let _ = decode_pixels(bytes);
            let _ = image::codecs::png::PngDecoder::new(cursor);
            let _ = image::codecs::bmp::BmpDecoder::new_without_file_header(cursor);
            let _ = tiff::decoder::Decoder::new(cursor);
            let _ = sceptre::Image::from_bytes(bytes);
            let renamed_primary = context.primary_image_handle().unwrap();
            let _ = lib.decode(&renamed_primary, color_space, None);
        }
    "#;

    let calls = direct_decoder_calls(source);

    assert_eq!(calls.len(), 7, "every direct decoder family must be denied");
}

#[test]
fn source_audit_recognizes_image_reader_open() {
    let source = r#"
        fn unchecked(path: &std::path::Path) {
            let _ = image::ImageReader::open(path);
        }
    "#;

    let calls = direct_decoder_calls(source);

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "image::ImageReader::open");
}

#[test]
fn source_audit_ignores_comments_and_string_literals() {
    let source = r#"
        fn messages() {
            // image::load_from_memory(bytes)
            let message = "tiff::decoder::Decoder::new(cursor)";
        }
    "#;

    assert!(direct_decoder_calls(source).is_empty());
}

#[test]
fn source_audit_requires_guard_in_same_approved_function() {
    let source = r#"
        fn guarded_elsewhere(budget: Budget) {
            budget.validate(10, 10, 300).unwrap();
        }
        fn decode_standard_image(bytes: &[u8]) {
            let _ = image::ImageReader::new(std::io::Cursor::new(bytes));
        }
    "#;
    let calls = direct_decoder_calls(source);

    assert_eq!(calls.len(), 1);
    assert!(!calls[0].guards.contains("method::validate"));
}

#[test]
fn production_audit_fires_when_guard_is_removed_or_moved() {
    let removed = r#"
        fn decode_standard_image(bytes: &[u8]) {
            let _ = image::ImageReader::new(std::io::Cursor::new(bytes));
        }
    "#;
    let moved = r#"
        fn sibling(bytes: &[u8], limits: Limits) { let _ = probe_standard_image(bytes, limits, None); }
        fn decode_standard_image(bytes: &[u8]) {
            let _ = image::ImageReader::new(std::io::Cursor::new(bytes));
        }
    "#;
    let moved_after_allocation = r#"
        fn decode_standard_image(bytes: &[u8], limits: Limits) {
            let _ = image::load_from_memory(bytes);
            let _ = probe_standard_image(bytes, limits, None);
        }
    "#;
    let guarded = r#"
        fn decode_standard_image(bytes: &[u8], limits: Limits) {
            let _ = probe_standard_image(bytes, limits, None);
            let _ = image::ImageReader::new(std::io::Cursor::new(bytes));
        }
    "#;

    assert_eq!(decoder_audit_violations("extraction/image_decode.rs", removed).len(), 1);
    assert_eq!(decoder_audit_violations("extraction/image_decode.rs", moved).len(), 1);
    assert_eq!(
        decoder_audit_violations("extraction/image_decode.rs", moved_after_allocation).len(),
        1
    );
    assert!(decoder_audit_violations("extraction/image_decode.rs", guarded).is_empty());
}

fn rust_sources(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            rust_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

type UseAliases = BTreeMap<String, Vec<String>>;

fn collect_use_aliases(tree: &syn::UseTree, prefix: &mut Vec<String>, aliases: &mut UseAliases) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_aliases(&path.tree, prefix, aliases);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut full_path = prefix.clone();
            full_path.push(name.ident.to_string());
            aliases
                .entry(name.ident.to_string())
                .or_default()
                .push(full_path.join("::"));
        }
        syn::UseTree::Rename(rename) => {
            let mut full_path = prefix.clone();
            full_path.push(rename.ident.to_string());
            aliases
                .entry(rename.rename.to_string())
                .or_default()
                .push(full_path.join("::"));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_aliases(item, prefix, aliases);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn resolved_paths(path: &syn::Path, aliases: &UseAliases) -> Vec<String> {
    let mut segments = path.segments.iter().map(|segment| segment.ident.to_string());
    let Some(first) = segments.next() else {
        return Vec::new();
    };
    let suffix: Vec<_> = segments.collect();
    let Some(prefixes) = aliases.get(&first) else {
        return vec![std::iter::once(first).chain(suffix).collect::<Vec<_>>().join("::")];
    };
    prefixes
        .iter()
        .map(|prefix| {
            std::iter::once(prefix.clone())
                .chain(suffix.clone())
                .collect::<Vec<_>>()
                .join("::")
        })
        .collect()
}

fn is_direct_decoder_path(path: &str) -> bool {
    matches!(
        path,
        "image::open"
            | "image::load_from_memory"
            | "image::load_from_memory_with_format"
            | "image::ImageReader::new"
            | "image::ImageReader::with_format"
            | "image::ImageReader::open"
            | "hayro_jbig2::Image::new"
            | "hayro_jpeg2000::Image::new"
            | "tiff::decoder::Decoder::new"
            | "sceptre::Image::from_bytes"
            | "xberg_libheif::HeifContext::read_from_bytes"
    ) || (path.starts_with("image::codecs::")
        && (path.ends_with("Decoder::new") || path.ends_with("Decoder::new_without_file_header")))
}

struct UseAliasVisitor {
    aliases: UseAliases,
}

impl<'ast> syn::visit::Visit<'ast> for UseAliasVisitor {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if item.ident != "tests" {
            syn::visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        collect_use_aliases(&item.tree, &mut Vec::new(), &mut self.aliases);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecoderCall {
    function: String,
    path: String,
    guards: std::collections::BTreeSet<String>,
}

struct DecoderCallVisitor<'a> {
    aliases: &'a UseAliases,
    current_function: Option<String>,
    calls: Vec<DecoderCall>,
    current_guards: std::collections::BTreeSet<String>,
    heif_handles: std::collections::BTreeSet<String>,
    decoder_handles: std::collections::BTreeSet<String>,
}

impl<'ast> syn::visit::Visit<'ast> for DecoderCallVisitor<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if item.ident != "tests" {
            syn::visit::visit_item_mod(self, item);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = call.func.as_ref() {
            for path in resolved_paths(&function.path, self.aliases) {
                if let Some(current) = &self.current_function {
                    self.current_guards.insert(path.clone());
                    if is_direct_decoder_path(&path) {
                        self.calls.push(DecoderCall {
                            function: current.clone(),
                            path,
                            guards: self.current_guards.clone(),
                        });
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let decodes_heif_handle = call.method == "decode"
            && call.args.first().is_some_and(|argument| {
                matches!(argument, syn::Expr::Reference(reference) if matches!(reference.expr.as_ref(), syn::Expr::Path(path) if path.path.get_ident().is_some_and(|ident| self.heif_handles.contains(&ident.to_string()))))
            });
        if decodes_heif_handle && let Some(current) = &self.current_function {
            self.calls.push(DecoderCall {
                function: current.clone(),
                path: "xberg_libheif::LibHeif::decode".to_string(),
                guards: self.current_guards.clone(),
            });
        }
        let decodes_tracked_decoder = matches!(call.method.to_string().as_str(), "decode" | "read_image")
            && matches!(call.receiver.as_ref(), syn::Expr::Path(path) if path.path.get_ident().is_some_and(|ident| self.decoder_handles.contains(&ident.to_string())));
        if decodes_tracked_decoder && let Some(current) = &self.current_function {
            self.calls.push(DecoderCall {
                function: current.clone(),
                path: format!("decoder::{}", call.method),
                guards: self.current_guards.clone(),
            });
        }
        if self.current_function.is_some() {
            self.current_guards.insert(format!("method::{}", call.method));
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let syn::Pat::Ident(binding) = &local.pat
            && local
                .init
                .as_ref()
                .is_some_and(|init| expression_calls_method(&init.expr, "primary_image_handle"))
        {
            self.heif_handles.insert(binding.ident.to_string());
        }
        if let syn::Pat::Ident(binding) = &local.pat
            && local
                .init
                .as_ref()
                .is_some_and(|init| expression_contains_direct_decoder(&init.expr, self.aliases))
        {
            self.decoder_handles.insert(binding.ident.to_string());
        }
        syn::visit::visit_local(self, local);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        let function_name = function.sig.ident.to_string();
        let previous = self.current_function.replace(function_name.clone());
        let previous_guards = std::mem::take(&mut self.current_guards);
        let previous_handles = std::mem::take(&mut self.heif_handles);
        let previous_decoder_handles = std::mem::take(&mut self.decoder_handles);
        syn::visit::visit_block(self, &function.block);
        self.current_guards = previous_guards;
        self.heif_handles = previous_handles;
        self.decoder_handles = previous_decoder_handles;
        self.current_function = previous;
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        let function_name = function.sig.ident.to_string();
        let previous = self.current_function.replace(function_name.clone());
        let previous_guards = std::mem::take(&mut self.current_guards);
        let previous_handles = std::mem::take(&mut self.heif_handles);
        let previous_decoder_handles = std::mem::take(&mut self.decoder_handles);
        syn::visit::visit_block(self, &function.block);
        self.current_guards = previous_guards;
        self.heif_handles = previous_handles;
        self.decoder_handles = previous_decoder_handles;
        self.current_function = previous;
    }
}

fn expression_contains_direct_decoder(expression: &syn::Expr, aliases: &UseAliases) -> bool {
    struct DecoderFinder<'a> {
        aliases: &'a UseAliases,
        found: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for DecoderFinder<'_> {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(function) = call.func.as_ref() {
                self.found |= resolved_paths(&function.path, self.aliases)
                    .iter()
                    .any(|path| is_direct_decoder_path(path));
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    use syn::visit::Visit;
    let mut finder = DecoderFinder { aliases, found: false };
    finder.visit_expr(expression);
    finder.found
}

fn expression_calls_method(expression: &syn::Expr, method: &str) -> bool {
    struct MethodFinder<'a> {
        method: &'a str,
        found: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for MethodFinder<'_> {
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            self.found |= call.method == self.method;
            syn::visit::visit_expr_method_call(self, call);
        }
    }
    use syn::visit::Visit;
    let mut finder = MethodFinder { method, found: false };
    finder.visit_expr(expression);
    finder.found
}

fn direct_decoder_calls(source: &str) -> Vec<DecoderCall> {
    use syn::visit::Visit;

    let syntax = syn::parse_file(source).expect("Rust source must parse");
    let mut alias_visitor = UseAliasVisitor {
        aliases: BTreeMap::new(),
    };
    alias_visitor.visit_file(&syntax);
    let mut visitor = DecoderCallVisitor {
        aliases: &alias_visitor.aliases,
        current_function: None,
        calls: Vec::new(),
        current_guards: std::collections::BTreeSet::new(),
        heif_handles: std::collections::BTreeSet::new(),
        decoder_handles: std::collections::BTreeSet::new(),
    };
    visitor.visit_file(&syntax);
    visitor.calls
}

#[test]
fn direct_image_decode_api_calls_are_audited() {
    let test_only_sources = ["paddle_ocr/tract_parity.rs"];
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_sources(&source_root, &mut files);
    let mut violations = BTreeMap::new();
    for path in files {
        let relative = path
            .strip_prefix(&source_root)
            .expect("source path under root")
            .to_string_lossy()
            .replace('\\', "/");
        if test_only_sources.contains(&relative.as_str()) || is_test_submodule(&relative) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read Rust source");
        let file_violations = decoder_audit_violations(&relative, &source);
        if !file_violations.is_empty() {
            violations.insert(relative, file_violations);
        }
    }
    assert_eq!(
        violations,
        BTreeMap::new(),
        "direct image decoder calls must live in an audited wrapper"
    );
}

/// A file that is only ever compiled as a `#[cfg(test)] mod tests;` submodule.
///
/// The audit walks source files, not modules, so it cannot see the `#[cfg(test)]` that gates
/// them. While a test module lived inline in its parent file the surrounding `cfg` kept it out
/// of scope; the GH#1567 paydown moved 18 such modules into sibling `tests.rs` files, and each
/// one then read as production code calling a decoder unguarded. Keyed on the convention rather
/// than on a path list, so the next split does not reopen this. ~keep
fn is_test_submodule(relative: &str) -> bool {
    let mut parts = relative.split('/');
    let name = parts.next_back().unwrap_or(relative);
    name == "tests.rs" || name.ends_with("_tests.rs") || parts.any(|dir| dir.ends_with("_tests"))
}

fn decoder_audit_violations(relative: &str, source: &str) -> Vec<String> {
    direct_decoder_calls(source)
        .into_iter()
        .filter_map(|call| {
            let approved = approved_decoder_guard(relative, &call.function, &call.path)
                .is_some_and(|guard| guard == "header-only" || call.guards.contains(guard));
            (!approved).then(|| format!("{}: {} (guards: {:?})", call.function, call.path, call.guards))
        })
        .collect()
}

fn approved_decoder_guard(relative: &str, function: &str, path: &str) -> Option<&'static str> {
    match (relative, function, path) {
        ("extraction/image_decode.rs", "probe_standard_image", _) => Some("method::validate"),
        ("extraction/image_decode.rs", "decode_standard_image", _)
        | ("extraction/image_decode.rs", "decode_standard_rgb8", _)
        | ("extraction/image_decode.rs", "decode_standard_luma8_with_security_limits", _) => {
            Some("probe_standard_image")
        }
        ("extraction/image_decode.rs", "standard_image_is_single_frame", _) => Some("header-only"),
        ("extraction/image.rs", "decode_jp2_to_rgb_with_security_limits", "hayro_jpeg2000::Image::new")
        | ("extraction/image.rs", "decode_jbig2_to_gray_with_security_limits", "hayro_jbig2::Image::new") => {
            Some("validate_encoded_image_input")
        }
        ("extraction/image.rs", "decode_jp2_to_rgb_with_security_limits", "decoder::decode")
        | ("extraction/image.rs", "decode_jbig2_to_gray_with_security_limits", "decoder::decode") => {
            Some("method::validate")
        }
        ("extraction/image.rs", "extract_image_metadata_with_security_limits", _)
        | ("extraction/image.rs", "extract_image_metadata_for_processing", _) => Some("method::validate"),
        ("extraction/image.rs", "detect_tiff_frame_count", _) => Some("header-only"),
        ("extraction/heif.rs", "decode_heic_to_png", "xberg_libheif::HeifContext::read_from_bytes") => {
            Some("validate_heif_encoded_input_budget")
        }
        ("extraction/heif.rs", "decode_heic_to_png", "xberg_libheif::LibHeif::decode") => {
            Some("validate_heif_decode_budget")
        }
        ("core/image_encode.rs", "decode_heic", "xberg_libheif::HeifContext::read_from_bytes") => {
            Some("validate_heic_encoded_input_budget")
        }
        ("core/image_encode.rs", "decode_heic", "xberg_libheif::LibHeif::decode") => {
            Some("validate_heic_decode_budget")
        }
        ("ocr/tesseract_wasm_backend.rs", "decode_wasm_ocr_image", "image::ImageReader::new") => {
            Some("wasm_ocr_decode_limits")
        }
        ("ocr/tesseract_wasm_backend.rs", "decode_wasm_ocr_image", "image::ImageReader::with_format")
        | ("ocr/tesseract_wasm_backend.rs", "decode_wasm_ocr_image", "decoder::decode") => {
            Some("validate_wasm_ocr_dimensions")
        }
        _ => None,
    }
}
