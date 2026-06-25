//! Named-entity recognition output types.
//!
//! Produced by the NER post-processor (`crates/xberg/src/text/ner/`) and
//! attached to [`ExtractionResult::entities`](super::extraction::ExtractionResult::entities).
//! Backends (`xberg-gliner` ONNX, LLM-driven) share a common `NerBackend`
//! trait so the redaction post-processor can consume the same entity stream.

use serde::{Deserialize, Serialize};

/// A single named entity detected in the extracted text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct Entity {
    /// Canonical category the entity belongs to (PERSON, ORG, LOCATION, etc.).
    pub category: EntityCategory,
    /// Raw mention text exactly as it appeared in the source.
    pub text: String,
    /// Byte-offset span in `ExtractionResult::content` where the mention starts.
    pub start: u32,
    /// Byte-offset span in `ExtractionResult::content` where the mention ends (exclusive).
    pub end: u32,
    /// Backend-reported confidence in `[0.0, 1.0]`. `None` when the backend does not
    /// expose confidence scores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Standard entity categories produced by built-in NER backends.
///
/// The `Custom(String)` variant lets caller-supplied categories (e.g. LLM
/// schemas) flow through without losing fidelity to the consumer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum EntityCategory {
    /// A person's name.
    Person,
    /// A company, institution, or organisation name.
    Organization,
    /// A geographic location (city, country, address).
    Location,
    /// A calendar date.
    Date,
    /// A time of day or duration.
    Time,
    /// A monetary amount with optional currency.
    Money,
    /// A percentage value.
    Percent,
    /// An email address.
    Email,
    /// A phone number.
    Phone,
    /// A URL or URI.
    Url,
    /// A caller-supplied custom category label.
    Custom(String),
}

impl Default for EntityCategory {
    fn default() -> Self {
        Self::Custom(String::new())
    }
}

impl From<String> for EntityCategory {
    fn from(s: String) -> Self {
        match s.as_str() {
            "person" => Self::Person,
            "organization" => Self::Organization,
            "location" => Self::Location,
            "date" => Self::Date,
            "time" => Self::Time,
            "money" => Self::Money,
            "percent" => Self::Percent,
            "email" => Self::Email,
            "phone" => Self::Phone,
            "url" => Self::Url,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::str::FromStr for EntityCategory {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s.to_string()))
    }
}
