use crate::{FrozenPlan, Result, require};
use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::Path;

const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| format!("open {}: {error}", path.display()))?
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    require(
        bytes.len() as u64 <= MAX_JSON_BYTES,
        "JSON exceeds the input byte limit",
    )?;
    parse_json(&bytes).map_err(|error| format!("parse {}: {error}", path.display()).into())
}

pub fn parse_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let value: UniqueValue = serde_json::from_slice(bytes)?;
    Ok(serde_json::from_value(value.0)?)
}

pub fn write_new<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    require(
        bytes.len() as u64 <= MAX_JSON_BYTES,
        "JSON exceeds the output byte limit",
    )?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("write {}: {error}", path.display()))?;
    Ok(())
}

pub fn load_plan(path: &Path, expected: &str) -> Result<FrozenPlan> {
    let plan: FrozenPlan = read_json(path)?;
    plan.validate()?;
    require(plan.sha256()? == expected, "frozen plan digest mismatch")?;
    Ok(plan)
}

struct UniqueValue(Value);
struct UniqueVisitor;

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueValue;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("JSON without duplicate keys")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<Self::Value, M::Error> {
        let mut object = serde_json::Map::new();
        while let Some((key, value)) = map.next_entry::<String, UniqueValue>()? {
            if object.insert(key.clone(), value.0).is_some() {
                return Err(serde::de::Error::custom(format!("duplicate JSON key: {key}")));
            }
        }
        Ok(UniqueValue(Value::Object(object)))
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> std::result::Result<Self::Value, S::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<UniqueValue>()? {
            values.push(value.0);
        }
        Ok(UniqueValue(Value::Array(values)))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> std::result::Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value.to_owned())))
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> std::result::Result<Self::Value, E> {
        Ok(UniqueValue(Value::Bool(value)))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> std::result::Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> std::result::Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> std::result::Result<Self::Value, E> {
        serde_json::Number::from_f64(value)
            .map(|number| UniqueValue(Value::Number(number)))
            .ok_or_else(|| serde::de::Error::custom("nonfinite JSON number"))
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }
}
