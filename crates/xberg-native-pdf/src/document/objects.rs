//! Indirect-object loading, xref recovery, and reference resolution.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Get the PDF version.
    ///
    /// Returns a tuple (major, minor) representing the PDF version.
    /// For example, PDF 1.7 returns (1, 7).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let (major, minor) = doc.version();
    /// println!("PDF version: {}.{}", major, minor);
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn version(&self) -> (u8, u8) {
        self.version
    }

    /// Get a reference to the trailer dictionary.
    ///
    /// The trailer dictionary contains important document metadata including:
    /// - /Root: Reference to the catalog dictionary
    /// - /Info: Reference to the document info dictionary (optional)
    /// - /Size: Number of entries in the cross-reference table
    /// - /Encrypt: Encryption dictionary (if encrypted)
    /// - /ID: File identifier array
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let trailer = doc.trailer();
    /// if let Some(dict) = trailer.as_dict() {
    ///     if let Some(info_ref) = dict.get("Info") {
    ///         println!("Document has an Info dictionary");
    ///     }
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn trailer(&self) -> &Object {
        &self.trailer
    }

    /// Return every object ID known to this document.
    ///
    /// Unions the cross-reference table with any object IDs that were
    /// recovered from compressed object streams (which may not have an
    /// explicit xref entry). The result is sorted and deduplicated so
    /// callers can iterate once and write each object exactly once.
    ///
    /// Used by `DocumentEditor::write_full_to_writer` to sweep any
    /// objects that were not reached during the shallow page-tree
    /// traversal (e.g. embedded font sub-objects such as
    /// `DescendantFonts`, `FontFile2`, `ToUnicode`, `FontDescriptor`).
    pub fn all_object_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.xref.all_object_numbers().collect();
        for r in self.object_cache.lock_or_recover().keys() {
            ids.push(r.id);
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Return references to every leaf page, in document order, with a single
    /// page-tree traversal.
    ///
    /// Replaces the O(n²) pattern of calling [`get_page_ref`] in a 0..n loop:
    /// each `get_page_ref(i)` walks the tree from the root and stops at the
    /// i-th leaf, so collecting all n refs walks 1+2+...+n nodes.
    ///
    /// Optimised for the common flat-tree case: when a `Pages` node's
    /// `Count` matches `Kids.len()`, every kid is a leaf and we can take
    /// the references straight from the array without loading each leaf.
    /// Only when the tree is multi-level do we recurse and load child nodes.
    pub(crate) fn all_page_refs(&self) -> Result<Vec<ObjectRef>> {
        let catalog = self.catalog()?;
        let catalog_dict = catalog.as_dict().ok_or_else(|| Error::InvalidObjectType {
            expected: "Dictionary".to_string(),
            found: catalog.type_name().to_string(),
        })?;
        let pages_ref = catalog_dict
            .get("Pages")
            .and_then(|p| p.as_reference())
            .ok_or_else(|| Error::InvalidPdf("Catalog missing /Pages entry".to_string()))?;

        let mut out: Vec<ObjectRef> = Vec::new();
        let mut visited: HashSet<ObjectRef> = HashSet::new();
        self.collect_page_refs(pages_ref, &mut out, &mut visited)?;
        Ok(out)
    }

    fn collect_page_refs(
        &self,
        node_ref: ObjectRef,
        out: &mut Vec<ObjectRef>,
        visited: &mut HashSet<ObjectRef>,
    ) -> Result<()> {
        if !visited.insert(node_ref) {
            return Ok(());
        }
        let node = self.load_object(node_ref)?;
        let dict = match node.as_dict() {
            Some(d) => d,
            None => return Ok(()),
        };

        let kids = match dict.get("Kids").and_then(|k| k.as_array()) {
            Some(k) => k,
            None => {
                out.push(node_ref);
                return Ok(());
            }
        };

        // Fast path: flat subtree — every kid is a leaf when /Count == kids.len(). ~keep
        let count = dict.get("Count").and_then(|c| c.as_integer()).unwrap_or(-1);
        if count >= 0 && (count as usize) == kids.len() {
            for kid in kids {
                if let Some(kid_ref) = kid.as_reference() {
                    out.push(kid_ref);
                }
            }
            return Ok(());
        }

        for kid in kids {
            if let Some(kid_ref) = kid.as_reference() {
                self.collect_page_refs(kid_ref, out, visited)?;
            }
        }
        Ok(())
    }

    /// Up to `len` bytes of the document starting at `offset`, or `None` when
    /// `offset` is past the end. Every read of the file body goes through an
    /// absolute offset into the document's own bytes, so no two callers share a
    /// cursor and no read depends on what another thread did first. ~keep
    pub(super) fn bytes_at(&self, offset: u64, len: usize) -> Option<&[u8]> {
        let start = usize::try_from(offset).ok()?;
        let bytes = self.source_bytes.get(start..)?;
        Some(&bytes[..len.min(bytes.len())])
    }

    /// Scan the file to find an object by its header.
    ///
    /// This is a fallback method used when an object is not in the xref table
    /// but is referenced by critical structures (like Pages from Catalog).
    /// Some PDFs have incomplete xref tables that are missing entries for
    /// objects that actually exist in the file.
    fn scan_for_object(&self, obj_ref: ObjectRef) -> Result<u64> {
        {
            let scan_cache = self.scanned_object_offsets.lock_or_recover();
            if let Some(offsets) = scan_cache.as_ref() {
                if let Some(&offset) = offsets.get(&obj_ref.id) {
                    return Ok(offset);
                }
                return Err(Error::ObjectNotFound(obj_ref.id, obj_ref.generation));
            }
        }

        tracing::info!(target: LOG_TARGET,
            "Building object offset map from file scan (triggered by object {} {})",
            obj_ref.id,
            obj_ref.generation
        );

        let content: &[u8] = &self.source_bytes;

        let mut offsets = HashMap::new();

        let mut pos = 0;
        while pos < content.len() {
            let valid_start = pos == 0 || content[pos - 1] == b'\n' || content[pos - 1] == b'\r';
            if !valid_start || !content[pos].is_ascii_digit() {
                pos += 1;
                continue;
            }

            let start = pos;
            while pos < content.len() && content[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos >= content.len() || content[pos] != b' ' {
                continue;
            }
            let obj_num_str = std::str::from_utf8(&content[start..pos]).unwrap_or("");
            let obj_num: u32 = match obj_num_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            pos += 1;

            let gen_start = pos;
            while pos < content.len() && content[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos >= content.len() || content[pos] != b' ' {
                continue;
            }
            let _gen_str = std::str::from_utf8(&content[gen_start..pos]).unwrap_or("");

            pos += 1;

            if pos + 3 <= content.len() && &content[pos..pos + 3] == b"obj" {
                let after_obj = pos + 3;
                let valid_end = after_obj >= content.len() || {
                    let c = content[after_obj];
                    c == b'\n' || c == b'\r' || c == b' ' || c == b'\t' || c == b'<'
                };
                if valid_end {
                    offsets.entry(obj_num).or_insert(start as u64);
                    pos = after_obj;
                    continue;
                }
            }
            // Reset pos to just after the start to avoid infinite loop ~keep
            pos = start + 1;
        }

        tracing::debug!(target: LOG_TARGET, "File scan found {} objects", offsets.len());

        let result = offsets.get(&obj_ref.id).copied();
        *self.scanned_object_offsets.lock_or_recover() = Some(offsets);

        match result {
            Some(offset) => Ok(offset),
            None => Err(Error::ObjectNotFound(obj_ref.id, obj_ref.generation)),
        }
    }

    /// One-time sweep over every known object stream (`/Type /ObjStm`),
    /// used to recover from xref tables that mis-mark compressed objects as
    /// free.
    ///
    /// Some PDF producers emit an xref where a compressed object's slot is
    /// type 0 (free) instead of type 2 (compressed → stream#). The object
    /// is physically stored inside an `ObjStm`, but `scan_for_object` can't
    /// find it because it has no standalone `N G obj` marker in the body.
    ///
    /// The recovery: iterate every uncompressed candidate, peek at the
    /// dictionary, and for those that are `/Type /ObjStm`, parse the stream
    /// and cache everything inside (overwriting any stale `Object::Null`
    /// entries from earlier free-entry short-circuits).
    ///
    /// Runs at most once per document — guarded by `objstm_recovery_done`.
    /// Cost is amortised across every recovered object.
    pub(super) fn recover_from_object_streams(&self) {
        use crate::objstm::parse_object_stream_with_decryption_outcome;

        {
            let done = self.objstm_recovery_done.lock_or_recover();
            if *done {
                return;
            }
        }

        tracing::debug!(target: LOG_TARGET, "Sweeping object streams to recover xref-flagged-free objects");

        // Find ObjStm candidates by raw pattern search in the file body.
        //
        // Why not iterate xref entries here: the xref is precisely what we
        // don't trust in this recovery path — its offsets may be wrong
        // its type tags may be lying about what each slot contains. A raw
        // search for `N G obj ... /Type /ObjStm` finds every object stream
        // the producer actually wrote, independent of how the xref
        // describes them.
        //
        // Only flip `objstm_recovery_done` after we finish the scan+parse
        // pass, so a later retry can still attempt recovery. ~keep
        let candidates = find_objstm_candidates(&self.source_bytes);

        let mut objstms_found = 0usize;
        let mut recovered = 0usize;
        for (stream_obj_num, offset) in &candidates {
            let stream_ref = ObjectRef::new(*stream_obj_num, 0);
            let stream_obj = match self.load_uncompressed_object(stream_ref, *offset) {
                Ok(obj) => obj,
                Err(_) => continue,
            };

            let is_objstm = stream_obj
                .as_dict()
                .and_then(|d| d.get("Type"))
                .and_then(|t| t.as_name())
                .is_some_and(|n| n == "ObjStm");
            if !is_objstm {
                continue;
            }
            objstms_found += 1;

            // Parse the stream body. ISO 32000-2:2020 §7.6.3 says ObjStm
            // shall NOT be individually encrypted, so skip decryption here
            // — mirrors the default branch in `load_compressed_object`. ~keep
            let outcome = match parse_object_stream_with_decryption_outcome(&stream_obj, None, 0, 0) {
                Ok(outcome) => outcome,
                Err(error) => {
                    tracing::warn!(
                        target: crate::LOG_TARGET_ROOT,
                        operation = "recover_object_stream",
                        stream_object_id = stream_obj_num,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "skipping malformed object stream during recovery"
                    );
                    continue;
                }
            };
            self.trace_object_stream_recovery_once(*stream_obj_num, &outcome);
            let objects_map = Arc::new(outcome.objects);
            self.object_stream_cache
                .lock_or_recover()
                .insert(ObjectRef::new(*stream_obj_num, 0), Arc::clone(&objects_map));

            let mut cache = self.object_cache.lock_or_recover();
            for (obj_num, object) in objects_map.iter() {
                let cache_ref = ObjectRef::new(*obj_num, 0);
                // Only overwrite entries we'd otherwise have resolved to
                // Null (the free-entry short-circuit caches Null). Never
                // clobber a real object loaded through the normal path. ~keep
                match cache.get(&cache_ref) {
                    Some(Object::Null) | None => {
                        cache.insert(cache_ref, object.clone());
                        recovered += 1;
                    }
                    _ => {}
                }
            }
        }

        tracing::debug!(target: LOG_TARGET,
            "Object-stream recovery sweep: {} candidate positions, {} ObjStms, {} objects cached",
            candidates.len(),
            objstms_found,
            recovered
        );

        *self.objstm_recovery_done.lock_or_recover() = true;
    }

    /// Load an object by its reference.
    ///
    /// This function:
    /// 1. Checks the object cache first
    /// 2. If not cached, looks up the byte offset in the xref table
    /// 3. Seeks to that offset and parses the object
    /// 4. Caches the result for future access
    /// 5. If object not in xref but is critical, scans file for it
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The object reference is not in the xref table and file scan fails
    /// - The object is not in use (free object)
    /// - Seeking to the object offset fails
    /// - Parsing the object fails
    /// - A circular reference is detected
    /// - The recursion depth limit is exceeded
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # use xberg_native_pdf::object::ObjectRef;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let obj_ref = ObjectRef::new(1, 0);
    /// let obj = doc.load_object(obj_ref)?;
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn load_object(&self, obj_ref: ObjectRef) -> Result<Object> {
        tracing::trace!(target: LOG_TARGET,
            object_id = obj_ref.id,
            generation = obj_ref.generation,
            "loading object"
        );

        // Check recursion depth (per-thread counter; no lock needed) ~keep
        {
            let depth = RECURSION_DEPTH.with(|d| *d.borrow());
            if depth >= MAX_RECURSION_DEPTH {
                tracing::error!(target: LOG_TARGET,
                    "Recursion depth limit exceeded ({}) while loading object {} gen {}",
                    MAX_RECURSION_DEPTH,
                    obj_ref.id,
                    obj_ref.generation
                );
                return Err(Error::RecursionLimitExceeded(MAX_RECURSION_DEPTH));
            }
        }

        // Check for circular references (per-thread stack; concurrent threads
        // resolving the same object do NOT appear as a false cycle) ~keep
        if RESOLVING_STACK.with(|s| s.borrow().contains(&obj_ref)) {
            tracing::error!(target: LOG_TARGET,
                "Circular reference detected for object {} gen {} (depth: {})",
                obj_ref.id,
                obj_ref.generation,
                RECURSION_DEPTH.with(|d| *d.borrow())
            );
            return Err(Error::CircularReference(obj_ref));
        }

        // Check cache first (warm path: fully parallel, no serialization). ~keep
        let cached_opt = self.object_cache.lock_or_recover().get(&obj_ref).cloned();
        if let Some(cached) = cached_opt {
            return Ok(cached);
        }

        let entry = match self.xref.get(obj_ref.id) {
            Some(entry) => entry,
            None => {
                // Object not in xref table - try scanning the file as fallback
                // This handles PDFs with incomplete/corrupted xref tables ~keep
                self.recovery
                    .xref_misses
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let available: Vec<u32> = self.xref.entries.keys().copied().take(20).collect();
                tracing::trace!(target: LOG_TARGET,
                    "Object {} not in xref table. Total entries: {}. First 20 objects: {:?}",
                    obj_ref.id,
                    self.xref.len(),
                    available
                );

                match self.scan_for_object(obj_ref) {
                    Ok(offset) => {
                        self.recovery
                            .file_scan_recoveries
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        tracing::trace!(target: LOG_TARGET,
                            "Successfully found object {} via file scan at offset {}",
                            obj_ref.id,
                            offset
                        );

                        RESOLVING_STACK.with(|s| {
                            s.borrow_mut().insert(obj_ref);
                        });
                        RECURSION_DEPTH.with(|d| *d.borrow_mut() += 1);

                        let result = self.load_uncompressed_object(obj_ref, offset);

                        RECURSION_DEPTH.with(|d| *d.borrow_mut() -= 1);
                        RESOLVING_STACK.with(|s| {
                            s.borrow_mut().remove(&obj_ref);
                        });

                        return result;
                    }
                    Err(_) => {
                        // PDF Spec §7.3.10: missing object reference "shall be treated as null"
                        // ~keep
                        self.recovery
                            .objects_treated_as_null
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        tracing::trace!(target: LOG_TARGET,
                            "Object {} gen {} not found (xref + file scan failed), treating as Null per §7.3.10",
                            obj_ref.id,
                            obj_ref.generation
                        );
                        self.object_cache.lock_or_recover().insert(obj_ref, Object::Null);
                        return Ok(Object::Null);
                    }
                }
            }
        };

        tracing::trace!(target: LOG_TARGET,
            "  → Found in xref: type={:?}, offset={}, gen={}, in_use={}",
            entry.entry_type,
            entry.offset,
            entry.generation,
            entry.in_use
        );

        if !entry.in_use {
            tracing::debug!(target: LOG_TARGET,
                "Object {} is marked as free (not in use). This may be due to a corrupted xref table.",
                obj_ref.id
            );

            // xref flags the object free, but this may be xref corruption
            // rather than an actual deletion. Run two recovery paths before
            // falling back to §7.3.10's null. The branches below apply
            // uniformly for all object ids (critical low-numbered catalog
            // objects and page objects in the thousands); previously low
            // ids took a separate "fall through to loading logic" path
            // that silently hit the Free arm of the entry_type match
            // still ended up Null.
            //
            // Recovery path 1 — standalone `N G obj` marker in the file
            // body. `scan_for_object` builds a whole-file offset map once
            // per document and caches it, so the amortised cost is a
            // single O(filesize) pass no matter how many free-marked
            // objects we probe. ~keep
            if let Ok(scanned_offset) = self.scan_for_object(obj_ref) {
                tracing::warn!(target: LOG_TARGET,
                    "Object {} marked free in xref but found in file scan at offset {}; recovering",
                    obj_ref.id,
                    scanned_offset
                );
                RESOLVING_STACK.with(|s| {
                    s.borrow_mut().insert(obj_ref);
                });
                RECURSION_DEPTH.with(|d| *d.borrow_mut() += 1);
                let result = self.load_uncompressed_object(obj_ref, scanned_offset);
                RECURSION_DEPTH.with(|d| *d.borrow_mut() -= 1);
                RESOLVING_STACK.with(|s| {
                    s.borrow_mut().remove(&obj_ref);
                });
                return result;
            }

            // Recovery path 2 — the object may be compressed inside a
            // `/Type /ObjStm`. Real-world producers have been seen to
            // mis-flag every compressed object's xref slot as free, so
            // sweep the object streams once and recheck the cache. ~keep
            self.recover_from_object_streams();
            if let Some(obj) = self.object_cache.lock_or_recover().get(&obj_ref).cloned()
                && !matches!(obj, Object::Null)
            {
                tracing::warn!(target: LOG_TARGET, "Object {} recovered from object-stream sweep", obj_ref.id);
                return Ok(obj);
            }

            // PDF Spec §7.3.10: free object treated as null ~keep
            tracing::warn!(target: LOG_TARGET,
                "Free object {} gen {}, treating as Null per §7.3.10",
                obj_ref.id,
                obj_ref.generation
            );
            self.object_cache.lock_or_recover().insert(obj_ref, Object::Null);
            return Ok(Object::Null);
        }

        RESOLVING_STACK.with(|s| {
            s.borrow_mut().insert(obj_ref);
        });
        RECURSION_DEPTH.with(|d| *d.borrow_mut() += 1);

        use crate::xref::XRefEntryType;
        let entry_type = entry.entry_type;
        let entry_offset = entry.offset;
        let entry_gen = entry.generation;
        let result = match entry_type {
            XRefEntryType::Compressed => {
                // Type 2 entry: object is in an object stream
                // entry.offset = stream object number
                // entry.generation = index within stream ~keep
                tracing::trace!(target: LOG_TARGET, "  → Compressed object in stream {}, index {}", entry_offset, entry_gen);
                self.load_compressed_object(obj_ref, entry_offset as u32, entry_gen)
            }
            XRefEntryType::Uncompressed => {
                tracing::trace!(target: LOG_TARGET, "  → Uncompressed object at offset {}", entry_offset);
                self.load_uncompressed_object(obj_ref, entry_offset)
            }
            XRefEntryType::Free => {
                // Free object - shouldn't happen since we check in_use above
                // PDF Spec §7.3.10: treat as null ~keep
                tracing::warn!(target: LOG_TARGET,
                    "Object {} has type Free despite in_use=true, treating as Null",
                    obj_ref.id
                );
                self.object_cache.lock_or_recover().insert(obj_ref, Object::Null);
                Ok(Object::Null)
            }
        };

        RECURSION_DEPTH.with(|d| *d.borrow_mut() -= 1);
        RESOLVING_STACK.with(|s| {
            s.borrow_mut().remove(&obj_ref);
        });

        result
    }

    /// Resolve references within an object recursively.
    ///
    /// This utility method resolves indirect references within an object,
    /// handling nested dictionaries and arrays up to a specified depth.
    /// Useful for processing complex PDF structures where properties
    /// may be stored as indirect references.
    ///
    /// # Arguments
    ///
    /// * `obj` - The object to resolve references within
    /// * `max_depth` - Maximum recursion depth to prevent infinite loops
    ///
    /// # Returns
    ///
    /// The object with all references resolved up to max_depth levels.
    /// If a reference cannot be resolved, it is left as-is.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// # let obj = doc.catalog()?;
    /// // Resolve all references in a dictionary up to 3 levels deep
    /// let resolved = doc.resolve_references(&obj, 3)?;
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn resolve_references(&self, obj: &Object, max_depth: usize) -> Result<Object> {
        if max_depth == 0 {
            return Ok(obj.clone());
        }

        match obj {
            Object::Reference(obj_ref) => match self.load_object(*obj_ref) {
                Ok(resolved) => self.resolve_references(&resolved, max_depth - 1),
                Err(error) => {
                    tracing::warn!(target: LOG_TARGET,
                        operation = "resolve_reference",
                        object_id = obj_ref.id,
                        generation = obj_ref.generation,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to resolve reference"
                    );
                    Ok(obj.clone())
                }
            },

            Object::Dictionary(dict) => {
                let mut resolved_dict = std::collections::HashMap::new();
                for (key, value) in dict.iter() {
                    let resolved_value = self.resolve_references(value, max_depth - 1)?;
                    resolved_dict.insert(key.clone(), resolved_value);
                }
                Ok(Object::Dictionary(resolved_dict))
            }

            Object::Array(arr) => {
                let resolved_arr: Result<Vec<Object>> = arr
                    .iter()
                    .map(|item| self.resolve_references(item, max_depth - 1))
                    .collect();
                Ok(Object::Array(resolved_arr?))
            }

            _ => Ok(obj.clone()),
        }
    }

    /// Resolve a single-level indirect reference (PDF spec §7.3.10).
    ///
    /// If `obj` is `Object::Reference(...)`, loads and returns the target object.
    /// For any other object type, returns a clone unchanged. This is the
    /// canonical way to handle "any value may be a direct or indirect reference"
    /// throughout the parser.
    pub(super) fn resolve_obj_ref(&self, obj: &Object) -> Object {
        if let Some(obj_ref) = obj.as_reference() {
            match self.load_object(obj_ref) {
                Ok(resolved) => resolved,
                Err(error) => {
                    tracing::warn!(target: LOG_TARGET,
                        operation = "resolve_indirect_reference",
                        object_id = obj_ref.id,
                        generation = obj_ref.generation,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to resolve indirect reference"
                    );
                    obj.clone()
                }
            }
        } else {
            obj.clone()
        }
    }

    /// Peek at an XObject's /Subtype without loading the full object.
    /// Returns true if the XObject is a Form XObject, false if Image or unknown.
    /// For compressed objects or on any error, returns true (conservative — will load fully).
    pub fn is_form_xobject(&self, obj_ref: ObjectRef) -> bool {
        {
            if self.image_xobject_cache.lock_or_recover().contains(&obj_ref) {
                return false;
            }
        }

        let cached_opt = self.object_cache.lock_or_recover().get(&obj_ref).cloned();
        if let Some(cached) = cached_opt {
            let is_form = cached
                .as_dict()
                .and_then(|d| d.get("Subtype"))
                .and_then(|s| s.as_name())
                == Some("Form");
            if !is_form {
                self.image_xobject_cache.lock_or_recover().insert(obj_ref);
            }
            return is_form;
        }

        let entry = match self.xref.get(obj_ref.id) {
            Some(e) => e,
            None => return true,
        };

        // Only peek uncompressed objects — compressed ones require full load ~keep
        use crate::xref::XRefEntryType;
        if entry.entry_type != XRefEntryType::Uncompressed || !entry.in_use {
            return true;
        }

        let Some(data) = self.bytes_at(entry.offset, 1024) else {
            return true;
        };

        if let Some(pos) = data.windows(8).position(|w| w == b"/Subtype") {
            let after = &data[pos + 8..];
            let trimmed = after
                .iter()
                .position(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n');
            if let Some(start) = trimmed {
                let name_data = &after[start..];
                if name_data.starts_with(b"/Form") {
                    return true;
                }
                self.image_xobject_cache.lock_or_recover().insert(obj_ref);
                return false;
            }
        }

        true
    }

    /// Load an uncompressed object (Type 1 xref entry).
    fn load_uncompressed_object(&self, obj_ref: ObjectRef, offset: u64) -> Result<Object> {
        self.load_uncompressed_object_impl(obj_ref, offset, false)
    }

    /// Recursively decrypt every `Object::String` inside `obj` using the
    /// per-object key derived from `obj_num`/`gen_num`. Streams are left
    /// untouched — they are decrypted lazily at read time through
    /// `decode_stream_with_encryption`. The `/Encrypt` dictionary itself
    /// must never be passed to this function; its strings are key material,
    /// not ciphertext.
    ///
    /// Per ISO 32000-1:2008 §7.6.2, strings inside encrypted-document
    /// objects are individually encrypted with the standard encryption
    /// algorithm. Parsed string tokens hold raw ciphertext and must be
    /// decrypted before downstream consumers (widget text, form field
    /// values, outlines, document info) can read them.
    fn decrypt_strings_in_object(handler: &EncryptionHandler, obj: &mut Object, obj_num: u32, gen_num: u32) {
        // A failure here leaves ciphertext in place for downstream readers, so
        // it must be visible — but this walk recurses through every array and
        // dictionary, so logging per string would emit thousands of identical
        // events for one broken key. Count instead, and report once per
        // object. ~keep
        let mut failed = 0usize;
        Self::decrypt_strings_in_object_inner(handler, obj, obj_num, gen_num, &mut failed);
        if failed > 0 {
            tracing::warn!(target: LOG_TARGET,
                object_id = obj_num,
                generation = gen_num,
                count = failed,
                "string decryption failed; ciphertext left in place"
            );
        }
    }

    fn decrypt_strings_in_object_inner(
        handler: &EncryptionHandler,
        obj: &mut Object,
        obj_num: u32,
        gen_num: u32,
        failed: &mut usize,
    ) {
        match obj {
            Object::String(bytes) => match handler.decrypt_string(bytes, obj_num, gen_num) {
                Ok(decrypted) => *bytes = decrypted,
                Err(e) => {
                    *failed += 1;
                    tracing::trace!(target: LOG_TARGET,
                        object_id = obj_num,
                        generation = gen_num,
                        "string decryption failed: {}",
                        e
                    );
                }
            },
            Object::Array(items) => {
                for item in items {
                    Self::decrypt_strings_in_object_inner(handler, item, obj_num, gen_num, failed);
                }
            }
            Object::Dictionary(dict) => {
                for value in dict.values_mut() {
                    Self::decrypt_strings_in_object_inner(handler, value, obj_num, gen_num, failed);
                }
            }
            Object::Stream { dict, .. } => {
                // Stream *data* is decrypted separately in
                // `decode_stream_with_encryption`. Its dict may still
                // contain encrypted strings (e.g., /Metadata). ~keep
                for value in dict.values_mut() {
                    Self::decrypt_strings_in_object_inner(handler, value, obj_num, gen_num, failed);
                }
            }
            _ => {}
        }
    }

    /// Implementation with recursion guard to prevent infinite loops.
    pub(super) fn load_uncompressed_object_impl(
        &self,
        obj_ref: ObjectRef,
        offset: u64,
        already_corrected: bool,
    ) -> Result<Object> {
        // The header and the body are read through ONE cursor, private to this
        // call, over the document's own bytes. The body read continues from where
        // the header read stopped, so the two must not be separable: a cursor any
        // other reader can move between them reads the body of a different object.
        // ~keep
        // Cap a single header-line read so a CR-terminated PDF (legal per ISO
        // 32000-1) whose next LF is far away — or absent for the rest of the
        // file — cannot make one `read_until` call allocate without limit
        // before any size check runs. MAX_BYTES on the body loop further down
        // does not cover this: it only checks *between* calls, and both reads
        // in this header phase had no cap of their own at all. A well-formed
        // header is a handful of bytes, so this is comfortably above any
        // legitimate header line while still bounding the pathological case.
        // ~keep
        const MAX_HEADER_LINE_BYTES: u64 = 64 * 1024; // 64 KB safety limit per header line ~keep
        let mut cursor = Cursor::new(self.source_bytes.as_slice());
        let (header_bytes, full_header) = {
            let reader = &mut cursor;
            reader.seek(SeekFrom::Start(offset))?;

            let mut header_bytes = Vec::new();
            let bytes_read = reader
                .by_ref()
                .take(MAX_HEADER_LINE_BYTES)
                .read_until(b'\n', &mut header_bytes)?;

            if bytes_read == 0 {
                let msg = format!("Unexpected EOF while reading object {} header", obj_ref.id);
                tracing::warn!(target: LOG_TARGET, "{}", msg);
                // also push into structured sink so
                // callers can retrieve as data via flatten_warnings. ~keep
                self.push_structured_warning(crate::extractors::warnings::Warning {
                    category: crate::extractors::warnings::WarningCategory::EofPremature,
                    page: None,
                    message: msg,
                    spec_section: Some("7.5"),
                });
                return Err(Error::UnexpectedEof);
            }

            let line = String::from_utf8_lossy(&header_bytes);

            // Handle multi-line object headers ~keep
            let mut full_header = line.to_string();
            let max_header_lines = 5;
            let mut lines_read = 1;

            while !has_standalone_obj_keyword(full_header.as_bytes()) && lines_read < max_header_lines {
                let mut next_bytes = Vec::new();
                // Same cap as the first header-line read above: without it, a
                // CR-terminated stream with no standalone "obj" keyword would
                // just move the unbounded single-call read from the first
                // line to this continuation read instead of bounding it. ~keep
                let next_read = reader
                    .by_ref()
                    .take(MAX_HEADER_LINE_BYTES)
                    .read_until(b'\n', &mut next_bytes)?;
                if next_read == 0 {
                    break;
                }
                let next_line = String::from_utf8_lossy(&next_bytes);
                full_header.push(' ');
                full_header.push_str(&next_line);
                lines_read += 1;
            }
            (header_bytes, full_header)
        };

        let parts: Vec<&str> = full_header.split_whitespace().collect();

        let obj_pos = parts
            .iter()
            .position(|&p| p == "obj" || (p.starts_with("obj") && !p.starts_with("endobj")));

        let obj_pos = match obj_pos {
            Some(pos) if pos >= 2 => pos,
            _ => {
                // Only try backwards search once to prevent infinite recursion ~keep
                if !already_corrected {
                    // xref offset might be incorrect (pointing to object body instead of header)
                    // Try searching backwards for the object header ~keep
                    tracing::warn!(target: LOG_TARGET,
                        "No object header at offset {}, searching backwards for object {} {} obj",
                        offset,
                        obj_ref.id,
                        obj_ref.generation
                    );

                    if let Ok(corrected_offset) = self.find_object_header_backwards(obj_ref, offset) {
                        tracing::warn!(target: LOG_TARGET,
                            "Found object header at offset {} (xref said {})",
                            corrected_offset,
                            offset
                        );
                        return self.load_uncompressed_object_impl(obj_ref, corrected_offset, true);
                    }
                }

                tracing::warn!(target: LOG_TARGET,
                    operation = "load_uncompressed_object",
                    error_code = "malformed_object_header",
                    error_offset = offset,
                    object_id = obj_ref.id,
                    generation = obj_ref.generation,
                    "PDF object recovery failed"
                );
                return Err(Error::ParseError {
                    offset: offset as usize,
                    reason: format!("Expected object header, found: {}", full_header.trim()),
                });
            }
        };

        let _obj_pos = obj_pos;

        // Parse the object number and generation from header. If either
        // fails to parse as a number, the xref-reported offset is pointing
        // into the middle of a previous object's tail (e.g. xref says 12345
        // but the real `N G obj` header starts at 12348 because three bytes
        // of CR/LF/terminator got mis-accounted for by the producer — a
        // pattern seen in the wild). Fall back to the whole-file scan
        // cache: if scan recorded a different offset for this id, retry
        // from there before giving up. ~keep
        let obj_num_parsed = parts[0].parse::<u32>();
        let gen_num_parsed = parts[1].parse::<u16>();
        if !already_corrected
            && (obj_num_parsed.is_err() || gen_num_parsed.is_err())
            && let Ok(scan_offset) = self.scan_for_object(obj_ref)
            && scan_offset != offset
        {
            tracing::warn!(target: LOG_TARGET,
                operation = "load_uncompressed_object",
                error_code = "invalid_object_header_number",
                error_offset = offset,
                recovered_offset = scan_offset,
                object_id = obj_ref.id,
                generation = obj_ref.generation,
                "retrying PDF object load at scan-reported offset"
            );
            return self.load_uncompressed_object_impl(obj_ref, scan_offset, true);
        }
        let obj_num: u32 = obj_num_parsed.map_err(|_| Error::ParseError {
            offset: offset as usize,
            reason: format!("Invalid object number in header: {}", parts[0]),
        })?;
        let gen_num: u16 = gen_num_parsed.map_err(|_| Error::ParseError {
            offset: offset as usize,
            reason: format!("Invalid generation number in header: {}", parts[1]),
        })?;

        // Verify object reference matches (warn but don't fail on mismatch) ~keep
        if obj_num != obj_ref.id || gen_num != obj_ref.generation {
            tracing::warn!(target: LOG_TARGET,
                "Object reference mismatch at offset {}: expected {} {} obj, found {} {} obj",
                offset,
                obj_ref.id,
                obj_ref.generation,
                obj_num,
                gen_num
            );
        }

        // Check if there's content after "obj" on the same line
        // Some PDFs have "N G obj\n<<..." while others have "N G obj<<..." on one line ~keep
        let mut data = Vec::new();

        if let Some(obj_keyword_pos) = header_bytes.windows(3).position(|w| w == b"obj") {
            let after_obj_pos = obj_keyword_pos + 3;

            let mut content_start = after_obj_pos;
            while content_start < header_bytes.len()
                && (header_bytes[content_start] == b' '
                    || header_bytes[content_start] == b'\t'
                    || header_bytes[content_start] == b'\r')
            {
                content_start += 1;
            }

            // If there's a newline, skip it (normal case: "N G obj\n")
            // If there's content (like "<<"), include it (malformed case: "N G obj<<...") ~keep
            if content_start < header_bytes.len() && header_bytes[content_start] != b'\n' {
                data.extend_from_slice(&header_bytes[content_start..]);
                tracing::trace!(target: LOG_TARGET,
                    "Object {} has content after 'obj' on header line ({} bytes)",
                    obj_ref.id,
                    header_bytes.len() - content_start
                );
            }
        }

        // Body, read from the same cursor the header left positioned.
        // Use byte limit instead of line count — large uncompressed streams can have
        // hundreds of thousands of short lines (e.g., vector path drawing commands). ~keep
        const MAX_BYTES: usize = 100 * 1024 * 1024; // 100 MB safety limit ~keep

        {
            let reader = &mut cursor;
            loop {
                let mut chunk = Vec::new();
                let bytes_read = reader.read_until(b'\n', &mut chunk)?;

                if data.len() > MAX_BYTES {
                    tracing::warn!(target: LOG_TARGET,
                        "Object {} exceeded maximum byte limit ({} bytes), truncating",
                        obj_ref.id,
                        MAX_BYTES
                    );
                    break;
                }

                if bytes_read == 0 {
                    let msg = format!(
                        "Unexpected EOF while reading object {} (no endobj found after {} bytes)",
                        obj_ref.id,
                        data.len()
                    );
                    tracing::warn!(target: LOG_TARGET, "{}", msg);
                    // structured-warnings sink. ~keep
                    self.push_structured_warning(crate::extractors::warnings::Warning {
                        category: crate::extractors::warnings::WarningCategory::EofPremature,
                        page: None,
                        message: msg,
                        spec_section: Some("7.5"),
                    });
                    // Don't fail - try to parse what we have ~keep
                    break;
                }

                if chunk.contains(&b'e')
                    && let Some(endobj_pos) = find_substring(&chunk, b"endobj")
                {
                    data.extend_from_slice(&chunk[..endobj_pos]);
                    break;
                }

                data.extend_from_slice(&chunk);
            }
        }

        tracing::trace!(target: LOG_TARGET,
            "About to parse object {} gen {} ({} bytes)",
            obj_ref.id,
            obj_ref.generation,
            data.len()
        );

        let mut obj = match parse_object(&data) {
            Ok((_, parsed_obj)) => parsed_obj,
            Err(e) => {
                let error_kind = match &e {
                    nom::Err::Incomplete(_) => "Incomplete data",
                    nom::Err::Error(err) | nom::Err::Failure(err) => match err.code {
                        nom::error::ErrorKind::Eof => "Unexpected EOF",
                        nom::error::ErrorKind::Tag => "Expected tag not found",
                        nom::error::ErrorKind::Fail => "Parse failed",
                        _ => "Parse error",
                    },
                };
                tracing::warn!(target: LOG_TARGET,
                    "Object {} at offset {} is corrupted ({}), using Null placeholder. \
                     This may result in missing content from the PDF.",
                    obj_ref.id,
                    offset,
                    error_kind
                );
                Object::Null
            }
        };

        let is_encrypt_dict = *self.encrypt_dict_ref.lock_or_recover() == Some(obj_ref);
        if !is_encrypt_dict {
            let handler_guard = self.encryption_handler.lock_or_recover();
            if let Some(handler) = handler_guard.as_ref()
                && handler.is_authenticated()
            {
                Self::decrypt_strings_in_object(handler, &mut obj, obj_ref.id, obj_ref.generation as u32);
            }
        }

        self.object_cache.lock_or_recover().insert(obj_ref, obj.clone());

        Ok(obj)
    }

    /// Load a compressed object from an object stream (Type 2 xref entry).
    ///
    /// # Arguments
    ///
    /// * `obj_ref` - The object reference being loaded
    /// * `stream_obj_num` - The object number of the object stream
    /// * `index_in_stream` - The index within the stream (unused but provided for completeness)
    pub(super) fn load_compressed_object(
        &self,
        obj_ref: ObjectRef,
        stream_obj_num: u32,
        _index_in_stream: u16,
    ) -> Result<Object> {
        tracing::trace!(target: LOG_TARGET,
            object_id = obj_ref.id,
            stream_object_id = stream_obj_num,
            "loading compressed object from object stream"
        );

        if let Err(error) = self.ensure_encryption_initialized() {
            trace_recoverable_pdf_error("initialize_object_stream_encryption", &error);
        }

        let Some((objects_map, newly_parsed)) = self.load_object_stream_objects(stream_obj_num)? else {
            self.object_cache.lock_or_recover().insert(obj_ref, Object::Null);
            return Ok(Object::Null);
        };

        let obj = match objects_map.get(&obj_ref.id) {
            Some(o) => o.clone(),
            None => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "load_compressed_object",
                    error_code = "missing_stream_object",
                    object_id = obj_ref.id,
                    stream_object_id = stream_obj_num,
                    "treating missing compressed object as null"
                );
                Object::Null
            }
        };

        if newly_parsed {
            self.cache_object_stream_entries(stream_obj_num, &objects_map);
        }
        Ok(obj)
    }

    fn load_object_stream_objects(&self, stream_obj_num: u32) -> Result<Option<(Arc<HashMap<u32, Object>>, bool)>> {
        use crate::objstm::parse_object_stream_with_decryption_outcome;

        let stream_ref = ObjectRef::new(stream_obj_num, 0);
        if let Some(cached) = self.object_stream_cache.lock_or_recover().get(&stream_ref).cloned() {
            return Ok(Some((cached, false)));
        }
        let Some(stream_entry) = self.xref.get(stream_obj_num) else {
            tracing::warn!(
                target: crate::LOG_TARGET_ROOT,
                operation = "load_object_stream",
                error_code = "missing_xref_entry",
                stream_object_id = stream_obj_num,
                "treating compressed object as null"
            );
            return Ok(None);
        };
        if stream_entry.entry_type != crate::xref::XRefEntryType::Uncompressed {
            return Err(Error::InvalidPdf(format!(
                "object stream {} is not an uncompressed object",
                stream_obj_num
            )));
        }
        let stream_obj = self.load_uncompressed_object(stream_ref, stream_entry.offset)?;
        let handler_ref = self.encryption_handler.lock_or_recover();
        let outcome = if let Some(handler) = handler_ref.as_ref() {
            match parse_object_stream_with_decryption_outcome(&stream_obj, None, 0, 0) {
                Ok(outcome) => outcome,
                Err(_) => {
                    let decrypt_fn =
                        |data: &[u8]| -> Result<Vec<u8>> { handler.decrypt_stream(data, stream_obj_num, 0) };
                    parse_object_stream_with_decryption_outcome(&stream_obj, Some(&decrypt_fn), stream_obj_num, 0)?
                }
            }
        } else {
            parse_object_stream_with_decryption_outcome(&stream_obj, None, 0, 0)?
        };
        drop(handler_ref);
        self.trace_object_stream_recovery_once(stream_obj_num, &outcome);
        let parsed = Arc::new(outcome.objects);
        self.object_stream_cache
            .lock_or_recover()
            .insert(stream_ref, Arc::clone(&parsed));
        Ok(Some((parsed, true)))
    }

    pub(super) fn trace_object_stream_recovery_once(
        &self,
        stream_obj_num: u32,
        outcome: &crate::objstm::ObjectStreamParseOutcome,
    ) {
        if !outcome.has_recovery() {
            return;
        }
        if self
            .object_stream_telemetry_seen
            .lock_or_recover()
            .should_emit(stream_obj_num)
        {
            outcome.trace_recovery();
        }
    }

    fn cache_object_stream_entries(&self, stream_obj_num: u32, objects_map: &HashMap<u32, Object>) {
        for (obj_num, object) in objects_map {
            let cache_ref = ObjectRef::new(*obj_num, 0);
            let should_cache = if let Some(entry) = self.xref.get(*obj_num) {
                entry.entry_type == crate::xref::XRefEntryType::Compressed && entry.offset == stream_obj_num as u64
            } else {
                true
            };
            if should_cache {
                self.object_cache.lock_or_recover().insert(cache_ref, object.clone());
            } else {
                tracing::trace!(target: LOG_TARGET,
                    object_id = obj_num,
                    stream_object_id = stream_obj_num,
                    "not caching object from stream — xref points elsewhere"
                );
            }
        }
    }

    /// Find object header by searching backwards from a given offset.
    ///
    /// Some PDF generators create xref tables with incorrect offsets that point
    /// to the object body instead of the header. This function searches backwards
    /// from the xref offset to find the actual "N G obj" header.
    ///
    /// We search up to 100 bytes backwards, looking for a line that matches
    /// the expected object header format.
    fn find_object_header_backwards(&self, obj_ref: ObjectRef, wrong_offset: u64) -> Result<u64> {
        if wrong_offset == 0 {
            return Err(Error::ParseError {
                offset: wrong_offset as usize,
                reason: "Cannot search backwards from offset 0".to_string(),
            });
        }

        let search_distance = std::cmp::min(100, wrong_offset);
        let search_start = wrong_offset - search_distance;

        let buffer = self
            .bytes_at(search_start, search_distance as usize + 100)
            .unwrap_or_default();
        let bytes_read = buffer.len();

        if bytes_read == 0 {
            return Err(Error::ParseError {
                offset: wrong_offset as usize,
                reason: "Could not read backwards search region".to_string(),
            });
        }

        // Build the expected header pattern as bytes (NOT string to avoid UTF-8 corruption) ~keep
        let expected_header = format!("{} {} obj", obj_ref.id, obj_ref.generation);
        let pattern_bytes = expected_header.as_bytes();

        // Search for the byte pattern directly (avoids UTF-8 conversion issues with binary data)
        // Find the match closest to wrong_offset (prefer before, but allow small offsets after)
        // ~keep
        let mut best_match: Option<(usize, i64)> = None;

        for (i, window) in buffer[..bytes_read].windows(pattern_bytes.len()).enumerate() {
            if window == pattern_bytes {
                let candidate_offset = search_start + i as u64;
                let distance = (candidate_offset as i64) - (wrong_offset as i64);

                // Accept matches within -100 to +10 bytes of wrong_offset
                // (xref might be slightly off by a few bytes) ~keep
                if (-100..=10).contains(&distance) {
                    let is_better = best_match
                        .as_ref()
                        .is_none_or(|(_, best_dist)| distance.abs() < best_dist.abs());

                    if is_better {
                        best_match = Some((i, distance));
                    }
                }
            }
        }

        if let Some((pos, distance)) = best_match {
            let absolute_offset = search_start + pos as u64;
            tracing::warn!(target: LOG_TARGET,
                "Found object header '{}' at offset {} ({:+} bytes from xref at {})",
                expected_header,
                absolute_offset,
                distance,
                wrong_offset
            );
            return Ok(absolute_offset);
        }

        // Try with whitespace variations (space, double-space, tab between obj_id and gen) ~keep
        let patterns = [
            format!("{} {} obj", obj_ref.id, obj_ref.generation).into_bytes(),
            format!("{}  {} obj", obj_ref.id, obj_ref.generation).into_bytes(),
            format!("{}\t{} obj", obj_ref.id, obj_ref.generation).into_bytes(),
            format!("{} {}\tobj", obj_ref.id, obj_ref.generation).into_bytes(),
        ];

        for pattern in &patterns {
            let mut best_match: Option<(usize, i64)> = None;

            for (i, window) in buffer[..bytes_read].windows(pattern.len()).enumerate() {
                if window == pattern.as_slice() {
                    let candidate_offset = search_start + i as u64;
                    let distance = (candidate_offset as i64) - (wrong_offset as i64);

                    if (-100..=10).contains(&distance) {
                        let is_better = best_match
                            .as_ref()
                            .is_none_or(|(_, best_dist)| distance.abs() < best_dist.abs());

                        if is_better {
                            best_match = Some((i, distance));
                        }
                    }
                }
            }

            if let Some((pos, distance)) = best_match {
                let absolute_offset = search_start + pos as u64;
                tracing::warn!(target: LOG_TARGET,
                    "Found object header '{}' at offset {} ({:+} bytes, pattern match)",
                    expected_header,
                    absolute_offset,
                    distance
                );
                return Ok(absolute_offset);
            }
        }

        Err(Error::ParseError {
            offset: wrong_offset as usize,
            reason: format!(
                "Could not find object header '{}' within {} bytes before offset",
                expected_header, search_distance
            ),
        })
    }
}
