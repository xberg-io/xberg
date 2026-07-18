//! Replacement strategy implementations.
//!
//! Maps each [`RedactionStrategy`] to a function that produces the replacement
//! token for a single matched span. The `TokenReplace` strategy is stateful
//! across a document — counters are stored in [`TokenCounter`] which the
//! engine threads through every match.

use crate::types::redaction::{PiiCategory, RedactionStrategy};
use ahash::AHashMap;
use sha2::{Digest, Sha256};

/// Per-category running counter for [`RedactionStrategy::TokenReplace`].
#[derive(Debug, Default, Clone)]
pub struct TokenCounter {
    counts: AHashMap<String, u32>,
    /// Per-category memoization so the same original value yields the same token
    /// twice in the same document.
    cache: AHashMap<(String, String), String>,
}

impl TokenCounter {
    /// Create a fresh counter with no previous state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the next token for `category` and `original`. If the original
    /// has been seen before in this category, the same token is reused.
    #[cfg_attr(alef, alef(skip))]
    pub fn next_token(&mut self, category: &PiiCategory, original: &str) -> String {
        let cat_label = category_label(category);
        let key = (cat_label.clone(), original.to_string());
        if let Some(existing) = self.cache.get(&key) {
            return existing.clone();
        }
        let count = self.counts.entry(cat_label.clone()).or_insert(0);
        *count += 1;
        let token = format!("[{}_{}]", cat_label.to_ascii_uppercase(), count);
        self.cache.insert(key, token.clone());
        token
    }

    /// Token to original-text view of every allocation made so far. The
    /// counter's dedup cache holds exactly the `TokenReplace` substitutions
    /// (no other strategy allocates tokens), so this is the complete
    /// rehydration map for the pass that used this counter.
    #[cfg(feature = "redaction-rehydrate")]
    #[cfg_attr(alef, alef(skip))]
    pub fn rehydration_map(&self) -> super::rehydration::RehydrationMap {
        self.cache
            .iter()
            .map(|((_category, original), token)| (token.clone(), original.clone()))
            .collect()
    }
}

/// Apply `strategy` to `original` for `category` and return the replacement token.
///
/// The optional `counter` is required for [`RedactionStrategy::TokenReplace`];
/// other strategies ignore it.
#[cfg_attr(alef, alef(skip))]
pub fn apply_strategy(
    strategy: RedactionStrategy,
    original: &str,
    category: &PiiCategory,
    counter: &mut TokenCounter,
) -> String {
    match strategy {
        RedactionStrategy::Mask => "[REDACTED]".to_string(),
        RedactionStrategy::Hash => {
            let mut hasher = Sha256::new();
            hasher.update(original.as_bytes());
            let digest = hasher.finalize();
            let hex = hex::encode(digest);
            format!("[HASH:{}]", &hex[..16])
        }
        RedactionStrategy::TokenReplace => counter.next_token(category, original),
        RedactionStrategy::Drop => String::new(),
    }
}

/// Stable lower-case label for a category used to build token names.
fn category_label(category: &PiiCategory) -> String {
    match category {
        PiiCategory::Email => "email".into(),
        PiiCategory::Phone => "phone".into(),
        PiiCategory::Ssn => "ssn".into(),
        PiiCategory::CreditCard => "credit_card".into(),
        PiiCategory::PostalCode => "postal_code".into(),
        PiiCategory::IpAddress => "ip_address".into(),
        PiiCategory::Iban => "iban".into(),
        PiiCategory::SwiftBic => "swift_bic".into(),
        PiiCategory::DateOfBirth => "date_of_birth".into(),
        PiiCategory::Person => "person".into(),
        PiiCategory::Organization => "organization".into(),
        PiiCategory::Location => "location".into(),
        PiiCategory::Custom(label) => label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_strategy() {
        let mut counter = TokenCounter::new();
        let token = apply_strategy(
            RedactionStrategy::Mask,
            "alice@example.com",
            &PiiCategory::Email,
            &mut counter,
        );
        assert_eq!(token, "[REDACTED]");
    }

    #[test]
    fn test_hash_strategy_is_deterministic() {
        let mut counter = TokenCounter::new();
        let a = apply_strategy(
            RedactionStrategy::Hash,
            "alice@example.com",
            &PiiCategory::Email,
            &mut counter,
        );
        let b = apply_strategy(
            RedactionStrategy::Hash,
            "alice@example.com",
            &PiiCategory::Email,
            &mut counter,
        );
        assert_eq!(a, b);
        assert!(a.starts_with("[HASH:"));
        assert_eq!(a.len(), "[HASH:0123456789abcdef]".len());
    }

    #[test]
    fn test_token_replace_reuses_for_same_value() {
        let mut counter = TokenCounter::new();
        let t1 = apply_strategy(
            RedactionStrategy::TokenReplace,
            "alice@example.com",
            &PiiCategory::Email,
            &mut counter,
        );
        let t2 = apply_strategy(
            RedactionStrategy::TokenReplace,
            "alice@example.com",
            &PiiCategory::Email,
            &mut counter,
        );
        let t3 = apply_strategy(
            RedactionStrategy::TokenReplace,
            "bob@example.com",
            &PiiCategory::Email,
            &mut counter,
        );
        assert_eq!(t1, "[EMAIL_1]");
        assert_eq!(t2, "[EMAIL_1]");
        assert_eq!(t3, "[EMAIL_2]");
    }

    #[test]
    fn test_drop_strategy() {
        let mut counter = TokenCounter::new();
        assert!(apply_strategy(RedactionStrategy::Drop, "x", &PiiCategory::Phone, &mut counter).is_empty());
    }
}
