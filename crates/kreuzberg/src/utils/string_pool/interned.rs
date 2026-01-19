//! Interned string type and trait implementations.
//!
//! This module provides the `InternedString` type which wraps an Arc<String>
//! to enable string deduplication and pointer-based comparisons.

use std::sync::Arc;

/// A reference to an interned string stored in an Arc.
///
/// This wraps an Arc<String> and provides convenient access to the string content.
/// Multiple calls with the same string content will share the same Arc, reducing memory usage.
#[derive(Clone)]
pub struct InternedString(pub(super) Arc<String>);

impl InternedString {
    /// Get the string content.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for InternedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InternedString").field(&self.as_str()).finish()
    }
}

impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}

impl Eq for InternedString {}

impl std::hash::Hash for InternedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl std::ops::Deref for InternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interned_string_display() {
        let s = InternedString(Arc::new("text/html".to_string()));
        assert_eq!(format!("{}", s), "text/html");
    }

    #[test]
    fn test_interned_string_deref() {
        let s = InternedString(Arc::new("application/json".to_string()));
        assert_eq!(&*s, "application/json");
        assert_eq!(s.as_ref(), "application/json");
        assert_eq!(s.as_str(), "application/json");
    }

    #[test]
    fn test_interned_string_hash() {
        use std::collections::HashSet;

        let s1 = InternedString(Arc::new("application/pdf".to_string()));
        let s2 = InternedString(Arc::clone(&s1.0));

        let mut set = HashSet::new();
        set.insert(s1);
        set.insert(s2);

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_interned_string_clone() {
        let s1 = InternedString(Arc::new("text/html".to_string()));
        let s2 = s1.clone();

        assert_eq!(s1, s2);
        assert!(Arc::ptr_eq(&s1.0, &s2.0));
    }
}
