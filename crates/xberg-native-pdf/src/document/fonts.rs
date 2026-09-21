//! Font loading, identity hashing, and embedded-font extraction.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Resolve an object reference.
    ///
    /// This is useful when working with indirect object references
    /// in content streams or resource dictionaries.
    pub fn resolve_object(&self, obj: &Object) -> Result<Object> {
        if let Some(ref_val) = obj.as_reference() {
            self.load_object(ref_val)
        } else {
            Ok(obj.clone())
        }
    }

    /// Look up a font from the per-document `font_cache`, parsing and inserting
    /// on a cache miss. Used by the page renderer so that `FontInfo::from_dict`
    /// (which decodes widths, CID maps, ToUnicode CMaps, and extracts embedded
    /// font bytes) is called at most once per PDF object reference, even when
    /// multiple pages share the same font resources.
    pub fn get_or_load_font_for_rendering(&self, font_obj: &Object) -> Result<Arc<crate::fonts::FontInfo>> {
        if let Some(font_ref) = font_obj.as_reference() {
            let cached = self.font_cache.lock_or_recover().get(&font_ref).cloned();
            if let Some(arc) = cached {
                return Ok(arc);
            }
        }
        let resolved = self.deref_object_for_inks(font_obj)?;
        let info = crate::fonts::FontInfo::from_dict(&resolved, self)?;
        let arc = Arc::new(info);
        if let Some(font_ref) = font_obj.as_reference() {
            self.font_cache.lock_or_recover().insert(font_ref, Arc::clone(&arc));
        }
        Ok(arc)
    }

    /// Compute a cheap content-based font identity hash from a loaded font object.
    /// Uses only inline fields (no reference resolution / load_object calls) to keep
    /// the cost at ~200ns. Relies on BaseFont + Subtype + Encoding (when inline) to
    /// uniquely identify fonts within a document. For reference-only fields (ToUnicode,
    /// FontDescriptor, DescendantFonts), hashes their presence to avoid false positives
    /// between fonts with vs without these features.
    /// `font_identity_hash_cheap` of `font_ref`'s object, memoized (an object's
    /// content is fixed within a document).
    fn cached_font_identity_hash(&self, font_ref: ObjectRef) -> Option<u64> {
        if !self.font_identity_shared_cache_enabled.load(Ordering::Acquire) {
            return None;
        }
        if let Some(cached) = self.font_id_hash_cache.lock_or_recover().get(&font_ref) {
            return *cached;
        }
        let font = self.load_object(font_ref).ok()?;
        let identity = self.font_identity_hash_details(&font);
        let cached = identity.cacheable.then_some(identity.value);
        self.font_id_hash_cache.lock_or_recover().insert(font_ref, cached);
        cached
    }

    fn font_set_identity_hash(&self, entries: &[(&String, &Object)]) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (name, font_obj) in entries {
            let font_ref = font_obj.as_reference()?;
            let font_hash = self.cached_font_identity_hash(font_ref)?;
            name.as_str().hash(&mut hasher);
            font_hash.hash(&mut hasher);
        }
        Some(hasher.finish())
    }

    /// Document-aware font identity that follows every indirect object consumed
    /// by `FontInfo::from_dict`. PDF object numbers are local to one document;
    /// only resolved content can safely key the process-wide cache. ~keep
    #[cfg(test)]
    pub(super) fn font_identity_hash_with_descendants(&self, font_obj: &Object) -> u64 {
        self.font_identity_hash_details(font_obj).value
    }

    pub(super) fn font_identity_hash_details(&self, font_obj: &Object) -> FontIdentityHash {
        use std::hash::{Hash, Hasher};
        let base = Self::font_identity_hash_cheap(font_obj);
        // Encrypted stream bytes are document-key-dependent ciphertext, so
        // neither hashing nor cross-document identity reuse is safe. ~keep
        if self.is_encrypted() || !self.font_identity_shared_cache_enabled.load(Ordering::Acquire) {
            return FontIdentityHash {
                value: base,
                cacheable: false,
                subtree_depth: 0,
            };
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        base.hash(&mut hasher);
        let mut cacheable = true;

        if let Some(font_dict) = font_obj.as_dict() {
            let mut traversal = FontHashTraversal::default();
            for field in FONT_IDENTITY_SEMANTIC_FIELDS {
                if let Some(value) = font_dict.get(*field) {
                    field.hash(&mut hasher);
                    cacheable &= self.hash_pdf_object_resolved(value, &mut hasher, &mut traversal, 0);
                }
            }
        }

        FontIdentityHash {
            value: hasher.finish(),
            cacheable: cacheable && self.font_identity_shared_cache_enabled.load(Ordering::Acquire),
            subtree_depth: 0,
        }
    }

    fn reserve_font_identity_hash_bytes(&self, bytes: usize) -> bool {
        if !self.font_identity_shared_cache_enabled.load(Ordering::Acquire) {
            return false;
        }
        let reserved = self
            .font_identity_hashed_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(bytes)
                    .filter(|next| *next <= FONT_IDENTITY_MAX_HASHED_BYTES)
            });
        if reserved.is_err() {
            self.font_identity_shared_cache_enabled.store(false, Ordering::Release);
            return false;
        }
        self.font_identity_shared_cache_enabled.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(super) fn font_identity_hashed_bytes(&self) -> usize {
        self.font_identity_hashed_bytes.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(super) fn font_identity_shared_cache_enabled(&self) -> bool {
        self.font_identity_shared_cache_enabled.load(Ordering::Acquire)
    }

    /// Hash an object by resolved content, not by its document-local object id.
    /// Any missing reference, cycle, or depth overflow makes the identity
    /// ineligible for the shared caches while remaining deterministic. ~keep
    fn hash_pdf_object_resolved<H: std::hash::Hasher>(
        &self,
        obj: &Object,
        hasher: &mut H,
        traversal: &mut FontHashTraversal,
        depth: usize,
    ) -> bool {
        use std::hash::Hash;
        traversal.observe_depth(depth);
        if depth >= FONT_IDENTITY_MAX_REFERENCE_DEPTH {
            FONT_HASH_DEPTH_LIMIT_MARKER.hash(hasher);
            obj.type_name().hash(hasher);
            return false;
        }

        match obj {
            Object::Reference(reference) => self.hash_pdf_reference_resolved(*reference, hasher, traversal, depth),
            Object::Null
            | Object::Boolean(_)
            | Object::Integer(_)
            | Object::Real(_)
            | Object::String(_)
            | Object::Name(_) => Self::hash_pdf_scalar(obj, hasher),
            Object::Array(values) => self.hash_pdf_array_resolved(values, hasher, traversal, depth),
            Object::Dictionary(dict) => {
                7u8.hash(hasher);
                self.hash_pdf_dictionary_resolved(dict, hasher, traversal, depth + 1)
            }
            Object::Stream { dict, data } => {
                8u8.hash(hasher);
                let cacheable = self.hash_pdf_dictionary_resolved(dict, hasher, traversal, depth + 1);
                data.len().hash(hasher);
                if !cacheable || !self.reserve_font_identity_hash_bytes(data.len()) {
                    FONT_HASH_BYTE_LIMIT_MARKER.hash(hasher);
                    return false;
                }
                data.as_ref().hash(hasher);
                self.font_identity_shared_cache_enabled.load(Ordering::Acquire)
            }
        }
    }

    fn hash_pdf_reference_resolved<H: std::hash::Hasher>(
        &self,
        reference: ObjectRef,
        hasher: &mut H,
        traversal: &mut FontHashTraversal,
        depth: usize,
    ) -> bool {
        use std::hash::Hash;
        FONT_HASH_RESOLVED_REFERENCE_MARKER.hash(hasher);
        if let Some(identity) = traversal.resolved_hashes.get(&reference) {
            return Self::hash_memoized_font_identity(*identity, hasher, traversal, depth);
        }
        if traversal.resolving.contains(&reference) {
            FONT_HASH_REFERENCE_CYCLE_MARKER.hash(hasher);
            return false;
        }
        if traversal.resolved_references >= FONT_IDENTITY_MAX_RESOLVED_REFERENCES {
            FONT_HASH_REFERENCE_LIMIT_MARKER.hash(hasher);
            return false;
        }
        traversal.resolved_references += 1;
        if self.font_identity_shared_cache_enabled.load(Ordering::Acquire)
            && let Some(identity) = self
                .font_reference_hash_cache
                .lock_or_recover()
                .get(&reference)
                .copied()
        {
            if !Self::hash_memoized_font_identity(identity, hasher, traversal, depth) {
                return false;
            }
            traversal.resolved_hashes.insert(reference, identity);
            return true;
        }
        traversal.resolving.insert(reference);

        let identity = self.resolve_font_reference_identity(reference, traversal, depth + 1);
        traversal.resolving.remove(&reference);
        identity.value.hash(hasher);
        traversal.resolved_hashes.insert(reference, identity);
        if identity.cacheable && self.font_identity_shared_cache_enabled.load(Ordering::Acquire) {
            self.font_reference_hash_cache
                .lock_or_recover()
                .insert(reference, identity);
        }
        identity.cacheable
    }

    fn hash_memoized_font_identity<H: std::hash::Hasher>(
        identity: FontIdentityHash,
        hasher: &mut H,
        traversal: &mut FontHashTraversal,
        depth: usize,
    ) -> bool {
        use std::hash::Hash;
        let Some(max_depth) = depth
            .checked_add(1)
            .and_then(|entry_depth| entry_depth.checked_add(identity.subtree_depth))
        else {
            FONT_HASH_DEPTH_LIMIT_MARKER.hash(hasher);
            return false;
        };
        if max_depth >= FONT_IDENTITY_MAX_REFERENCE_DEPTH {
            FONT_HASH_DEPTH_LIMIT_MARKER.hash(hasher);
            return false;
        }
        traversal.observe_depth(max_depth);
        identity.value.hash(hasher);
        identity.cacheable
    }

    fn resolve_font_reference_identity(
        &self,
        reference: ObjectRef,
        traversal: &mut FontHashTraversal,
        depth: usize,
    ) -> FontIdentityHash {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        traversal.begin_reference(depth);
        let cacheable = match self.load_object(reference) {
            Ok(resolved) => self.hash_pdf_object_resolved(&resolved, &mut hasher, traversal, depth),
            Err(_) => {
                FONT_HASH_UNRESOLVED_REFERENCE_MARKER.hash(&mut hasher);
                false
            }
        };
        let subtree_depth = traversal.end_reference();
        FontIdentityHash {
            value: hasher.finish(),
            cacheable,
            subtree_depth,
        }
    }

    fn hash_pdf_array_resolved<H: std::hash::Hasher>(
        &self,
        values: &[Object],
        hasher: &mut H,
        traversal: &mut FontHashTraversal,
        depth: usize,
    ) -> bool {
        use std::hash::Hash;
        6u8.hash(hasher);
        values.len().hash(hasher);
        let mut cacheable = true;
        for value in values {
            cacheable &= self.hash_pdf_object_resolved(value, hasher, traversal, depth + 1);
        }
        cacheable
    }

    fn hash_pdf_scalar<H: std::hash::Hasher>(obj: &Object, hasher: &mut H) -> bool {
        use std::hash::Hash;
        match obj {
            Object::Null => 0u8.hash(hasher),
            Object::Boolean(value) => {
                1u8.hash(hasher);
                value.hash(hasher);
            }
            Object::Integer(value) => {
                2u8.hash(hasher);
                value.hash(hasher);
            }
            Object::Real(value) => {
                3u8.hash(hasher);
                value.to_bits().hash(hasher);
            }
            Object::String(value) => {
                4u8.hash(hasher);
                value.hash(hasher);
            }
            Object::Name(value) => {
                5u8.hash(hasher);
                value.hash(hasher);
            }
            _ => return false,
        }
        true
    }

    fn hash_pdf_dictionary_resolved<H: std::hash::Hasher>(
        &self,
        dict: &HashMap<String, Object>,
        hasher: &mut H,
        traversal: &mut FontHashTraversal,
        depth: usize,
    ) -> bool {
        use std::hash::Hash;
        let mut keys: Vec<&str> = dict
            .keys()
            .map(String::as_str)
            .filter(|key| FONT_IDENTITY_CONSUMED_DICTIONARY_FIELDS.contains(key))
            .collect();
        keys.sort_unstable();
        keys.len().hash(hasher);
        let mut cacheable = true;
        for key in keys {
            key.hash(hasher);
            let value_cacheable = dict
                .get(key)
                .is_some_and(|value| self.hash_pdf_object_resolved(value, hasher, traversal, depth));
            cacheable &= value_cacheable;
        }
        cacheable
    }

    pub(super) fn font_identity_hash_cheap(font_obj: &Object) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        if let Some(d) = font_obj.as_dict() {
            if let Some(Object::Name(n)) = d.get("BaseFont") {
                1u8.hash(&mut hasher);
                n.hash(&mut hasher);
            }
            if let Some(Object::Name(n)) = d.get("Subtype") {
                2u8.hash(&mut hasher);
                n.hash(&mut hasher);
            }
            if let Some(enc) = d.get("Encoding") {
                3u8.hash(&mut hasher);
                match enc {
                    Object::Name(n) => n.hash(&mut hasher),
                    Object::Reference(_) => b"enc_ref".hash(&mut hasher),
                    Object::Dictionary(_) => b"enc_dict".hash(&mut hasher),
                    _ => {}
                }
            }
            if let Some(to_unicode) = d.get("ToUnicode") {
                4u8.hash(&mut hasher);
                to_unicode.type_name().hash(&mut hasher);
            }
            if d.get("FontDescriptor").is_some() {
                5u8.hash(&mut hasher);
            }
            if let Some(Object::Array(arr)) = d.get("DescendantFonts") {
                6u8.hash(&mut hasher);
                arr.len().hash(&mut hasher);
            }
            // Width metrics: two non-subset fonts can share
            // BaseFont + Subtype + Encoding yet ship different glyph widths —
            // Standard-14 fonts may carry producer-specific /Widths overrides
            // (§9.6.2.2), and differently-optimized embeds of the same named
            // font diverge similarly. Without folding widths into the key,
            // such fonts collide on the cross-document cache and the second
            // document gets the first's advances. The cheap seed covers direct
            // simple-font widths; the document-aware identity resolves indirect
            // widths and every descendant CIDFont metric. ~keep
            if let Some(Object::Integer(first_char)) = d.get("FirstChar") {
                7u8.hash(&mut hasher);
                first_char.hash(&mut hasher);
            }
            if let Some(Object::Integer(last_char)) = d.get("LastChar") {
                8u8.hash(&mut hasher);
                last_char.hash(&mut hasher);
            }
            if let Some(Object::Array(widths)) = d.get("Widths") {
                9u8.hash(&mut hasher);
                (widths.len() as u64).hash(&mut hasher);
                for w in widths {
                    match w {
                        Object::Integer(i) => i.hash(&mut hasher),
                        // Bit-pattern hash so equal widths hash equally
                        // (these are glyph advances, never NaN in practice). ~keep
                        Object::Real(r) => r.to_bits().hash(&mut hasher),
                        _ => 0u8.hash(&mut hasher),
                    }
                }
            }
        }
        hasher.finish()
    }

    /// Whether a font dictionary describes a font that is *document-local* and
    /// therefore must never be served from / inserted into the cross-document
    /// global font cache (Layer 6), even if its cheap identity hash collides
    /// with a font in another document.
    ///
    /// Type 3 fonts (PDF 32000-1 §9.6.5) define their glyphs as streams of PDF
    /// graphics operators in a `/CharProcs` dictionary whose procedures
    /// reference the *owning document's* resources (XObjects, ColorSpaces,
    /// ExtGState, …). Two Type 3 fonts from different documents that happen to
    /// share `/Name` + `/Encoding` shape are NOT interchangeable: serving one
    /// document's parsed `FontInfo` for the other yields wrong glyphs. Such
    /// fonts carry no subset prefix, so the cheap hash cannot distinguish them
    /// — this predicate gates them out of the global cache instead.
    pub(super) fn font_is_document_local(font_obj: &Object) -> bool {
        let dict = match font_obj.as_dict() {
            Some(d) => d,
            None => return false,
        };

        if dict.get("Subtype").and_then(|s| s.as_name()) == Some("Type3") {
            return true;
        }

        // Subset fonts carry a document-specific glyph subset and ToUnicode
        // CMap, so they are unsafe to share across documents even when the
        // BaseFont name collides. A subset BaseFont is tagged with exactly six
        // uppercase letters and a '+' per ISO 32000-1:2008 §9.6.4
        // (e.g. `AAAAAA+ArialUnicodeMS`). ~keep
        match dict.get("BaseFont").and_then(|b| b.as_name()) {
            Some(base_font) => Self::is_subset_basefont(base_font),
            // A non-Type3 font is required by the spec to carry /BaseFont; if it
            // is absent we cannot prove the font is shareable, so fail safe and
            // treat it as document-local rather than risk poisoning the cache. ~keep
            None => true,
        }
    }

    /// Detect a PDF subset-font tag on a `/BaseFont` name: exactly six uppercase
    /// ASCII letters followed by `+`, per ISO 32000-1:2008 §9.6.4 (e.g.
    /// `AAAAAA+ArialUnicodeMS`). `is_ascii_uppercase` is precisely A–Z, so
    /// multibyte (CJK) names never satisfy the test and are treated as full
    /// fonts — correct, since subset tags are by definition ASCII A–Z.
    fn is_subset_basefont(base_font: &str) -> bool {
        let bytes = base_font.as_bytes();
        bytes.len() > 7 && bytes[6] == b'+' && bytes[..6].iter().all(|b| b.is_ascii_uppercase())
    }

    /// Load fonts from a Resources dictionary into the extractor.
    pub(crate) fn load_fonts(
        &self,
        resources: &Object,
        extractor: &mut crate::extractors::TextExtractor<'_>,
    ) -> Result<()> {
        use crate::fonts::FontInfo;

        let resources_obj = if let Some(res_ref) = resources.as_reference() {
            self.load_object(res_ref)?
        } else {
            resources.clone()
        };

        let resources_dict = match resources_obj.as_dict() {
            Some(d) => d,
            None => {
                tracing::warn!(target: LOG_TARGET,
                    "Resources is not a dictionary (type: {}), treating as empty",
                    resources_obj.type_name()
                );
                return Ok(());
            }
        };

        if let Some(font_obj) = resources_dict.get("Font") {
            let font_dict_ref = font_obj.as_reference();
            let font_dict_obj = if let Some(font_ref) = font_dict_ref {
                self.load_object(font_ref)?
            } else {
                font_obj.clone()
            };

            // Layer 2: Check font set cache for the /Font dictionary.
            // Pages sharing the same /Font dict skip the entire per-font loop. ~keep
            if let Some(font_dict_ref) = font_dict_ref {
                let cached_set_opt = self.font_set_cache.lock_or_recover().get(&font_dict_ref).cloned();
                if let Some(cached_set) = cached_set_opt {
                    for (name, font_arc) in &cached_set {
                        extractor.add_font_shared(name.clone(), Arc::clone(font_arc));
                    }
                    extractor.share_truetype_cmaps();
                    return Ok(());
                }
            }

            if let Some(font_dict) = font_dict_obj.as_dict() {
                // Compute font fingerprint from (name → ObjectRef) pairs.
                // Hash the MAPPING between font names and their object refs,
                // not just the sets separately. This prevents false cache hits
                // when two font dicts have the same set of refs and names but
                // different name-to-ref assignments. ~keep
                //
                // A font written inline instead of as a reference has no object id
                // to hash, so a dictionary holding one leaves nothing in the key but
                // the resource names — and `/F1 /F2` names every page and every Form
                // XObject of a Word-produced PDF alike. The first dictionary to
                // arrive would then answer for all of them, which makes the text a
                // property of the visit order. There is no stable key here, so there
                // is no entry: the per-font loop below already declines to cache a
                // direct font object for the same reason. ~keep
                let fingerprint = {
                    use std::hash::{Hash, Hasher};
                    let mut name_ref_pairs: Vec<(&str, Option<ObjectRef>)> = font_dict
                        .iter()
                        .map(|(name, fo)| (name.as_str(), fo.as_reference()))
                        .collect();
                    name_ref_pairs.sort_by(|a, b| a.0.cmp(b.0));
                    name_ref_pairs.iter().all(|(_, obj_ref)| obj_ref.is_some()).then(|| {
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        for (name, obj_ref) in &name_ref_pairs {
                            name.hash(&mut hasher);
                            if let Some(r) = obj_ref {
                                r.id.hash(&mut hasher);
                                r.generation.hash(&mut hasher);
                            }
                        }
                        hasher.finish()
                    })
                };

                let cached_fingerprint_opt =
                    fingerprint.and_then(|key| self.font_fingerprint_cache.lock_or_recover().get(&key).cloned());
                if let Some(cached_set) = cached_fingerprint_opt {
                    for (name, font_arc) in &cached_set {
                        extractor.add_font_shared(name.clone(), Arc::clone(font_arc));
                    }
                    extractor.share_truetype_cmaps();
                    return Ok(());
                }

                // Layer 4: Name-based font set cache with spot-check verification.
                // Pages in the same document often use the same font names mapped to
                // different ObjectRefs but identical base fonts (e.g., 764 pages each
                // creating T1_0→Helvetica, T1_1→Times-Roman with unique object numbers).
                // Cache the resolved font set by sorted font names, then on subsequent
                // pages verify ONE font via load+hash to confirm the mapping is the same. ~keep
                let name_hash = {
                    use std::hash::{Hash, Hasher};
                    let mut font_names: Vec<&str> = font_dict.keys().map(|k| k.as_str()).collect();
                    font_names.sort();
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    font_names.hash(&mut hasher);
                    hasher.finish()
                };

                let cached_name_set = self.font_name_set_cache.lock_or_recover().get(&name_hash).cloned();
                // Sort font entries by name for deterministic processing order.
                // HashMap iteration order is randomized per-process, which causes
                // non-deterministic text extraction when font CMap sharing depends
                // on the order fonts are loaded. ~keep
                let mut sorted_font_entries: Vec<(&String, &Object)> = font_dict.iter().collect();
                sorted_font_entries.sort_by_key(|(name, _)| name.as_str());

                if let Some((cached_set, check_hash)) = cached_name_set {
                    // Verify the cached font set by computing a combined identity hash
                    // over ALL reference fonts in the current Resources dict (sorted by
                    // name). This prevents false cache hits when pages reuse the same
                    // font key names but embed different per-page subsets — a single-font
                    // spot-check is insufficient because it only guards one entry
                    // lets differing sibling fonts (F2, F3 …) slip through unchecked.
                    // Fixes the regression described above. ~keep
                    if self.font_set_identity_hash(&sorted_font_entries) == Some(check_hash) {
                        for (name, font_arc) in cached_set.iter() {
                            extractor.add_font_shared(name.clone(), Arc::clone(font_arc));
                        }
                        extractor.share_truetype_cmaps();
                        return Ok(());
                    }
                }

                // What every cache below stores is what THIS /Font dictionary
                // resolves to, collected here as the loop resolves it.
                //
                // Reading it back off the extractor afterwards is what made a page's
                // text depend on the order the pages were read. The extractor also
                // holds the fonts of the page that owns it and of any enclosing Form
                // XObject, so its font set is a union belonging to whichever page
                // reached this dictionary first, and a later page that shares the
                // dictionary inherits that page's fonts under its own resource
                // names. Reading it back after cmap donation adds the same problem
                // again, because donation rewrites a font using whatever other fonts
                // stand beside it. Donation still runs, per page, over that page's
                // own set; it just does not reach the cache. ~keep
                let mut resolved: Vec<(String, Arc<FontInfo>)> = Vec::with_capacity(sorted_font_entries.len());

                let mut all_from_cache = true;

                for (name, font_obj) in &sorted_font_entries {
                    if let Some(font_ref) = font_obj.as_reference() {
                        let cached_font_opt = self.font_cache.lock_or_recover().get(&font_ref).cloned();
                        if let Some(cached) = cached_font_opt {
                            resolved.push(((*name).clone(), Arc::clone(&cached)));
                            extractor.add_font_shared((*name).clone(), cached);
                            continue;
                        }
                        all_from_cache = false;
                        let font = self.load_object(font_ref)?;

                        // Resolve and hash every indirect semantic subtree that
                        // `FontInfo::from_dict` consumes. PDF object numbers are
                        // document-local and cannot identify shared cache entries. ~keep
                        let identity = self.font_identity_hash_details(&font);
                        let id_hash = identity.value;

                        // Type 3 fonts and subset fonts must not cross
                        // PdfDocument boundaries via the global cache — their
                        // glyph procs / glyph-subset + ToUnicode mappings are
                        // document-specific. The per-document Layer 4/5 caches
                        // below stay safe to use. ~keep
                        let is_document_local = Self::font_is_document_local(&font);

                        // Layer 5: Per-font identity cache — skip from_dict when a
                        // structurally identical font was already parsed elsewhere. ~keep
                        let cached_identity_opt = identity
                            .cacheable
                            .then(|| self.font_identity_cache.lock_or_recover().get(&id_hash).cloned())
                            .flatten();
                        if let Some(cached) = cached_identity_opt {
                            self.font_cache.lock_or_recover().insert(font_ref, Arc::clone(&cached));
                            resolved.push(((*name).clone(), Arc::clone(&cached)));
                            extractor.add_font_shared((*name).clone(), cached);
                            continue;
                        }

                        // Layer 6: Global cross-document font cache — reuse fonts
                        // parsed by previous PdfDocument instances in this process.
                        // Skipped entirely for document-local fonts. ~keep
                        if identity.cacheable
                            && !is_document_local
                            && let Some(cached) = crate::fonts::global_cache::global_font_cache_get(id_hash)
                        {
                            self.font_identity_cache
                                .lock_or_recover()
                                .insert(id_hash, Arc::clone(&cached));
                            self.font_cache.lock_or_recover().insert(font_ref, Arc::clone(&cached));
                            resolved.push(((*name).clone(), Arc::clone(&cached)));
                            extractor.add_font_shared((*name).clone(), cached);
                            continue;
                        }

                        match FontInfo::from_dict(&font, self) {
                            Ok(font_info) => {
                                let arc = Arc::new(font_info);
                                // Populate the document-level caches always; the
                                // global cross-document cache only for fonts that
                                // are safe to share across documents. ~keep
                                if identity.cacheable && !is_document_local {
                                    crate::fonts::global_cache::global_font_cache_insert(id_hash, Arc::clone(&arc));
                                }
                                if identity.cacheable {
                                    self.font_identity_cache
                                        .lock_or_recover()
                                        .insert(id_hash, Arc::clone(&arc));
                                }
                                self.font_cache.lock_or_recover().insert(font_ref, Arc::clone(&arc));
                                resolved.push(((*name).clone(), Arc::clone(&arc)));
                                extractor.add_font_shared((*name).clone(), arc);
                            }
                            Err(error) => {
                                tracing::warn!(
                                    target: crate::LOG_TARGET_ROOT,
                                    operation = "load_font_resource",
                                    error_code = error.telemetry_code(),
                                    error_offset = ?error.telemetry_offset(),
                                    "using fallback font encoding"
                                );
                                continue;
                            }
                        }
                    } else {
                        // Direct font object — parse without caching (no stable key) ~keep
                        all_from_cache = false;
                        let font = *font_obj;
                        match FontInfo::from_dict(font, self) {
                            Ok(font_info) => {
                                let arc = Arc::new(font_info);
                                resolved.push(((*name).clone(), Arc::clone(&arc)));
                                extractor.add_font_shared((*name).clone(), arc);
                            }
                            Err(error) => {
                                tracing::warn!(
                                    target: crate::LOG_TARGET_ROOT,
                                    operation = "load_font_resource",
                                    error_code = error.telemetry_code(),
                                    error_offset = ?error.telemetry_offset(),
                                    "using fallback font encoding"
                                );
                                continue;
                            }
                        }
                    }
                }

                // Always re-share TrueType CMaps after loading fonts. Cached fonts
                // may lack donated CMaps because Arc::make_mut creates a per-extractor
                // clone that is not written back to per-font cache. A donor font added
                // in a later load_fonts call (e.g. an XObject font donating to a
                // page-level font already in the extractor) requires sharing to run
                // again even when all fonts came from cache. ~keep
                extractor.share_truetype_cmaps();

                if let Some(fdr) = font_dict_ref {
                    self.font_set_cache.lock_or_recover().insert(fdr, resolved.clone());
                }
                if let Some(key) = fingerprint {
                    self.font_fingerprint_cache
                        .lock_or_recover()
                        .insert(key, resolved.clone());
                }

                // Layer 4 is keyed by font NAMES alone, so it carries a combined
                // identity hash over every reference font in the Resources dict
                // (sorted by name): a hit requires all of them to match, not one.
                // This prevents false positives when pages reuse the same font key
                // names with different per-page subsets. ~keep
                if !all_from_cache && let Some(combined_check_hash) = self.font_set_identity_hash(&sorted_font_entries)
                {
                    self.font_name_set_cache
                        .lock_or_recover()
                        .insert(name_hash, (Arc::new(resolved), combined_check_hash));
                }

                return Ok(());
            }
        }

        Ok(())
    }

    /// Public wrapper for `load_fonts` (normally pub(crate)).
    /// Loads font dictionaries from a resources object into a TextExtractor.
    pub fn load_fonts_public(
        &self,
        resources: &Object,
        extractor: &mut crate::extractors::TextExtractor<'_>,
    ) -> Result<()> {
        self.load_fonts(resources, extractor)
    }

    /// Per-page mapping of PDF font-resource names (e.g. `"F75"`) to their
    /// canonical face name (e.g. `"TeXGyreTermesX-Regular"`, with any
    /// subset-prefix `ABCDEF+` stripped).
    ///
    /// Used by the layout-preserving DOCX writer so each text span can be
    /// emitted with the actual face name in `<w:rFonts>` instead of a
    /// PDF-internal resource id. The vector is `pages × map`; `map[i]`
    /// covers all fonts referenced by page `i`'s Resources.
    pub fn page_font_face_lookups(&self) -> Result<Vec<std::collections::HashMap<String, String>>> {
        use std::collections::HashMap;
        let n = self.page_count()?;
        let mut out: Vec<HashMap<String, String>> = Vec::with_capacity(n);
        for page_idx in 0..n {
            let mut lookup: HashMap<String, String> = HashMap::new();
            // Inline get_page → Resources so this works without `rendering`. ~keep
            let resources = match self.get_page(page_idx) {
                Ok(page) => match page.as_dict() {
                    Some(d) => {
                        let r = d
                            .get("Resources")
                            .cloned()
                            .unwrap_or(Object::Dictionary(std::collections::HashMap::new()));
                        if let Some(rref) = r.as_reference() {
                            self.load_object(rref)
                                .unwrap_or(Object::Dictionary(std::collections::HashMap::new()))
                        } else {
                            r
                        }
                    }
                    None => {
                        out.push(lookup);
                        continue;
                    }
                },
                Err(_) => {
                    out.push(lookup);
                    continue;
                }
            };
            let mut extractor = crate::extractors::TextExtractor::new();
            if self.load_fonts_public(&resources, &mut extractor).is_ok() {
                for (resource_name, info) in extractor.get_font_set() {
                    let canonical = info
                        .base_font
                        .split_once('+')
                        .map(|(_, rest)| rest)
                        .unwrap_or(info.base_font.as_str())
                        .to_string();
                    lookup.insert(resource_name, canonical);
                }
            }
            out.push(lookup);
        }
        Ok(out)
    }

    /// Extract every embedded font program (TrueType / OpenType bytes) used
    /// anywhere in the document, deduplicated by `BaseFont` name.
    ///
    /// Walks every page's font dictionary, loads each font via the same path
    /// `extract_text` uses, and returns the unique set of fonts that have
    /// embedded `FontFile2`/`FontFile3` streams. The `String` is the base
    /// font name (with any subset prefix like `ABCDEF+` stripped) and the
    /// `Vec<u8>` is the raw font program — directly suitable for re-embedding
    /// into another container (DOCX `word/fonts/`, another PDF, etc.).
    ///
    /// Fonts without embedded data (standard 14, missing FontFile streams)
    /// are skipped — there's nothing to extract.
    pub fn extract_embedded_fonts(&self) -> Result<Vec<(String, Vec<u8>)>> {
        use std::collections::HashMap;
        let mut by_name: HashMap<String, Vec<u8>> = HashMap::new();

        let n = self.page_count()?;
        for page_idx in 0..n {
            // Inline get_page_resources so this works without `rendering`. ~keep
            let resources = match self.get_page(page_idx) {
                Ok(page) => match page.as_dict() {
                    Some(d) => {
                        let r = d
                            .get("Resources")
                            .cloned()
                            .unwrap_or(Object::Dictionary(std::collections::HashMap::new()));
                        if let Some(rref) = r.as_reference() {
                            self.load_object(rref)
                                .unwrap_or_else(|_| Object::Dictionary(std::collections::HashMap::new()))
                        } else {
                            r
                        }
                    }
                    None => continue,
                },
                Err(_) => continue,
            };
            let mut extractor = crate::extractors::TextExtractor::new();
            if self.load_fonts_public(&resources, &mut extractor).is_err() {
                continue;
            }
            for (_resource_name, font_arc) in extractor.get_font_set() {
                let Some(data) = font_arc.embedded_font_data.as_ref() else {
                    continue;
                };
                if data.is_empty() {
                    continue;
                }
                // Subset-prefix stripping: PDF font subsets carry a 6-letter
                // prefix followed by `+`, e.g. `ABCDEF+Calibri-Bold`. The
                // prefix is meaningless to consumers — strip it for dedup. ~keep
                let base = font_arc.base_font.as_str();
                let canonical = base.split_once('+').map(|(_, rest)| rest).unwrap_or(base);
                // When several subsets share a base name, `get_font_set()` yields
                // them in HashMap order, so `or_insert` kept a NONDETERMINISTIC
                // one - the returned bytes changed run to run for the same PDF.
                //
                // Choose by a TOTAL ORDER instead: largest program, ties broken
                // bytewise. Size is only a heuristic for "the richer subset" - a
                // program's byte count also grows with hinting and auxiliary
                // tables, so a larger subset is not necessarily a superset of a
                // smaller one. What the total order does guarantee is the property
                // callers actually depend on: the same PDF always yields the same
                // bytes. ~keep
                match by_name.entry(canonical.to_string()) {
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(data.as_ref().clone());
                    }
                    std::collections::hash_map::Entry::Occupied(mut o) => {
                        let cand = data.as_ref();
                        let cur = o.get();
                        if (cand.len(), cand.as_slice()) > (cur.len(), cur.as_slice()) {
                            *o.get_mut() = cand.clone();
                        }
                    }
                }
            }
        }

        let mut out: Vec<(String, Vec<u8>)> = by_name.into_iter().collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Like [`Self::extract_embedded_fonts`] but additionally returns a
    /// per-font Unicode → GID map reconstructed from the source PDF's
    /// `/ToUnicode` CMap and the font's CID/byte→GID table.
    ///
    /// CFF font subsets in PDFs (the typical Word/LibreOffice output)
    /// often ship without a Unicode cmap because CIDs encode the
    /// glyph stream directly. The font program parses fine but
    /// `EmbeddedFont::glyph_lookup` is empty; downstream font
    /// registration treats the font as unusable and falls back to
    /// Helvetica.
    ///
    /// The map returned here lets office_oxide / xberg-native-pdf write
    /// pipelines call [`crate::writer::EmbeddedFont::extend_glyph_lookup`]
    /// to re-populate the missing Unicode→GID entries from the
    /// source-PDF's own `/ToUnicode`. Result: CFF subset fonts
    /// register and render with the source typeface program instead
    /// of base-14 Helvetica.
    pub fn extract_embedded_fonts_with_unicode_maps(
        &self,
    ) -> Result<Vec<(String, Vec<u8>, std::collections::HashMap<u32, u16>)>> {
        let with_widths = self.extract_embedded_fonts_with_unicode_maps_and_widths()?;
        Ok(with_widths
            .into_iter()
            .map(|(name, data, uni, _widths)| (name, data, uni))
            .collect())
    }

    /// Like [`Self::extract_embedded_fonts_with_unicode_maps`] but also
    /// returns the per-glyph widths from the source PDF's `/W` array
    /// (in 1/1000 em units, keyed by GID). Required for re-embedding
    /// CFF font subsets whose synthetic OpenType wrapper carries no
    /// `hmtx` table — without this, ttf-parser returns 0 for every
    /// glyph advance and the round-trip writer emits a `/W` of zeros.
    pub fn extract_embedded_fonts_with_unicode_maps_and_widths(
        &self,
    ) -> Result<
        Vec<(
            String,
            Vec<u8>,
            std::collections::HashMap<u32, u16>,
            std::collections::HashMap<u16, u16>,
        )>,
    > {
        use std::collections::HashMap;
        let mut by_name: HashMap<String, (Vec<u8>, HashMap<u32, u16>, HashMap<u16, u16>)> = HashMap::new();

        let n = self.page_count()?;
        for page_idx in 0..n {
            let resources = match self.get_page(page_idx) {
                Ok(page) => match page.as_dict() {
                    Some(d) => {
                        let r = d
                            .get("Resources")
                            .cloned()
                            .unwrap_or(Object::Dictionary(std::collections::HashMap::new()));
                        if let Some(rref) = r.as_reference() {
                            self.load_object(rref)
                                .unwrap_or_else(|_| Object::Dictionary(std::collections::HashMap::new()))
                        } else {
                            r
                        }
                    }
                    None => continue,
                },
                Err(_) => continue,
            };
            let mut extractor = crate::extractors::TextExtractor::new();
            if self.load_fonts_public(&resources, &mut extractor).is_err() {
                continue;
            }
            for (_resource_name, font_arc) in extractor.get_font_set() {
                let Some(data) = font_arc.embedded_font_data.as_ref() else {
                    continue;
                };
                if data.is_empty() {
                    continue;
                }
                let base = font_arc.base_font.as_str();
                let canonical = base.split_once('+').map(|(_, rest)| rest).unwrap_or(base);

                // Build Unicode → GID via ToUnicode CMap + GID resolver.
                //
                // We must consult the ToUnicode CMap *directly* rather than
                // going through `char_to_unicode`. `char_to_unicode` falls
                // through to a CID-as-Unicode fallback when the ToUnicode
                // CMap has no entry for a given code (Identity-H + Adobe-
                // Identity ordering, source font without a Unicode cmap).
                // That fallback returns spurious mappings like
                // U+0069 'i' → GID 105 (because CID 105 has no real
                // ToUnicode entry; the CID-as-Unicode path yields 'i'
                // for code=105 and the embedded TTF has no cmap to set us
                // straight). The spurious entries overwrite the real ones
                // we collected from CIDs that *do* have ToUnicode
                // entries (e.g. CID 0x4C → 'i', GID 76 for a
                // MicrosoftSansSerif subset) — which then makes the
                // injected cmap point Unicode codepoints at the wrong
                // glyph slots and the DOCX round-trip renders broken
                // lowercase letters. ~keep
                let mut uni_to_gid: HashMap<u32, u16> = HashMap::new();
                let to_unicode_cmap = font_arc.to_unicode.as_ref().and_then(|lazy| lazy.get());
                for code in 0u32..=0xFFFF {
                    // Require an authoritative ToUnicode entry. If the
                    // font has no ToUnicode CMap at all we conservatively
                    // skip injection — the fallback chain would only
                    // produce the misleading identity mapping. ~keep
                    let unicode_str = match to_unicode_cmap.as_ref().and_then(|cmap| cmap.get(&code)) {
                        Some(s) if !s.is_empty() && s.as_ref() != "\u{FFFD}" => s.into_owned(),
                        _ => continue,
                    };
                    let cp = match unicode_str.chars().next() {
                        Some(c) => c as u32,
                        None => continue,
                    };
                    // Bare C0 controls (other than the legitimate
                    // whitespace handled in char_to_unicode) never name
                    // a real glyph — drop them so we don't inject a
                    // cmap entry that points U+0000..U+001F at random
                    // GIDs. ~keep
                    if matches!(cp, 0x00..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F) {
                        continue;
                    }
                    // Only emit a Unicode→GID mapping when we have a
                    // real byte/CID → GID resolver from the source PDF.
                    // Falling back to identity for simple fonts whose
                    // CFF encoding parser couldn't extract a mapping
                    // produces a synthetic cmap that points Unicode at
                    // the wrong CFF charset positions: the round-trip
                    // emits Type0+Identity-H+CIDFontType0 and the
                    // viewer reads `glyph_at_charset[byte_code]`,
                    // which only equals the source glyph when CFF
                    // charset == StandardEncoding byte order — rarely
                    // true for subsetted CFF. Without a real mapping
                    // we leave the font un-patched, and office_oxide
                    // falls back to base-14 Helvetica via
                    // `EmbeddedFont::has_usable_unicode_cmap`. ~keep
                    let gid_opt = if let Some(ref map) = font_arc.cff_gid_map {
                        if code <= 0xFF {
                            map.get(&(code as u8)).copied()
                        } else {
                            None
                        }
                    } else if let Some(ref cid_map) = font_arc.cid_to_gid_map {
                        if code <= 0xFFFF {
                            Some(cid_map.get_gid(code as u16))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(gid) = gid_opt {
                        uni_to_gid.insert(cp, gid);
                    }
                }

                // Build GID → width from the source PDF's /W array.
                // For CIDFontType0+Identity-H: CID == GID directly.
                // For CIDFontType2: CID → GID via CIDToGIDMap.
                // For simple CFF (cff_gid_map): byte-code → GID. ~keep
                let mut gid_to_width: HashMap<u16, u16> = HashMap::new();
                if let Some(ref cid_widths) = font_arc.cid_widths {
                    if font_arc.cid_font_type.as_deref() == Some("CIDFontType0") {
                        for (&cid, &w) in cid_widths {
                            gid_to_width.insert(cid, w.round() as u16);
                        }
                    } else if let Some(ref cid_map) = font_arc.cid_to_gid_map {
                        for (&cid, &w) in cid_widths {
                            let gid = cid_map.get_gid(cid);
                            gid_to_width.insert(gid, w.round() as u16);
                        }
                    } else {
                        for (&cid, &w) in cid_widths {
                            gid_to_width.insert(cid, w.round() as u16);
                        }
                    }
                } else if let Some(ref cff_map) = font_arc.cff_gid_map {
                    // Simple CFF font: width-by-byte-code in font_arc.widths. ~keep
                    if let (Some(widths), Some(first)) = (font_arc.widths.as_ref(), font_arc.first_char) {
                        for (i, w) in widths.iter().enumerate() {
                            let byte = first + i as u32;
                            if byte > 0xFF {
                                break;
                            }
                            if let Some(&gid) = cff_map.get(&(byte as u8)) {
                                gid_to_width.insert(gid, w.round() as u16);
                            }
                        }
                    }
                }

                let entry = by_name
                    .entry(canonical.to_string())
                    .or_insert_with(|| (data.as_ref().clone(), HashMap::new(), HashMap::new()));
                // Same total-order choice as `extract_embedded_fonts`: largest
                // program, ties broken bytewise, rather than whichever HashMap
                // order surfaced first. (Size is a heuristic for the richer
                // subset, not a proof of superset - see the note there.)
                //
                // KNOWN GAP, deliberately left for a follow-up: the maps below
                // still merge across ALL subsets while the emitted program is now
                // a single chosen one, so a GID in the maps need not exist in the
                // program we hand back. Worse, when two subsets disagree about a
                // codepoint's GID - which subsets of one base font routinely do -
                // `or_insert` keeps whichever arrived first, so the maps carry the
                // very HashMap-order nondeterminism this fix removes from the
                // program. Fixing it means binding the maps to the chosen subset
                // instead of merging; that is a behaviour change (coverage may
                // shrink where subsets are disjoint) and belongs in its own PR. ~keep
                let cand = data.as_ref();
                if (cand.len(), cand.as_slice()) > (entry.0.len(), entry.0.as_slice()) {
                    entry.0 = cand.clone();
                }
                for (cp, gid) in uni_to_gid {
                    entry.1.entry(cp).or_insert(gid);
                }
                for (gid, w) in gid_to_width {
                    entry.2.entry(gid).or_insert(w);
                }
            }
        }

        let mut out: Vec<(String, Vec<u8>, HashMap<u32, u16>, HashMap<u16, u16>)> = by_name
            .into_iter()
            .map(|(name, (data, cmap, widths))| (name, data, cmap, widths))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }
}
