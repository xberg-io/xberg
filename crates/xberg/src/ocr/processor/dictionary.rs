//! Word-validity lookup across every language a Tesseract run was configured with.

use std::cell::OnceCell;
use xberg_tesseract::TesseractAPI;

/// Languages beyond the primary one that a lookup may open a dictionary for.
///
/// A configuration with more languages than this (the `all` wildcard resolves to every
/// installed pack) cannot be judged cheaply, so a word the primary dictionary rejects gets no
/// verdict instead of one opened handle per language. ~keep
const MAX_SECONDARY_DICTIONARIES: usize = 4;

#[cfg(test)]
thread_local! {
    static SECONDARY_INITIALIZATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn secondary_initialization_count() -> usize {
    SECONDARY_INITIALIZATIONS.get()
}

fn initialize_secondary(tessdata_path: &str, language: &str, oem: i32) -> Option<TesseractAPI> {
    let api = TesseractAPI::new().ok()?;
    api.init_2(tessdata_path, language, oem).ok()?;
    #[cfg(test)]
    SECONDARY_INITIALIZATIONS.set(SECONDARY_INITIALIZATIONS.get() + 1);
    Some(api)
}

struct SecondaryDictionary<T> {
    language: String,
    api: OnceCell<T>,
}

pub(super) struct SecondaryDictionaries<T = TesseractAPI> {
    tessdata_path: String,
    oem: i32,
    entries: Vec<SecondaryDictionary<T>>,
    too_many_languages: bool,
}

impl<T> SecondaryDictionaries<T> {
    pub(super) fn new(tessdata_path: &str, language: &str, oem: u8) -> Self {
        let mut languages = language.split('+').map(str::trim).filter(|lang| !lang.is_empty());
        let first = languages.next();
        let mut entries: Vec<SecondaryDictionary<T>> = Vec::new();
        for lang in languages {
            if Some(lang) != first && !entries.iter().any(|existing| existing.language == lang) {
                entries.push(SecondaryDictionary {
                    language: lang.to_string(),
                    api: OnceCell::new(),
                });
                if entries.len() > MAX_SECONDARY_DICTIONARIES {
                    break;
                }
            }
        }
        let too_many_languages = entries.len() > MAX_SECONDARY_DICTIONARIES;
        Self {
            tessdata_path: tessdata_path.to_string(),
            oem: i32::from(oem),
            entries,
            too_many_languages,
        }
    }

    pub(super) fn initialize(
        &self,
        index: usize,
        constructor: impl FnOnce(&str, &str, i32) -> Option<T>,
    ) -> Option<&T> {
        if self.too_many_languages {
            return None;
        }
        let entry = self.entries.get(index)?;
        if entry.api.get().is_none() {
            let api = constructor(&self.tessdata_path, &entry.language, self.oem)?;
            let _ = entry.api.set(api);
        }
        entry.api.get()
    }

    /// The secondary handles created so far.
    pub(super) fn initialized(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter_map(|entry| entry.api.get())
    }

    fn for_call(&self) -> DictionaryLookups<'_, T> {
        DictionaryLookups {
            dictionaries: self,
            cached: self.entries.iter().map(|_| OnceCell::new()).collect(),
        }
    }
}

struct DictionaryLookups<'a, T> {
    dictionaries: &'a SecondaryDictionaries<T>,
    cached: Vec<OnceCell<Option<&'a T>>>,
}

impl<'a, T> DictionaryLookups<'a, T> {
    fn initialize(&self, index: usize, constructor: impl FnOnce(&str, &str, i32) -> Option<T>) -> Option<&'a T> {
        let dictionaries: &'a SecondaryDictionaries<T> = self.dictionaries;
        // ~keep A failed native initialization is memoized only for this OCR call, so later calls can retry it.
        *self
            .cached
            .get(index)?
            .get_or_init(|| dictionaries.initialize(index, constructor))
    }
}

/// Dictionary membership for a `lang1+lang2` run.
///
/// `TessBaseAPIIsValidWord` consults only the first language's dictionary. A word is valid when
/// any configured language accepts it. Secondary handles load lazily and remain owned by the
/// keyed primary session, so pool capacity and eviction bound their lifetime and memory. ~keep
pub(super) struct WordDictionary<'a> {
    primary: &'a TesseractAPI,
    secondary: DictionaryLookups<'a, TesseractAPI>,
}

impl<'a> WordDictionary<'a> {
    pub(super) fn new(primary: &'a TesseractAPI, secondary: &'a SecondaryDictionaries) -> Self {
        Self {
            primary,
            secondary: secondary.for_call(),
        }
    }

    /// `Some(true)` valid in some configured language, `Some(false)` invalid in all of them,
    /// `None` no verdict (a lookup failed or a dictionary needed to rule the word out is unavailable).
    pub(super) fn is_valid_word(&self, text: &str) -> Option<bool> {
        let primary = match self.primary.is_valid_word(text) {
            Ok(0) => Some(false),
            Ok(_) => Some(true),
            Err(_) => None,
        };
        let dictionaries = self.secondary.dictionaries;
        if dictionaries.entries.is_empty() {
            return primary;
        }
        if dictionaries.too_many_languages {
            return if primary == Some(true) { primary } else { None };
        }
        combine_verdicts(
            primary,
            (0..dictionaries.entries.len()).map(|index| self.lookup(index, text)),
        )
    }

    fn lookup(&self, index: usize, text: &str) -> Option<bool> {
        let api = self.secondary.initialize(index, initialize_secondary)?;
        match api.is_valid_word(text) {
            Ok(0) => Some(false),
            Ok(_) => Some(true),
            Err(_) => None,
        }
    }
}

/// A word is valid when any language accepts it and invalid only when every language
/// rejects it; an unanswered lookup keeps "invalid" from being proven. The secondary lookups
/// are lazy, so none runs once the primary dictionary has accepted the word.
fn combine_verdicts(primary: Option<bool>, secondary: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    if primary == Some(true) {
        return Some(true);
    }
    let mut unproven = primary.is_none();
    for verdict in secondary {
        match verdict {
            Some(true) => return Some(true),
            Some(false) => {}
            None => unproven = true,
        }
    }
    if unproven { None } else { Some(false) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_word_any_language_accepts_is_valid() {
        assert_eq!(
            combine_verdicts(Some(false), [Some(false), Some(true)].into_iter()),
            Some(true)
        );
        assert_eq!(combine_verdicts(None, [Some(true)].into_iter()), Some(true));
    }

    #[test]
    fn a_word_is_invalid_only_when_every_language_rejects_it() {
        assert_eq!(
            combine_verdicts(Some(false), [Some(false), Some(false)].into_iter()),
            Some(false)
        );
        assert_eq!(combine_verdicts(Some(false), std::iter::empty()), Some(false));
    }

    #[test]
    fn an_unanswered_lookup_cannot_prove_a_word_invalid() {
        assert_eq!(combine_verdicts(Some(false), [None, Some(false)].into_iter()), None);
        assert_eq!(combine_verdicts(None, [Some(false)].into_iter()), None);
    }

    #[test]
    fn secondary_lookups_are_skipped_once_the_primary_accepts() {
        let calls = Cell::new(0);
        let lookups = std::iter::from_fn(|| {
            calls.set(calls.get() + 1);
            Some(Some(false))
        });
        assert_eq!(combine_verdicts(Some(true), lookups), Some(true));
        assert_eq!(calls.get(), 0);
    }
}

#[cfg(test)]
#[path = "dictionary_cache_tests.rs"]
mod cache_tests;
