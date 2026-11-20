//! Document catalog, structure tree, and output intents.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Whether the catalog can be inspected without xref reconstruction or a catalog scan.
    #[doc(hidden)]
    pub fn has_unrecovered_catalog(&self) -> bool {
        !self.xref_reconstructed
            && self
                .trailer
                .as_dict()
                .and_then(|trailer| trailer.get("Root"))
                .and_then(Object::as_reference)
                .is_some()
    }

    /// Get the document catalog (root object).
    ///
    /// The catalog is the root of the document's object hierarchy.
    /// It contains references to the page tree, outlines, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The /Root entry is present but is not a reference
    /// - Loading the catalog object fails
    /// - The trailer omits /Root **and** no `/Type /Catalog` object can be
    ///   found by scanning (the recovery path: a missing /Root is
    ///   not itself fatal — the Catalog is discovered by object scan, as
    ///   Poppler / PDFium do — but it does error if that scan also fails)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let catalog = doc.catalog()?;
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn catalog(&self) -> Result<Object> {
        let trailer_dict = self
            .trailer
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Trailer is not a dictionary".to_string()))?;

        if let Some(root_obj) = trailer_dict.get("Root") {
            let root_ref = root_obj
                .as_reference()
                .ok_or_else(|| Error::InvalidPdf("/Root is not a reference".to_string()))?;
            return self.load_object(root_ref);
        }

        // The trailer omits /Root. A Linearized file's sparse end-of-file
        // trailer legitimately does this; discover the Catalog
        // by scanning indirect objects for /Type /Catalog, as Poppler /
        // PDFium do. ~keep
        self.find_catalog_by_scan().ok_or_else(|| {
            Error::InvalidPdf("Trailer omits /Root and no /Type /Catalog object could be found by scanning".to_string())
        })
    }

    /// Scan indirect objects for the document Catalog (`/Type /Catalog`).
    ///
    /// Used only as a fallback when the trailer omits `/Root`.
    /// Bounded so a pathological xref can't turn this into an unbounded
    /// scan; the Catalog is virtually always one of the first objects.
    ///
    /// The smallest `MAX_SCAN` object numbers are scanned, ascending.
    /// `all_object_numbers()` is `HashMap`-backed, so iterating it directly
    /// would be nondeterministic — a bounded scan over an arbitrary subset
    /// can miss the Catalog on different runs. `smallest_object_numbers`
    /// makes discovery deterministic, scans low-numbered objects first
    /// (where the Catalog conventionally lives), and bounds the candidate
    /// set *before* sorting so a pathological xref stays O(n log MAX_SCAN).
    fn find_catalog_by_scan(&self) -> Option<Object> {
        const MAX_SCAN: usize = 4096;
        let nums = self.xref.smallest_object_numbers(MAX_SCAN);
        let mut checked = 0usize;
        for num in nums {
            if checked >= MAX_SCAN {
                break;
            }
            let generation = match self.xref.get(num) {
                Some(e) if e.in_use => e.generation,
                _ => continue,
            };
            checked += 1;
            if let Ok(obj) = self.load_object(ObjectRef::new(num, generation))
                && obj.as_dict().and_then(|d| d.get("Type")).and_then(|t| t.as_name()) == Some("Catalog")
            {
                tracing::warn!(target: LOG_TARGET, "Catalog discovered by object scan: {} {} obj", num, generation);
                return Some(obj);
            }
        }
        None
    }

    /// Get the structure tree (logical structure) of the document.
    ///
    /// Tagged PDFs contain a structure tree that defines the logical structure
    /// and reading order of the document. This is the PDF-spec-compliant way
    /// to determine reading order.
    ///
    /// Returns `Ok(Some(StructTreeRoot))` if the document has a structure tree,
    /// `Ok(None)` if it's not a tagged PDF, or an error if parsing fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// if let Some(struct_tree) = doc.structure_tree()? {
    ///     println!("This is a Tagged PDF with logical structure");
    /// } else {
    ///     println!("This PDF does not have a structure tree");
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn structure_tree(&self) -> Result<Option<crate::structure::StructTreeRoot>> {
        crate::structure::parse_structure_tree(self)
    }

    /// Returns the document's structure tree, bounding the parse work by an
    /// optional wall-clock `budget`.
    ///
    /// `None` parses the complete tree (identical to [`Self::structure_tree`]);
    /// `Some(duration)` returns `Ok(None)` if parsing exceeds that budget, so a
    /// latency-sensitive caller can fall back to another strategy. Prefer `None`
    /// unless you have a concrete responsiveness requirement.
    pub fn structure_tree_with_budget(
        &self,
        budget: Option<std::time::Duration>,
    ) -> Result<Option<crate::structure::StructTreeRoot>> {
        crate::structure::parse_structure_tree_with_budget(self, budget)
    }

    /// Returns the document's structure tree **only when it is trustworthy for
    /// reading-order purposes**, per ISO 32000-1:2008 §14.8.2.3.1 and §14.7.1.
    ///
    /// A `/StructTreeRoot` encodes the producer's *logical structure order* — a
    /// depth-first traversal of the tag hierarchy — which is authoritative for
    /// reading order independent of glyph geometry (§14.7.1). It is trusted when
    /// the document is `/Marked` (Tagged PDF) **or** the catalog directly
    /// references a `/StructTreeRoot` (PDF 1.3/1.4 tagged files predate the
    /// `/MarkInfo` dictionary; §7.7.2) — matching the historical gate so output
    /// for non-suspect documents is byte-for-byte unchanged — **and**
    /// `/MarkInfo /Suspects` is not `true`. A `true` `/Suspects` flag is the
    /// spec-sanctioned signal (the `/TagSuspect /Ordering` mechanism,
    /// §14.8.2.3.1) that page content order may not match logical structure
    /// order, so the tree is rejected and callers fall back to geometric order.
    ///
    /// Shares `structure_tree_cache`, so this costs a single cached parse.
    pub(crate) fn struct_tree_trustworthy(&self) -> Option<Arc<crate::structure::StructTreeRoot>> {
        let mark = self.mark_info().unwrap_or_default();
        // Suspect documents: geometric reading order is spec-correct
        // (§14.8.2.3.1). This is the only behavioural change versus the legacy
        // inline gate, which never consulted /Suspects. ~keep
        if mark.suspects {
            return None;
        }
        let cached = self.structure_tree_cache.lock_or_recover().clone();
        match cached {
            Some(tree) => tree,
            None => {
                let has_struct_tree_root = self
                    .catalog()
                    .ok()
                    .and_then(|cat| cat.as_dict().map(|d| d.contains_key("StructTreeRoot")))
                    .unwrap_or(false);
                let tree = if mark.marked || has_struct_tree_root {
                    self.structure_tree().ok().flatten().map(Arc::new)
                } else {
                    None
                };
                *self.structure_tree_cache.lock_or_recover() = Some(tree.clone());
                tree
            }
        }
    }

    /// Returns the document's structure tree whenever it is **available**,
    /// independent of `/MarkInfo /Suspects`.
    ///
    /// The `/Suspects` flag (§14.7.1) signals that the producer's *reading
    /// order* may be unreliable, so `struct_tree_trustworthy` rejects the
    /// tree for ordering. `/ActualText`, however, is content replacement
    /// (§14.9.4) and remains trustworthy: a producer that bothered to
    /// supply the replacement text for a glyph run is asserting what
    /// that run is *meant* to read as, regardless of whether sibling
    /// reading-order tags are reliable. This accessor lets the
    /// ActualText pipeline honour the producer's intent on Suspects=true
    /// documents while geometric reading order takes over the ordering
    /// problem.
    ///
    /// Shares `structure_tree_cache` with `struct_tree_trustworthy`, so
    /// both predicates cost a single cached parse.
    pub(crate) fn struct_tree_marked(&self) -> Option<Arc<crate::structure::StructTreeRoot>> {
        let cached = self.structure_tree_cache.lock_or_recover().clone();
        match cached {
            Some(tree) => tree,
            None => {
                let mark = self.mark_info().unwrap_or_default();
                let has_struct_tree_root = self
                    .catalog()
                    .ok()
                    .and_then(|cat| cat.as_dict().map(|d| d.contains_key("StructTreeRoot")))
                    .unwrap_or(false);
                let tree = if mark.marked || has_struct_tree_root {
                    self.structure_tree().ok().flatten().map(Arc::new)
                } else {
                    None
                };
                *self.structure_tree_cache.lock_or_recover() = Some(tree.clone());
                tree
            }
        }
    }

    /// Returns the cached [`ActualTextIndex`] for this document.
    ///
    /// Builds the index lazily on first call, then serves cached copies.
    /// Returns `None` for untagged documents and for tagged documents
    /// whose structure tree carries no `/ActualText`.
    ///
    /// Decoupled from `/MarkInfo /Suspects` — see [`struct_tree_marked`].
    pub(crate) fn actualtext_index(&self) -> Option<Arc<crate::structure::ActualTextIndex>> {
        if let Some(cached) = self.actualtext_index_cache.lock_or_recover().clone() {
            return cached;
        }
        let tree = self.struct_tree_marked();
        let built = tree.and_then(|t| {
            let idx = crate::structure::traversal::build_actualtext_index(&t);
            if idx.is_empty() { None } else { Some(Arc::new(idx)) }
        });
        *self.actualtext_index_cache.lock_or_recover() = Some(built.clone());
        built
    }

    /// Whether text extraction uses the Tagged-PDF *logical structure order* (a
    /// depth-first traversal of `/StructTreeRoot`) rather than geometric
    /// page-content order for this document.
    ///
    /// Returns `true` exactly when the document carries a trustworthy structure
    /// tree per ISO 32000-1:2008 §14.8.2.3.1 / §14.7.1: it is `/Marked` or the
    /// catalog references a `/StructTreeRoot`, the tree resolves non-empty, and
    /// `/MarkInfo /Suspects` is not `true`. When `false`, extraction falls back
    /// to geometric reading order. This is a read-only introspection accessor;
    /// it does not change extraction behaviour.
    pub fn prefers_structure_reading_order(&self) -> bool {
        self.struct_tree_trustworthy().is_some()
    }

    /// Find the document's default CMYK output-intent profile.
    ///
    /// Per ISO 32000-1:2008 §14.11.5, an `/OutputIntents` array in the
    /// catalog advertises the colour characteristics of the target
    /// output device. Each entry is a dictionary; the `DestOutputProfile`
    /// key (when present) references an ICC profile stream identifying
    /// the intended press / display calibration.
    ///
    /// This method returns the **first CMYK** `DestOutputProfile` it
    /// finds (N = 4) — the usual match for "here is how my CMYK ink
    /// should look" on PDF/X files. Callers can use it as a fallback
    /// profile for plain `/DeviceCMYK` images that lack their own ICC
    /// colour space.
    ///
    /// Returns `None` when no output intent exists, no CMYK entry is
    /// present, or the profile stream can't be parsed as ICC.
    pub fn output_intent_cmyk_profile(&self) -> Option<std::sync::Arc<crate::color::IccProfile>> {
        // Memoise the (potentially expensive) decode + parse: hot rendering
        // paths consult this accessor once per paint, and qcms / lcms2
        // header validation + LUT decode on a hundreds-of-KB profile is
        // not free. `Some(None)` means "checked once, no usable CMYK
        // OutputIntent"; a subsequent call must NOT re-walk the catalog. ~keep
        if let Some(cached) = self.output_intent_cmyk_profile_cache.lock_or_recover().as_ref() {
            return cached.clone();
        }
        let resolved = self.compute_output_intent_cmyk_profile();
        *self.output_intent_cmyk_profile_cache.lock_or_recover() = Some(resolved.clone());
        resolved
    }

    /// True when the document catalog declares an `/OutputIntents`
    /// array, regardless of whether the contained profile bytes
    /// successfully parse. Coupled with
    /// [`Self::output_intent_cmyk_profile`] returning `None`, this
    /// distinguishes "no OutputIntent requested" (acceptable silent
    /// fallback) from "OutputIntent requested but unusable" (degraded
    /// press output that callers should warn about). Tracks upstream
    /// issue upstream issue #712 on swallowed profile-parse
    /// diagnostics.
    pub fn has_output_intents_declaration(&self) -> bool {
        let Ok(catalog) = self.catalog() else {
            return false;
        };
        let Some(cat_dict) = catalog.as_dict() else {
            return false;
        };
        let Some(intents_obj) = cat_dict.get("OutputIntents") else {
            return false;
        };
        let intents_obj = match intents_obj {
            Object::Reference(r) => match self.load_object(*r) {
                Ok(o) => o,
                Err(_) => return false,
            },
            other => other.clone(),
        };
        matches!(intents_obj, Object::Array(_))
    }

    fn compute_output_intent_cmyk_profile(&self) -> Option<std::sync::Arc<crate::color::IccProfile>> {
        let catalog = self.catalog().ok()?;
        let cat_dict = catalog.as_dict()?;

        let intents_obj = cat_dict.get("OutputIntents")?;
        let intents_obj = match intents_obj {
            Object::Reference(r) => self.load_object(*r).ok()?,
            _ => intents_obj.clone(),
        };
        let intents_arr = match &intents_obj {
            Object::Array(a) => a.clone(),
            _ => return None,
        };

        for entry in intents_arr {
            let entry = match entry {
                // Skip a broken entry rather than aborting the whole array (§7.3.10). ~keep
                Object::Reference(r) => match self.load_object(r) {
                    Ok(o) => o,
                    Err(error) => {
                        crate::error::trace_recovery("load_output_intent_entry", &error);
                        continue;
                    }
                },
                other => other,
            };
            let entry_dict = match entry.as_dict() {
                Some(d) => d.clone(),
                None => continue,
            };
            let profile_obj = match entry_dict.get("DestOutputProfile") {
                Some(p) => p.clone(),
                None => continue,
            };
            let profile_stream = match profile_obj {
                Object::Reference(r) => match self.load_object(r) {
                    Ok(o) => o,
                    Err(error) => {
                        crate::error::trace_recovery("load_output_intent_profile", &error);
                        continue;
                    }
                },
                other => other,
            };

            let Object::Stream { dict, .. } = &profile_stream else {
                continue;
            };
            let n = match dict.get("N").and_then(|o| o.as_integer()) {
                Some(4) => 4u8, // only CMYK; ignore RGB/Gray output intents here ~keep
                _ => continue,
            };
            let bytes = match profile_stream.decode_stream_data() {
                Ok(b) => b,
                Err(error) => {
                    crate::error::trace_recovery("decode_output_intent_profile", &error);
                    continue;
                }
            };
            match crate::color::IccProfile::parse(bytes, n) {
                Some(prof) => return Some(std::sync::Arc::new(prof)),
                None => tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "parse_output_intent_profile",
                    error_code = "invalid_icc_profile",
                    "skipping invalid CMYK output profile"
                ),
            }
        }
        None
    }

    /// Get the MarkInfo dictionary from the document catalog.
    ///
    /// The MarkInfo dictionary indicates whether the document conforms to
    /// Tagged PDF conventions and whether the structure tree might contain
    /// suspect (unreliable) content.
    ///
    /// Per ISO 32000-1:2008 Section 14.7.1, the MarkInfo dictionary contains:
    /// - `/Marked` - Whether the document conforms to Tagged PDF conventions
    /// - `/Suspects` - Whether the document contains suspect content
    /// - `/UserProperties` - Whether the document uses user properties
    ///
    /// # Returns
    ///
    /// Returns `MarkInfo` with the parsed values, or default values if
    /// the MarkInfo dictionary is not present.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let mark_info = doc.mark_info()?;
    /// if mark_info.is_structure_reliable() {
    ///     println!("Structure tree can be trusted for reading order");
    /// } else if mark_info.suspects {
    ///     println!("Structure tree may contain unreliable content");
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn mark_info(&self) -> Result<crate::structure::MarkInfo> {
        let catalog = self.catalog()?;
        let catalog_dict = match catalog.as_dict() {
            Some(d) => d,
            None => return Ok(crate::structure::MarkInfo::default()),
        };

        let mark_info_obj = match catalog_dict.get("MarkInfo") {
            Some(obj) => obj,
            None => return Ok(crate::structure::MarkInfo::default()),
        };

        let mark_info_obj = if let Some(r) = mark_info_obj.as_reference() {
            self.load_object(r)?
        } else {
            mark_info_obj.clone()
        };

        let mark_info_dict = match mark_info_obj.as_dict() {
            Some(d) => d,
            None => return Ok(crate::structure::MarkInfo::default()),
        };

        let marked = mark_info_dict
            .get("Marked")
            .and_then(|o: &crate::object::Object| o.as_bool())
            .unwrap_or(false);

        let suspects = mark_info_dict
            .get("Suspects")
            .and_then(|o: &crate::object::Object| o.as_bool())
            .unwrap_or(false);

        let user_properties = mark_info_dict
            .get("UserProperties")
            .and_then(|o: &crate::object::Object| o.as_bool())
            .unwrap_or(false);

        Ok(crate::structure::MarkInfo {
            marked,
            suspects,
            user_properties,
        })
    }
}
