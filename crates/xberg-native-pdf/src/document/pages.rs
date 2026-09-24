//! Page tree traversal, counting, and per-page geometry.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    fn inheritable_value_is_resolvable(&self, value: &Object) -> bool {
        let Some(reference) = value.as_reference() else {
            return true;
        };
        // ~keep Free xref slots load as Null rather than Err; neither can supply an inherited value.
        matches!(self.load_object(reference), Ok(resolved) if !matches!(resolved, Object::Null | Object::Reference(_)))
    }

    /// Get the number of pages in the document.
    ///
    /// This function:
    /// 1. Loads the catalog (root object)
    /// 2. Follows the /Pages reference to the page tree root
    /// 3. Extracts the /Count value from the page tree
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The catalog cannot be loaded
    /// - The /Pages entry is missing or invalid
    /// - The page tree root does not contain a /Count entry
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let count = doc.page_count()?;
    /// println!("Document has {} pages", count);
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn page_count(&self) -> Result<usize> {
        let primary: Result<usize> = match self.get_page_count_standard() {
            Ok(count) => {
                tracing::debug!(target: LOG_TARGET, "Page count from /Count: {}", count);
                Ok(count)
            }
            Err(Error::EncryptedPdf) => return Err(Error::EncryptedPdf),
            Err(e) => {
                // For encrypted PDFs any failure to read the page tree means we
                // cannot access the content, so surface it immediately. ~keep
                if self.is_encrypted() {
                    trace_recoverable_pdf_error("count_encrypted_pages", &e);
                    return Err(Error::EncryptedPdf);
                }
                trace_recoverable_pdf_error("count_pages_from_tree", &e);
                match self.get_page_count_by_scanning() {
                    Ok(count) => {
                        tracing::warn!(target: LOG_TARGET, "Page count from scanning: {}", count);
                        Ok(count)
                    }
                    Err(scan_err) => {
                        tracing::error!(target: LOG_TARGET,
                            operation = "count_pages",
                            primary_error_code = e.telemetry_code(),
                            primary_error_offset = ?e.telemetry_offset(),
                            fallback_error_code = scan_err.telemetry_code(),
                            fallback_error_offset = ?scan_err.telemetry_offset(),
                            "all PDF page count strategies failed"
                        );
                        Err(e)
                    }
                }
            }
        };

        // Enumerator rescue. A count of 0 from the /Count-based readers on a
        // non-encrypted document is almost always a page tree they could not
        // resolve - `/Pages` packed inside an object stream, or a deeply nested
        // `/Pages` -> `/Pages` -> `/Page` tree - not a genuinely empty document.
        // The /Count readers and `all_page_refs` (which walks `/Pages` -> `/Kids`
        // via `collect_page_refs`) both MISS such a tree; `get_page` still reaches
        // every page through its own per-page traversal / `collect_all_pages` bulk
        // walk, so count by agreeing with what it can actually reach. Gated on a
        // primary result of 0, so every document the standard reader counts
        // normally is unchanged. ~keep
        if matches!(primary, Ok(0)) && !self.is_encrypted() {
            // The /Count readers - and `all_page_refs`, which walks
            // `/Pages` -> `/Kids` via `collect_page_refs` - miss a page tree
            // packed inside an object stream. `get_page` still resolves every
            // such page through its own per-page traversal / `collect_all_pages`
            // bulk walk, so count by probing it: the definitive agreement with
            // the pages the rest of the API can actually reach. `get_page` never
            // calls back into `page_count` (no recursion) and caches each page
            // (repeat probes are cheap). For an ObjStm-packed tree each `get_page`
            // can fall back to a full object scan, so counting this way is
            // O(n * objects) - bounded by the sanity cap, and only ever on an
            // already-broken document. Only runs when the primary count is 0, so
            // normally-counted documents are byte-identical. ~keep
            let mut n = 0usize;
            while n < 1_000_000 && self.get_page(n).is_ok() {
                n += 1;
            }
            if n > 0 {
                tracing::warn!(target: LOG_TARGET, "Page /Count was 0; enumerated {} pages via get_page", n);
                return Ok(n);
            }
        }
        primary
    }

    /// Get the MediaBox of a page.
    ///
    /// MediaBox defines the physical boundaries of the page in user space units.
    pub fn get_page_media_box(&self, page_index: usize) -> Result<(f32, f32, f32, f32)> {
        let page = self.get_page(page_index)?;
        let page_dict = page
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Page is not a dictionary".to_string()))?;

        // Resolve indirect reference if present — PDF spec §7.3.10 permits any value
        // to be an indirect reference, e.g. `/MediaBox 174 0 R` where 174 0 R is `[0 0 612 792]`.
        // ~keep
        let media_box_obj_raw = page_dict
            .get("MediaBox")
            .ok_or_else(|| Error::InvalidPdf("MediaBox not found or not an array".to_string()))?;
        let media_box_obj = self.resolve_obj_ref(media_box_obj_raw);
        let media_box = media_box_obj
            .as_array()
            .ok_or_else(|| Error::InvalidPdf("MediaBox not found or not an array".to_string()))?;

        if media_box.len() < 4 {
            return Err(Error::InvalidPdf("MediaBox must have at least 4 elements".to_string()));
        }

        fn to_f32(obj: &Object) -> f32 {
            match obj {
                Object::Integer(v) => *v as f32,
                Object::Real(v) => *v as f32,
                _ => 0.0,
            }
        }

        // §7.3.10: *any* element of the rectangle array may itself be an
        // indirect reference (pdf.js issue7872 stores `/MediaBox
        // [4 0 R 5 0 R 6 0 R 7 0 R]`). Resolve each element before
        // coercing — otherwise an unresolved Reference reads as 0.0 and
        // the page collapses to a zero-area box that clips all content. ~keep
        Ok((
            to_f32(&self.resolve_obj_ref(&media_box[0])),
            to_f32(&self.resolve_obj_ref(&media_box[1])),
            to_f32(&self.resolve_obj_ref(&media_box[2])),
            to_f32(&self.resolve_obj_ref(&media_box[3])),
        ))
    }

    /// Page `/Rotate` normalised to one of `{0, 90, 180, 270}`
    /// (ISO 32000-1 §7.7.3.3); `0` when absent or invalid.
    ///
    /// Pure inspection (no feature gate) for the auto-extraction
    /// classifier (case I — transformed-bbox coverage / OCR
    /// orientation). Resolves via [`get_page`](Self::get_page), so the
    /// inheritable `/Rotate` attribute (ISO 32000-1 §7.7.3.4) is walked
    /// up the page tree — a `/Rotate` set on an ancestor `/Pages` node
    /// is honoured, not just one on the leaf page object.
    pub fn get_page_rotation(&self, page_index: usize) -> Result<i32> {
        let page = self.get_page(page_index)?;
        let dict = page
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Page is not a dictionary".to_string()))?;
        let raw = match dict.get("Rotate") {
            Some(r) => match self.resolve_obj_ref(r) {
                Object::Integer(v) => v as i32,
                Object::Real(v) => v as i32,
                _ => 0,
            },
            None => 0,
        };
        // `/Rotate` shall be a multiple of 90 (ISO 32000-1 §7.7.3.3);
        // a non-multiple is invalid → `0` (per this fn's contract),
        // NOT silently floored (e.g. 135 must not become 90). ~keep
        let n = ((raw % 360) + 360) % 360;
        Ok(if n % 90 == 0 { n } else { 0 })
    }

    /// Sibling of [`get_page_rotation`](Self::get_page_rotation) that keeps the
    /// three cases the latter folds into `0` (absent, non-numeric, and a
    /// numeric value that is not a multiple of 90) distinct. Same inheritance
    /// resolution via [`get_page`](Self::get_page).
    pub fn get_page_rotation_status(&self, page_index: usize) -> Result<PageRotation> {
        let page = self.get_page(page_index)?;
        let dict = page
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Page is not a dictionary".to_string()))?;
        let Some(r) = dict.get("Rotate") else {
            return Ok(PageRotation::Absent);
        };
        let raw = match self.resolve_obj_ref(r) {
            Object::Integer(v) => v as i32,
            // ~keep A non-integral `/Rotate` (e.g. `90.5`) is out of spec. `get_page_rotation`
            // truncates it because it folds everything invalid to 0 anyway; here truncating
            // would report `Valid(90)` and re-introduce the exact conflation this type exists
            // to remove, so only an integral Real is accepted.
            Object::Real(v) if v.fract() == 0.0 => v as i32,
            _ => return Ok(PageRotation::Malformed),
        };
        let n = ((raw % 360) + 360) % 360;
        Ok(if n % 90 == 0 {
            PageRotation::Valid(n)
        } else {
            PageRotation::Malformed
        })
    }

    /// Get page count using the standard /Count field
    pub(super) fn get_page_count_standard(&self) -> Result<usize> {
        let catalog = self.catalog()?;
        let catalog_dict = catalog.as_dict().ok_or_else(|| Error::InvalidObjectType {
            expected: "Dictionary".to_string(),
            found: catalog.type_name().to_string(),
        })?;

        let pages_ref = catalog_dict
            .get("Pages")
            .ok_or_else(|| Error::InvalidPdf("Catalog missing /Pages entry".to_string()))?
            .as_reference()
            .ok_or_else(|| Error::InvalidPdf("/Pages is not a reference".to_string()))?;

        let pages_obj = self.load_object(pages_ref)?;
        let pages_dict = match pages_obj.as_dict() {
            Some(d) => d,
            None => {
                // If the page tree root resolved to Null it usually means the
                // PDF is encrypted and the page tree could not be decrypted.
                // Surface the real error instead of silently reporting 0 pages. ~keep
                if matches!(pages_obj, crate::object::Object::Null) && self.is_encrypted() {
                    return Err(Error::EncryptedPdf);
                }
                tracing::warn!(target: LOG_TARGET,
                    "Page tree root is {} (expected Dictionary), treating as 0 pages",
                    pages_obj.type_name()
                );
                return Ok(0);
            }
        };

        let count = pages_dict
            .get("Count")
            .ok_or_else(|| Error::InvalidPdf("Page tree missing /Count entry".to_string()))?
            .as_integer()
            .ok_or_else(|| Error::InvalidPdf("/Count is not an integer".to_string()))?;

        // Validate /Count against PDF spec limits (Annex C.2: max 8,388,607 indirect objects) ~keep
        const MAX_PAGES: i64 = 8_388_607;
        if !(0..=MAX_PAGES).contains(&count) {
            tracing::warn!(target: LOG_TARGET,
                "/Count value {} is unreasonable (max {}), falling back to tree scan",
                count,
                MAX_PAGES
            );
            return self.get_page_count_by_scanning();
        }

        let max_objects = self.xref.len();
        if (count as usize) > max_objects {
            tracing::warn!(target: LOG_TARGET,
                "/Count {} exceeds total objects {}, falling back to tree scan",
                count,
                max_objects
            );
            return self.get_page_count_by_scanning();
        }

        Ok(count as usize)
    }

    /// Get page count by scanning the page tree (fallback method)
    fn get_page_count_by_scanning(&self) -> Result<usize> {
        let catalog = self.catalog()?;
        let catalog_dict = catalog.as_dict().ok_or_else(|| Error::InvalidObjectType {
            expected: "Dictionary".to_string(),
            found: catalog.type_name().to_string(),
        })?;

        let pages_ref = catalog_dict
            .get("Pages")
            .ok_or_else(|| Error::InvalidPdf("Catalog missing /Pages entry".to_string()))?
            .as_reference()
            .ok_or_else(|| Error::InvalidPdf("/Pages is not a reference".to_string()))?;

        self.count_pages_recursive(pages_ref, 0)
    }

    /// Recursively count pages in the page tree
    pub(super) fn count_pages_recursive(&self, node_ref: ObjectRef, depth: u32) -> Result<usize> {
        if depth >= MAX_PAGE_TREE_DEPTH {
            tracing::warn!(target: LOG_TARGET,
                object_id = node_ref.id,
                max_depth = MAX_PAGE_TREE_DEPTH,
                "page tree depth limit exceeded while counting pages"
            );
            return Err(Error::RecursionLimitExceeded(MAX_PAGE_TREE_DEPTH));
        }

        let node = match self.load_object(node_ref) {
            Ok(n) => n,
            Err(error) => {
                tracing::warn!(target: LOG_TARGET,
                    object_id = node_ref.id,
                    generation = node_ref.generation,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "failed to load page tree node"
                );
                return Ok(0);
            }
        };

        let node_dict = match node.as_dict() {
            Some(d) => d,
            None => {
                tracing::warn!(target: LOG_TARGET, "Page tree node {} is not a dictionary", node_ref);
                return Ok(0);
            }
        };

        let node_type = node_dict.get("Type").and_then(|obj| obj.as_name());

        match node_type {
            Some("Page") => Ok(1),
            Some("Pages") => {
                let kids = match node_dict.get("Kids").and_then(|obj| obj.as_array()) {
                    Some(k) => k,
                    None => {
                        tracing::warn!(target: LOG_TARGET, "Pages node {} missing /Kids array", node_ref);
                        return Ok(0);
                    }
                };

                let mut count = 0;
                for kid in kids {
                    if let Some(kid_ref) = kid.as_reference() {
                        match self.count_pages_recursive(kid_ref, depth + 1) {
                            Ok(page_count) => count += page_count,
                            Err(Error::CircularReference(obj_ref)) => {
                                tracing::warn!(target: LOG_TARGET, "Circular reference in page tree at object {}, skipping", obj_ref);
                                continue;
                            }
                            // GH#1755: RecursionLimitExceeded used to get its own arm here,
                            // but it only ever changed the log line — telemetry_code()
                            // already reports "recursion_limit_exceeded" through the
                            // general arm below, so the split was dead. ~keep
                            Err(error) => {
                                tracing::warn!(target: LOG_TARGET,
                                    error_code = error.telemetry_code(),
                                    error_offset = ?error.telemetry_offset(),
                                    "error counting pages in branch; skipping"
                                );
                                continue;
                            }
                        }
                    }
                }
                Ok(count)
            }
            _ => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "traverse_page_tree",
                    error_code = "unknown_node_type",
                    "skipping unknown page tree node"
                );
                Ok(0)
            }
        }
    }

    /// Get page count as u32 (legacy API).
    ///
    /// This is a convenience method that returns the page count as a u32.
    /// It calls `page_count()` internally but converts the result
    /// returns 0 if an error occurs (for backward compatibility).
    #[deprecated(since = "0.1.0", note = "Use page_count() instead, which returns Result")]
    pub fn page_count_u32(&self) -> u32 {
        self.page_count().unwrap_or(0) as u32
    }

    /// Returns the page index range `0..page_count`, or an empty range
    /// when `page_count()` fails.
    ///
    /// Designed for `for i in doc.page_indices() { ... }` so callers
    /// don't have to write `for i in 0..doc.page_count()?`. The
    /// fallible-vs-iterator tension that motivated the issue is
    /// resolved by treating a metadata-broken document as having no
    /// pages at the iteration level — every per-page extraction call
    /// is already fallible and surfaces the real error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// for i in doc.page_indices() {
    ///     let text = doc.extract_text(i)?;
    ///     println!("page {}: {} chars", i, text.len());
    /// }
    /// ```
    pub fn page_indices(&self) -> std::ops::Range<usize> {
        0..self.page_count().unwrap_or(0)
    }

    /// Get a page object by index (0-based).
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// The page dictionary object.
    ///
    /// # Errors
    ///
    /// Returns an error if the page index is out of bounds or if the page
    /// tree structure is invalid.
    pub fn get_page(&self, page_index: usize) -> Result<Object> {
        // Check page cache first — page tree is static per §7.7.3.2 ~keep
        if let Some(cached) = self.page_cache.lock_or_recover().get(&page_index).cloned() {
            return Ok(cached);
        }

        // Defer bulk page tree walk until enough pages are accessed. ~keep
        const LAZY_THRESHOLD: usize = 64;
        let cache_misses = self.page_cache.lock_or_recover().len();

        if !self.page_cache_populated.load(Ordering::Acquire) && cache_misses >= LAZY_THRESHOLD {
            self.page_cache_populated.store(true, Ordering::Release);
            if let Err(e) = self.populate_page_cache() {
                trace_recoverable_pdf_error("populate_page_cache", &e);
            }
            if let Some(cached) = self.page_cache.lock_or_recover().get(&page_index).cloned() {
                return Ok(cached);
            }
        }

        let catalog = self.catalog()?;
        let catalog_dict = catalog.as_dict().ok_or_else(|| Error::InvalidObjectType {
            expected: "Dictionary".to_string(),
            found: catalog.type_name().to_string(),
        })?;

        let pages_ref = catalog_dict
            .get("Pages")
            .ok_or_else(|| Error::InvalidPdf("Catalog missing /Pages entry".to_string()))?
            .as_reference()
            .ok_or_else(|| Error::InvalidPdf("/Pages is not a reference".to_string()))?;

        let mut inherited = HashMap::new();

        let page = match self.get_page_from_tree(pages_ref, page_index, &mut 0, &mut inherited) {
            Ok(page) => {
                if let Some(dict) = page.as_dict() {
                    tracing::debug!(target: LOG_TARGET, "Collected page {}, keys: {:?}", page_index, dict.keys());
                    if let Some(contents) = dict.get("Contents") {
                        tracing::debug!(target: LOG_TARGET, "  -> /Contents: {:?}", contents);
                    }
                    if let Some(rotate) = dict.get("Rotate") {
                        tracing::debug!(target: LOG_TARGET, "  -> /Rotate: {:?}", rotate);
                    }
                }
                Ok(page)
            }
            Err(error) => {
                if matches!(
                    error,
                    Error::InvalidPdf(_)
                        | Error::InvalidObjectType { .. }
                        | Error::CircularReference(_)
                        | Error::ObjectNotFound(_, _)
                ) {
                    trace_recoverable_pdf_error("traverse_page_tree", &error);
                    self.get_page_by_scanning(page_index)
                } else {
                    Err(error)
                }
            }
        }?;

        self.page_cache.lock_or_recover().insert(page_index, page.clone());
        Ok(page)
    }

    /// Walk the page tree once and populate page_cache for ALL pages.
    /// This avoids O(n²) cost when pages are accessed sequentially.
    fn populate_page_cache(&self) -> Result<()> {
        let catalog = self.catalog()?;
        let catalog_dict = catalog.as_dict().ok_or_else(|| Error::InvalidObjectType {
            expected: "Dictionary".to_string(),
            found: catalog.type_name().to_string(),
        })?;

        let pages_ref = catalog_dict
            .get("Pages")
            .ok_or_else(|| Error::InvalidPdf("Catalog missing /Pages entry".to_string()))?
            .as_reference()
            .ok_or_else(|| Error::InvalidPdf("/Pages is not a reference".to_string()))?;

        let mut page_index = 0usize;
        let mut inherited = HashMap::new();
        self.collect_all_pages(pages_ref, &mut page_index, &mut inherited, &mut HashSet::new(), 0)?;
        tracing::debug!(target: LOG_TARGET, "Populated page cache with {} pages", page_index);
        Ok(())
    }

    /// Pre-populate `image_xobject_cache` for all XObject refs across all cached pages.
    /// Collects all unique XObject references, sorts them by xref offset for sequential
    /// I/O (avoids random seeking in large files), then peeks each one via `is_form_xobject()`.
    #[allow(dead_code)]
    fn prefetch_xobject_subtypes(&self) {
        let mut xobj_refs: Vec<ObjectRef> = Vec::new();
        let page_dicts: Vec<Object> = self.page_cache.lock_or_recover().values().cloned().collect();

        for page_obj in &page_dicts {
            let page_dict = match page_obj.as_dict() {
                Some(d) => d,
                None => continue,
            };
            let resources = match page_dict.get("Resources") {
                Some(r) => {
                    if let Some(ref_obj) = r.as_reference() {
                        match self.load_object(ref_obj) {
                            Ok(obj) => obj,
                            Err(_) => continue,
                        }
                    } else {
                        r.clone()
                    }
                }
                None => continue,
            };
            let res_dict = match resources.as_dict() {
                Some(d) => d,
                None => continue,
            };
            let xobj_obj = match res_dict.get("XObject") {
                Some(x) => {
                    if let Some(ref_obj) = x.as_reference() {
                        match self.load_object(ref_obj) {
                            Ok(obj) => obj,
                            Err(_) => continue,
                        }
                    } else {
                        x.clone()
                    }
                }
                None => continue,
            };
            if let Some(xobj_dict) = xobj_obj.as_dict() {
                for val in xobj_dict.values() {
                    if let Some(obj_ref) = val.as_reference()
                        && !self.image_xobject_cache.lock_or_recover().contains(&obj_ref)
                    {
                        xobj_refs.push(obj_ref);
                    }
                }
            }
        }

        xobj_refs.sort_unstable_by_key(|r| (r.id, r.generation));
        xobj_refs.dedup();

        xobj_refs.sort_by_key(|r| self.xref.get(r.id).map(|e| e.offset).unwrap_or(u64::MAX));

        tracing::debug!(target: LOG_TARGET, "Prefetching XObject subtypes for {} unique refs", xobj_refs.len());

        // Peek each ref — populates image_xobject_cache as a side effect ~keep
        for obj_ref in xobj_refs {
            self.is_form_xobject(obj_ref);
        }
    }

    /// Recursively walk the page tree and collect all pages into page_cache.
    fn collect_all_pages(
        &self,
        node_ref: ObjectRef,
        page_index: &mut usize,
        inherited: &mut HashMap<String, Object>,
        visited: &mut HashSet<ObjectRef>,
        depth: u32,
    ) -> Result<()> {
        if depth >= MAX_PAGE_TREE_DEPTH {
            tracing::warn!(target: LOG_TARGET,
                object_id = node_ref.id,
                max_depth = MAX_PAGE_TREE_DEPTH,
                "page tree depth limit exceeded while collecting pages"
            );
            return Err(Error::RecursionLimitExceeded(MAX_PAGE_TREE_DEPTH));
        }
        if !visited.insert(node_ref) {
            return Err(Error::CircularReference(node_ref));
        }

        let node = self.load_object(node_ref)?;
        let node_dict = match node.as_dict() {
            Some(d) => d,
            None => return Ok(()),
        };

        let node_type = node_dict.get("Type").and_then(|obj| obj.as_name()).unwrap_or("");

        match node_type {
            "Page" => {
                let mut page_dict = node_dict.clone();
                for attr_name in &["Resources", "MediaBox", "CropBox", "Rotate"] {
                    if !page_dict
                        .get(*attr_name)
                        .is_some_and(|value| self.inheritable_value_is_resolvable(value))
                        && let Some(inherited_value) = inherited.get(*attr_name)
                    {
                        tracing::debug!(target: LOG_TARGET, "Page {} inheriting {}: {:?}", *page_index, attr_name, inherited_value);
                        page_dict.insert(attr_name.to_string(), inherited_value.clone());
                    }
                }
                tracing::debug!(target: LOG_TARGET, "Collected page {}, keys: {:?}", *page_index, page_dict.keys());
                if let Some(contents) = page_dict.get("Contents") {
                    tracing::debug!(target: LOG_TARGET, "  -> /Contents: {:?}", contents);
                }
                if let Some(rotate) = page_dict.get("Rotate") {
                    tracing::debug!(target: LOG_TARGET, "  -> /Rotate: {:?}", rotate);
                }
                self.page_cache
                    .lock_or_recover()
                    .insert(*page_index, Object::Dictionary(page_dict));
                *page_index += 1;
            }
            "Pages" => {
                // Save inherited state so siblings don't see each other's overrides ~keep
                let saved = inherited.clone();

                // Nearest ancestor's attributes override more distant ones (PDF spec §7.7.3.4).
                // insert() is correct here because we snapshot/restore `inherited` around
                // the recursion, so this node's values apply only to its subtree. ~keep
                for attr_name in &["Resources", "MediaBox", "CropBox", "Rotate"] {
                    if let Some(attr_value) = node_dict
                        .get(*attr_name)
                        .filter(|value| self.inheritable_value_is_resolvable(value))
                    {
                        tracing::debug!(target: LOG_TARGET,
                            "Pages node at {:?} providing inheritable {}: {:?}",
                            node_ref,
                            attr_name,
                            attr_value
                        );
                        inherited.insert(attr_name.to_string(), attr_value.clone());
                    }
                }

                if let Some(kids) = node_dict.get("Kids").and_then(|obj| obj.as_array()) {
                    for kid in kids {
                        if let Some(kid_ref) = kid.as_reference()
                            && let Err(error) =
                                self.collect_all_pages(kid_ref, page_index, inherited, visited, depth + 1)
                        {
                            tracing::warn!(target: LOG_TARGET,
                                error_code = error.telemetry_code(),
                                error_offset = ?error.telemetry_offset(),
                                "error collecting page from tree; skipping branch"
                            );
                        }
                    }
                }

                *inherited = saved;
            }
            _ => {}
        }

        Ok(())
    }

    /// Get a page by scanning all objects in the PDF (fallback for broken page trees)
    /// This method is used when the standard page tree traversal fails due to malformed structure.
    fn get_page_by_scanning(&self, target_index: usize) -> Result<Object> {
        let mut current_index = 0;

        // Prime the ObjStm recovery cache up front when the xref looks
        // unreliable. Without this, the first pass below iterates only
        // `xref.all_object_numbers()` — which misses compressed objects
        // whose xref slots have been mis-flagged free. The sweep is a
        // one-shot, guarded by `objstm_recovery_done`, so this is cheap
        // if recovery already happened. ~keep
        self.recover_from_object_streams();

        // Collect all object numbers first to avoid borrow checker issues.
        // Sort for deterministic iteration order (HashMap iteration is
        // non-deterministic). We union the xref-listed ids with the object
        // ids recovered from the ObjStm sweep so that pages compressed in
        // streams whose xref slots were mis-flagged free still get visited. ~keep
        let mut obj_nums: Vec<u32> = self.xref.all_object_numbers().collect();
        for r in self.object_cache.lock_or_recover().keys() {
            obj_nums.push(r.id);
        }
        obj_nums.sort_unstable();
        obj_nums.dedup();

        for &obj_num in &obj_nums {
            if let Ok(obj) = self.load_object(ObjectRef {
                id: obj_num,
                generation: 0,
            }) && let Some(dict) = obj.as_dict()
                && let Some(type_obj) = dict.get("Type")
                && let Some(type_name) = type_obj.as_name()
                && type_name == "Page"
            {
                if current_index == target_index {
                    return Ok(obj);
                }
                current_index += 1;
            }
        }

        // Second pass: heuristic detection for pages without /Type entry.
        // Runs as a complement to pass 1 — counts page-like dicts that lack
        // a /Type entry alongside the /Type /Page matches, so that PDFs
        // whose corruption stripped /Type from some page dicts still reach
        // the full page count. Previously this pass only ran when pass 1
        // found zero pages, which meant any partial pass-1 match (e.g. 200
        // of 253 pages) would silently short pass 2 and fail. ~keep
        let mut heuristic_index = current_index;
        for &obj_num in &obj_nums {
            if let Ok(obj) = self.load_object(ObjectRef {
                id: obj_num,
                generation: 0,
            }) && let Some(dict) = obj.as_dict()
            {
                let has_no_type = dict.get("Type").is_none();
                // Also handle /Type that is an unresolvable reference (Null) ~keep
                let type_is_null = dict.get("Type").is_some_and(|t| matches!(t, Object::Null));
                if (has_no_type || type_is_null)
                    && (dict.contains_key("MediaBox")
                        || dict.contains_key("Contents")
                        || (dict.contains_key("Resources") && dict.contains_key("Parent")))
                {
                    tracing::warn!(target: LOG_TARGET,
                        "Heuristic page candidate: object {} (page-like keys without valid /Type)",
                        obj_num
                    );
                    if heuristic_index == target_index {
                        return Ok(obj);
                    }
                    heuristic_index += 1;
                }
            }
        }
        current_index = heuristic_index;

        if current_index == 0
            && let Ok(catalog) = self.catalog()
            && let Some(catalog_dict) = catalog.as_dict()
            && let Some(pages_ref) = catalog_dict.get("Pages").and_then(|p| p.as_reference())
            && let Ok(pages_obj) = self.load_object(pages_ref)
            && let Some(pages_dict) = pages_obj.as_dict()
            && let Some(kids) = pages_dict.get("Kids").and_then(|k| k.as_array())
        {
            let mut kids_index = 0;
            for kid in kids {
                if let Some(kid_ref) = kid.as_reference() {
                    // Skip self-referencing kids (cycle detection) ~keep
                    if kid_ref == pages_ref {
                        continue;
                    }
                    if let Ok(kid_obj) = self.load_object(kid_ref)
                        && let Some(kid_dict) = kid_obj.as_dict()
                    {
                        let is_pages_node = kid_dict
                            .get("Type")
                            .and_then(|t| t.as_name())
                            .is_some_and(|n| n == "Pages");
                        if is_pages_node {
                            continue;
                        }
                        if kids_index == target_index {
                            tracing::warn!(target: LOG_TARGET,
                                "Found page {} via direct /Kids resolution of object {}",
                                target_index,
                                kid_ref.id
                            );
                            return Ok(kid_obj);
                        }
                        kids_index += 1;
                    }
                }
            }
        }

        Err(Error::InvalidPdf(format!(
            "Page index {} not found by scanning",
            target_index
        )))
    }

    /// Recursively traverse page tree to find a specific page.
    ///
    /// PDF Spec: ISO 32000-1:2008, Section 7.7.3.3 - Page Objects
    /// Implements attribute inheritance for /Resources, /MediaBox, /CropBox, /Rotate.
    ///
    /// Inheritable attributes from parent Pages nodes are collected as we traverse down
    /// the tree. When a Page is found, inherited attributes are merged in (only if the
    /// Page doesn't already have them - child values override parent values).
    fn get_page_from_tree(
        &self,
        node_ref: ObjectRef,
        target_index: usize,
        current_index: &mut usize,
        inherited: &mut HashMap<String, Object>,
    ) -> Result<Object> {
        self.get_page_from_tree_inner(node_ref, target_index, current_index, inherited, &mut HashSet::new(), 0)
    }

    fn get_page_from_tree_inner(
        &self,
        node_ref: ObjectRef,
        target_index: usize,
        current_index: &mut usize,
        inherited: &mut HashMap<String, Object>,
        visited: &mut HashSet<ObjectRef>,
        depth: u32,
    ) -> Result<Object> {
        if depth >= MAX_PAGE_TREE_DEPTH {
            tracing::warn!(target: LOG_TARGET,
                object_id = node_ref.id,
                max_depth = MAX_PAGE_TREE_DEPTH,
                "page tree depth limit exceeded while resolving a page"
            );
            return Err(Error::RecursionLimitExceeded(MAX_PAGE_TREE_DEPTH));
        }
        if !visited.insert(node_ref) {
            return Err(Error::CircularReference(node_ref));
        }
        let node = self.load_object(node_ref)?;
        let node_dict = match node.as_dict() {
            Some(d) => d,
            None => {
                tracing::warn!(target: LOG_TARGET,
                    "Page tree node {} is {} (expected Dictionary), skipping",
                    node_ref.id,
                    node.type_name()
                );
                return Err(Error::InvalidPdf(format!(
                    "Page tree node {} is not a dictionary",
                    node_ref.id
                )));
            }
        };

        let node_type = node_dict
            .get("Type")
            .and_then(|obj| obj.as_name())
            .ok_or_else(|| Error::InvalidPdf("Page tree node missing /Type".to_string()))?;

        match node_type {
            "Pages" if *current_index < target_index => {
                // Skip entire subtree if /Count shows target is past this node. ~keep
                if let Some(count) = node_dict.get("Count").and_then(|c| c.as_integer()).filter(|&c| c > 0) {
                    let count = count as usize;
                    if *current_index + count <= target_index {
                        *current_index += count;
                        return Err(Error::InvalidPdf(format!(
                            "Page index {} not found in tree",
                            target_index
                        )));
                    }
                }
            }
            _ => {}
        }

        match node_type {
            "Page" => {
                if *current_index == target_index {
                    // Apply inherited attributes to this page
                    // PDF Spec: "If not present in the page dictionary, the value is inherited
                    // from an ancestor node in the page tree" ~keep
                    let mut page_dict = node_dict.clone();

                    // Inheritable attributes per PDF Spec Table 30:
                    // - Resources (required, can be inherited)
                    // - MediaBox (required, can be inherited)
                    // - CropBox (optional, can be inherited)
                    // - Rotate (optional, can be inherited) ~keep
                    let inheritable_attrs = ["Resources", "MediaBox", "CropBox", "Rotate"];

                    for attr_name in &inheritable_attrs {
                        if !page_dict
                            .get(*attr_name)
                            .is_some_and(|value| self.inheritable_value_is_resolvable(value))
                            && let Some(inherited_value) = inherited.get(*attr_name)
                        {
                            tracing::debug!(target: LOG_TARGET,
                                "Page {} inheriting /{} from ancestor Pages node",
                                target_index,
                                attr_name
                            );
                            page_dict.insert(attr_name.to_string(), inherited_value.clone());
                        }
                    }

                    Ok(Object::Dictionary(page_dict))
                } else {
                    *current_index += 1;
                    Err(Error::InvalidPdf(format!(
                        "Page index {} not found in tree",
                        target_index
                    )))
                }
            }
            "Pages" => {
                let inheritable_attrs = ["Resources", "MediaBox", "CropBox", "Rotate"];

                for attr_name in &inheritable_attrs {
                    if let Some(attr_value) = node_dict
                        .get(*attr_name)
                        .filter(|value| self.inheritable_value_is_resolvable(value))
                    {
                        inherited.insert(attr_name.to_string(), attr_value.clone());
                    }
                }

                let kids = match node_dict.get("Kids").and_then(|obj| obj.as_array()) {
                    Some(k) => k,
                    None => {
                        tracing::warn!(target: LOG_TARGET, "Malformed PDF: Pages node missing /Kids array");
                        // Malformed PDF: Pages node has no /Kids array
                        // Gracefully return without error to allow other recovery paths
                        // The scanning method will find pages eventually ~keep
                        return Err(Error::InvalidPdf(
                            "Pages node missing /Kids array - try fallback method".to_string(),
                        ));
                    }
                };

                for kid in kids {
                    let kid_ref = kid
                        .as_reference()
                        .ok_or_else(|| Error::InvalidPdf("Kid in /Kids array is not a reference".to_string()))?;

                    // ~keep A failed child may alter inheritance while descending; isolate siblings.
                    let mut child_inherited = inherited.clone();
                    match self.get_page_from_tree_inner(
                        kid_ref,
                        target_index,
                        current_index,
                        &mut child_inherited,
                        visited,
                        depth + 1,
                    ) {
                        Ok(page) => return Ok(page),
                        Err(Error::CircularReference(obj_ref)) => {
                            tracing::warn!(target: LOG_TARGET, "Circular reference in page tree at object {}, skipping", obj_ref);
                            continue;
                        }
                        // GH#1755: RecursionLimitExceeded used to get its own arm here,
                        // but it only ever changed the log line (and the previous
                        // catch-all below logged nothing at all) — fold both into one
                        // arm with the same structured logging `count_pages_recursive`
                        // uses for its equivalent catch-all. ~keep
                        Err(error) => {
                            tracing::warn!(target: LOG_TARGET,
                                error_code = error.telemetry_code(),
                                error_offset = ?error.telemetry_offset(),
                                "error walking to page in tree; skipping branch"
                            );
                            continue;
                        }
                    }
                }

                Err(Error::InvalidPdf(format!("Page index {} not found", target_index)))
            }
            _ => Err(Error::InvalidPdf(format!("Unknown page tree node type: {}", node_type))),
        }
    }

    /// Get the object reference for a page by index.
    ///
    /// This is used by outline and annotations to find page references.
    pub(crate) fn get_page_ref(&self, page_index: usize) -> Result<ObjectRef> {
        let catalog = self.catalog()?;
        let catalog_dict = catalog.as_dict().ok_or_else(|| Error::InvalidObjectType {
            expected: "Dictionary".to_string(),
            found: catalog.type_name().to_string(),
        })?;

        let pages_ref = catalog_dict
            .get("Pages")
            .ok_or_else(|| Error::InvalidPdf("Catalog missing /Pages entry".to_string()))?
            .as_reference()
            .ok_or_else(|| Error::InvalidPdf("/Pages is not a reference".to_string()))?;

        self.get_page_ref_recursive(pages_ref, page_index, &mut 0, &mut HashSet::new(), 0)
    }

    /// Recursively find page reference in the page tree.
    pub(crate) fn get_page_ref_recursive(
        &self,
        node_ref: ObjectRef,
        target_index: usize,
        current_index: &mut usize,
        visited: &mut HashSet<ObjectRef>,
        depth: u32,
    ) -> Result<ObjectRef> {
        if depth >= MAX_PAGE_TREE_DEPTH {
            tracing::warn!(target: LOG_TARGET,
                object_id = node_ref.id,
                max_depth = MAX_PAGE_TREE_DEPTH,
                "page tree depth limit exceeded while resolving a page reference"
            );
            return Err(Error::RecursionLimitExceeded(MAX_PAGE_TREE_DEPTH));
        }
        if !visited.insert(node_ref) {
            return Err(Error::CircularReference(node_ref));
        }
        let node = self.load_object(node_ref)?;
        let node_dict = match node.as_dict() {
            Some(d) => d,
            None => {
                tracing::warn!(target: LOG_TARGET,
                    "Page tree node {} is {} (expected Dictionary), skipping",
                    node_ref.id,
                    node.type_name()
                );
                return Err(Error::InvalidPdf(format!(
                    "Page tree node {} is not a dictionary",
                    node_ref.id
                )));
            }
        };

        let node_type = node_dict
            .get("Type")
            .and_then(|t| t.as_name())
            .ok_or_else(|| Error::InvalidPdf("Node missing Type".to_string()))?;

        match node_type {
            "Page" => {
                if *current_index == target_index {
                    Ok(node_ref)
                } else {
                    *current_index += 1;
                    Err(Error::InvalidPdf(format!("Page {} not found", target_index)))
                }
            }
            "Pages" => {
                let kids = node_dict
                    .get("Kids")
                    .and_then(|k| k.as_array())
                    .ok_or_else(|| Error::InvalidPdf("Pages node missing Kids".to_string()))?;

                for kid_obj in kids {
                    if let Some(kid_ref) = kid_obj.as_reference() {
                        match self.get_page_ref_recursive(kid_ref, target_index, current_index, visited, depth + 1) {
                            Ok(page_ref) => return Ok(page_ref),
                            Err(_) => continue,
                        }
                    }
                }

                Err(Error::InvalidPdf(format!("Page {} not found", target_index)))
            }
            _ => Err(Error::InvalidPdf(format!("Unknown node type: {}", node_type))),
        }
    }

    /// 0-based page-tree position of a page object.
    ///
    /// Defers to [`Self::all_page_refs`] rather than walking the tree again:
    /// that collector already handles the `/Count` fast path, nodes that omit
    /// `/Type`, and cycles, and sharing it keeps this direction of the
    /// index <-> reference mapping consistent with the other by construction.
    /// A page whose object is not in the tree is an error, never a guess —
    /// a plausible-but-wrong page number is worse than no destination.
    pub(crate) fn page_index_of_ref(&self, page_ref: ObjectRef) -> Result<usize> {
        self.all_page_refs()?
            .iter()
            .position(|candidate| *candidate == page_ref)
            .ok_or_else(|| Error::InvalidPdf(format!("object {} is not a page in the page tree", page_ref.id)))
    }
}
