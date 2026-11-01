//! Floats and 64-bit keys as hex text, for data that passes through the page's JavaScript: the worker's kept state
//! (`career::state`) and the pairwise-ceiling table in data.json (`pb`). JavaScript's numbers cannot hold a 64-bit key,
//! and its JSON writes an infinite float (a search's `next_bound` or `cert`, a step's careers) as null and -0.0 as 0;
//! a float's bit pattern as 16 hex digits comes back what left, bit for bit, whatever its value.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

/// A float as its bit pattern in 16 hex digits (`f64::to_bits`).
#[derive(Clone, Copy, Debug, PartialEq, TS)]
#[ts(type = "string")]
pub(crate) struct Bits(pub(crate) f64);

/// A 64-bit key (a hash) in 16 hex digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(type = "string")]
pub(crate) struct Key(pub(crate) u64);

/// `value`'s 16 hex digits.
pub(crate) fn f64_hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The float of 16 hex digits (`f64_hex`); None for other text.
pub(crate) fn hex_f64(text: &str) -> Option<f64> {
    (text.len() == 16).then(|| u64::from_str_radix(text, 16).ok().map(f64::from_bits)).flatten()
}

impl Serialize for Bits {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&f64_hex(self.0))
    }
}

impl<'de> Deserialize<'de> for Bits {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        hex_f64(&text).map(Self).ok_or_else(|| serde::de::Error::custom(format!("not a float's bits: {text}")))
    }
}

impl Serialize for Key {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:016x}", self.0))
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        u64::from_str_radix(&text, 16).map(Self).map_err(|_| serde::de::Error::custom(format!("not a key: {text}")))
    }
}

/// Floats as their bits, in order.
pub(crate) fn bits(values: &[f64]) -> Vec<Bits> {
    values.iter().map(|&value| Bits(value)).collect()
}
