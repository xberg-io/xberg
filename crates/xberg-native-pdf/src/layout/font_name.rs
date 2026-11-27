//! Shared font-name handle carried by text spans, glyphs and blocks.

use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// A font name shared by every span, glyph and block drawn in that font.
///
/// Cloning bumps a reference count instead of copying the name, so a page's
/// glyphs, and the span copies the page caches hand out, share one allocation
/// per font. It compares, hashes, orders, formats and serializes exactly as the
/// `str` it holds, so swapping it for a `String` changes no output.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontName(Arc<str>);

impl FontName {
    /// Create a font name holding a copy of `name`.
    pub fn new(name: &str) -> Self {
        Self(Arc::from(name))
    }

    /// The name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Return the entry of `names` equal to `name`, adding one when it is absent,
/// so repeated names share one allocation.
pub(crate) fn intern_font_name(names: &mut HashSet<FontName>, name: &str) -> FontName {
    if let Some(interned) = names.get(name) {
        return interned.clone();
    }
    let interned = FontName::new(name);
    names.insert(interned.clone());
    interned
}

impl Deref for FontName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for FontName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for FontName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl From<&str> for FontName {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<&String> for FontName {
    fn from(name: &String) -> Self {
        Self::new(name)
    }
}

impl From<String> for FontName {
    fn from(name: String) -> Self {
        Self(Arc::from(name))
    }
}

impl From<Arc<str>> for FontName {
    fn from(name: Arc<str>) -> Self {
        Self(name)
    }
}

impl From<FontName> for String {
    fn from(name: FontName) -> Self {
        String::from(&*name.0)
    }
}

impl From<&FontName> for String {
    fn from(name: &FontName) -> Self {
        String::from(&*name.0)
    }
}

impl PartialEq<str> for FontName {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for FontName {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for FontName {
    fn eq(&self, other: &String) -> bool {
        *self.0 == **other
    }
}

impl PartialEq<FontName> for str {
    fn eq(&self, other: &FontName) -> bool {
        self == &*other.0
    }
}

impl PartialEq<FontName> for &str {
    fn eq(&self, other: &FontName) -> bool {
        *self == &*other.0
    }
}

impl PartialEq<FontName> for String {
    fn eq(&self, other: &FontName) -> bool {
        **self == *other.0
    }
}

impl serde::Serialize for FontName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for FontName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, RandomState};

    #[test]
    fn clones_share_one_allocation() {
        let name = FontName::from("ABCDEF+CMR10");
        let copy = name.clone();
        assert!(std::ptr::eq(name.as_str().as_ptr(), copy.as_str().as_ptr()));
    }

    #[test]
    fn interning_returns_the_existing_name() {
        let mut names = HashSet::new();
        let first = intern_font_name(&mut names, "Helvetica");
        let second = intern_font_name(&mut names, "Helvetica");
        let other = intern_font_name(&mut names, "Times-Roman");
        assert!(std::ptr::eq(first.as_str().as_ptr(), second.as_str().as_ptr()));
        assert_eq!(other, "Times-Roman");
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn formats_like_the_string_it_replaces() {
        let name = FontName::from("Bold \"Quoted\" Font");
        let string = String::from("Bold \"Quoted\" Font");
        assert_eq!(format!("{name:?}"), format!("{string:?}"));
        assert_eq!(format!("{name}"), string);
        assert_eq!(name.to_string(), string);
    }

    #[test]
    fn serializes_like_the_string_it_replaces() {
        let name = FontName::from("Helvetica-Bold");
        assert_eq!(
            serde_json::to_string(&name).unwrap(),
            serde_json::to_string("Helvetica-Bold").unwrap()
        );
        let back: FontName = serde_json::from_str("\"Helvetica-Bold\"").unwrap();
        assert_eq!(back, name);
    }

    #[test]
    // The owned operands are the point: each comparison exercises one `FontName` impl. ~keep
    #[allow(clippy::cmp_owned)]
    fn hashes_and_orders_like_the_string_it_replaces() {
        let state = RandomState::new();
        let name = FontName::from("CMSY10");
        assert_eq!(state.hash_one(&name), state.hash_one(String::from("CMSY10")));
        let mut names = vec![FontName::from("b"), FontName::from("a"), FontName::from("c")];
        names.sort();
        assert_eq!(names, ["a", "b", "c"]);
        assert!(FontName::from("a") < FontName::from("b"));
    }

    #[test]
    #[allow(clippy::cmp_owned)]
    fn compares_with_strings_both_ways() {
        let name = FontName::from("Courier");
        assert!(name == "Courier");
        assert!("Courier" == name);
        assert!(name == String::from("Courier"));
        assert!(String::from("Courier") == name);
        assert!(*"Courier" == name);
        assert_eq!(String::from(&name), "Courier");
        assert_eq!(FontName::default(), "");
    }
}
