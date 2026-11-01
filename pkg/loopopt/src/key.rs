//! Content keys: a typed value hashed through its serde encoding (`content_key`), the key the worker keeps a search,
//! plan, roll or check under and finds it by in a later job. Every field the value serialises is in its key, by its
//! name and its value (a number by its bits, a string by its bytes, a variant by its name), and nothing else is: a job's
//! key does not read the text it came in as (key order, whitespace, a field left out or sent at its default). Keys
//! are only compared for equality (and stored, `career::state`), never ordered.
use serde::ser::{self, Serialize};
use std::hash::{DefaultHasher, Hasher};

/// The 64-bit key of `value`'s content.
pub(crate) fn content_key(value: &(impl Serialize + ?Sized)) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.serialize(&mut KeyEncoder { hasher: &mut hasher }).expect("a key's encoding cannot fail");
    hasher.finish()
}

/// What each piece of the encoding is, written before it, so no two different values write the same bytes (a
/// sequence's or map's end is marked, so `[[1], []]` and `[[], [1]]` differ).
#[derive(Clone, Copy)]
#[repr(u8)]
enum Mark {
    Bool = 1,
    Signed,
    Unsigned,
    Float,
    Text,
    Bytes,
    None,
    Some,
    Unit,
    Variant,
    Sequence,
    Map,
    End,
}

/// The serde serialiser that feeds a value's encoding to a hasher.
struct KeyEncoder<'hash> {
    hasher: &'hash mut DefaultHasher,
}

impl KeyEncoder<'_> {
    fn mark(&mut self, mark: Mark) {
        self.hasher.write_u8(mark as u8);
    }
    fn text(&mut self, text: &str) {
        self.mark(Mark::Text);
        self.hasher.write_usize(text.len());
        self.hasher.write(text.as_bytes());
    }
    fn variant(&mut self, name: &str) {
        self.mark(Mark::Variant);
        self.text(name);
    }
}

/// The encoder's error type: it never fails, but serde asks for one.
#[derive(Debug)]
pub(crate) struct KeyError;

impl std::fmt::Display for KeyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a key's encoding failed")
    }
}
impl std::error::Error for KeyError {}
impl ser::Error for KeyError {
    fn custom<T: std::fmt::Display>(_message: T) -> Self {
        Self
    }
}

impl ser::Serializer for &mut KeyEncoder<'_> {
    type Ok = ();
    type Error = KeyError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, value: bool) -> Result<(), KeyError> {
        self.mark(Mark::Bool);
        self.hasher.write_u8(u8::from(value));
        Ok(())
    }
    fn serialize_i8(self, value: i8) -> Result<(), KeyError> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i16(self, value: i16) -> Result<(), KeyError> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i32(self, value: i32) -> Result<(), KeyError> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i64(self, value: i64) -> Result<(), KeyError> {
        self.mark(Mark::Signed);
        self.hasher.write_i64(value);
        Ok(())
    }
    fn serialize_u8(self, value: u8) -> Result<(), KeyError> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u16(self, value: u16) -> Result<(), KeyError> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u32(self, value: u32) -> Result<(), KeyError> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u64(self, value: u64) -> Result<(), KeyError> {
        self.mark(Mark::Unsigned);
        self.hasher.write_u64(value);
        Ok(())
    }
    fn serialize_f32(self, value: f32) -> Result<(), KeyError> {
        self.serialize_f64(f64::from(value))
    }
    fn serialize_f64(self, value: f64) -> Result<(), KeyError> {
        self.mark(Mark::Float);
        self.hasher.write_u64(value.to_bits());
        Ok(())
    }
    fn serialize_char(self, value: char) -> Result<(), KeyError> {
        self.text(value.encode_utf8(&mut [0; 4]));
        Ok(())
    }
    fn serialize_str(self, value: &str) -> Result<(), KeyError> {
        self.text(value);
        Ok(())
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<(), KeyError> {
        self.mark(Mark::Bytes);
        self.hasher.write_usize(value.len());
        self.hasher.write(value);
        Ok(())
    }
    fn serialize_none(self) -> Result<(), KeyError> {
        self.mark(Mark::None);
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), KeyError> {
        self.mark(Mark::Some);
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), KeyError> {
        self.mark(Mark::Unit);
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), KeyError> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(self, _name: &'static str, _index: u32, variant: &'static str) -> Result<(), KeyError> {
        self.variant(variant);
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _name: &'static str, value: &T) -> Result<(), KeyError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), KeyError> {
        self.variant(variant);
        value.serialize(self)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self, KeyError> {
        self.mark(Mark::Sequence);
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self, KeyError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self, KeyError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self, KeyError> {
        self.variant(variant);
        self.serialize_seq(Some(len))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self, KeyError> {
        self.mark(Mark::Map);
        Ok(self)
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self, KeyError> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self, KeyError> {
        self.variant(variant);
        self.serialize_map(Some(len))
    }
}

/// A sequence's, tuple's or map's elements in turn, then its end.
macro_rules! elements {
    ($($trait:ident :: $method:ident),*) => {$(
        impl ser::$trait for &mut KeyEncoder<'_> {
            type Ok = ();
            type Error = KeyError;
            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), KeyError> {
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<(), KeyError> {
                self.mark(Mark::End);
                Ok(())
            }
        }
    )*};
}
elements!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field
);

impl ser::SerializeMap for &mut KeyEncoder<'_> {
    type Ok = ();
    type Error = KeyError;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), KeyError> {
        key.serialize(&mut **self)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), KeyError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), KeyError> {
        self.mark(Mark::End);
        Ok(())
    }
}

/// A struct's fields, each by its name then its value, then its end.
macro_rules! fields {
    ($($trait:ident),*) => {$(
        impl ser::$trait for &mut KeyEncoder<'_> {
            type Ok = ();
            type Error = KeyError;
            fn serialize_field<T: Serialize + ?Sized>(&mut self, name: &'static str, value: &T) -> Result<(), KeyError> {
                self.text(name);
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<(), KeyError> {
                self.mark(Mark::End);
                Ok(())
            }
        }
    )*};
}
fields!(SerializeStruct, SerializeStructVariant);

#[cfg(test)]
mod tests {
    use super::content_key;

    /// Equal content, equal key; any field, value or nesting changed, another key.
    #[test]
    fn keys_follow_content() {
        #[derive(serde::Serialize)]
        struct Job<'text> {
            name: &'text str,
            shares: [f64; 3],
            nested: Vec<Vec<u8>>,
            maybe: Option<u8>,
        }
        let job = |name, share, nested: Vec<Vec<u8>>, maybe| Job { name, shares: [share, 0.45, 0.05], nested, maybe };
        let base = content_key(&job("a", 0.5, vec![vec![1], vec![]], None));
        assert_eq!(base, content_key(&job("a", 0.5, vec![vec![1], vec![]], None)), "equal content");
        for (other, what) in [
            (job("b", 0.5, vec![vec![1], vec![]], None), "a string"),
            (job("a", 0.5 + f64::EPSILON, vec![vec![1], vec![]], None), "a float's last bit"),
            (job("a", 0.5, vec![vec![], vec![1]], None), "nesting"),
            (job("a", 0.5, vec![vec![1], vec![]], Some(0)), "None against Some(0)"),
        ] {
            assert_ne!(base, content_key(&other), "{what}");
        }
    }
}
