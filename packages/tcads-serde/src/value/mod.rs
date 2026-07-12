use indexmap::IndexMap;
use indexmap::map::Slice;
use std::ops::{Index, IndexMut};

pub mod de;
pub mod number;
pub mod ser;
pub mod visitors;

pub use number::{Float, Integer, Number, SignedInteger, UnsignedInteger};

/// A dynamically typed TwinCAT Runtime ADS value.
///
/// [`Value`] is used to parse raw PLC memory when the layout is unknown at compile time.
/// Instead of strictly binding to a `#[derive(serde::Deserialize)]` Rust struct, memory parsed
/// into a [`Value`] becomes an explorable, JSON-like tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// An IEC 61131-3 `BOOL`.
    Bool(bool),
    /// An IEC 61131-3 numeric type (`INT`, `LREAL`, `BYTE`, etc.).
    Number(Number),
    /// An IEC 61131-3 `STRING` or `WSTRING`.
    String(String),
    /// A TwinCAT `ARRAY` chunked into a dynamic list of values.
    Array(Vec<Value>),
    /// A TwinCAT `STRUCT` or `FUNCTION_BLOCK`. Fields are stored in an `IndexMap`
    /// to preserve the PLC's internal memory declaration order.
    Struct(IndexMap<String, Value>),
    /// A TwinCAT `ENUM` variant, represented by its string name.
    Enum(String),
}

impl Value {
    /// Creats struct from pairs.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use tcads_serde::Value;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let point = Value::struct_from([("x", 1.0), ("y", 2.0)]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn struct_from<K, V, I>(pairs: I) -> Self
    where
        K: Into<String>,
        V: Into<Value>,
        I: IntoIterator<Item = (K, V)>,
    {
        Value::Struct(
            pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }

    /// Returns `true` if the value is a `Bool`.
    pub const fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns `true` if the value is a `Number`.
    pub const fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns `true` if the value is a `String`.
    pub const fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns `true` if the value is an `Array`.
    pub const fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Returns `true` if the value is a `Struct`.
    pub const fn is_struct(&self) -> bool {
        matches!(self, Value::Struct(_))
    }

    /// Returns `true` if the value is an `Enum`.
    pub const fn is_enum(&self) -> bool {
        matches!(self, Value::Enum(_))
    }

    /// If the [`Value`] is a `Bool`, returns the associated `bool`. Returns `None` otherwise.
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// If the [`Value`] is a `Number`, returns the associated `Number`. Returns `None` otherwise.
    pub const fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    /// If the `Value` is a `String`, returns the associated string slice. Returns `None` otherwise.
    pub const fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// If the `Value` is an `Array`, returns the underlying vector as a slice. Returns `None` otherwise.
    pub const fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    /// If the `Value` is a `Struct`, returns a reference to the underlying index map as a slice of
    /// key-value pairs. Returns `None` otherwise.
    pub fn as_struct(&self) -> Option<&Slice<String, Value>> {
        match self {
            Value::Struct(fields) => Some(fields.as_slice()),
            _ => None,
        }
    }

    /// If the `Value` is an `Enum`, returns the string name of the variant. Returns `None` otherwise.
    pub fn as_enum(&self) -> Option<&str> {
        match self {
            Value::Enum(name) => Some(name),
            _ => None,
        }
    }

    /// If the `Value` is a `Bool`, returns a mutable reference to the associated `bool`. Returns
    /// `None` otherwise.
    pub const fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// If the `Value` is a `Number`, returns a mutable reference to the underlying numeric enum.
    /// Returns `None` otherwise.
    pub const fn as_number_mut(&mut self) -> Option<&mut Number> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    /// If the `Value` is an `Array`, returns a mutable reference to the underlying vector slice.
    /// Returns `None` otherwise.
    pub const fn as_array_mut(&mut self) -> Option<&mut [Value]> {
        match self {
            Value::Array(arr) => Some(arr.as_mut_slice()),
            _ => None,
        }
    }

    /// If the `Value` is a `Struct`, returns a mutable reference to the underlying index map slice.
    /// Returns `None` otherwise.
    pub fn as_struct_mut(&mut self) -> Option<&mut Slice<String, Value>> {
        match self {
            Value::Struct(fields) => Some(fields.as_mut_slice()),
            _ => None,
        }
    }

    /// If the `Value` is an `Enum`, returns a mutable reference to the string name of the variant.
    /// Returns `None` otherwise.
    pub fn as_enum_mut(&mut self) -> Option<&mut String> {
        match self {
            Value::Enum(name) => Some(name),
            _ => None,
        }
    }

    /// Accesses a nested `Value` by its string key.
    ///
    /// - If the value is a `Struct`, this looks up the child field by its exact name.
    /// - If the value is an `Array`, this attempts to parse the string key as a `usize` index
    ///   and fetches that element.
    /// - Returns `None` for all other variants, or if the key/index does not exist.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Struct(map) => map.get(key),
            Value::Array(arr) => key.parse::<usize>().ok().and_then(|idx| arr.get(idx)),
            _ => None,
        }
    }

    /// Accesses a mutable reference to a nested `Value` by its string key.
    ///
    /// - If the value is a `Struct`, this looks up the child field by its exact name.
    /// - If the value is an `Array`, this attempts to parse the string key as a `usize` index and
    ///   fetches that element.
    /// - Returns `None` for all other variants, or if the key/index does not exist.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Struct(map) => map.get_mut(key),
            Value::Array(arr) => key
                .parse::<usize>()
                .ok()
                .and_then(move |idx| arr.get_mut(idx)),
            _ => None,
        }
    }
}

impl Index<&str> for Value {
    type Output = Value;

    fn index(&self, index: &str) -> &Value {
        self.get(index).expect("Value does not contain key/index")
    }
}

impl IndexMut<&str> for Value {
    fn index_mut(&mut self, index: &str) -> &mut Value {
        self.get_mut(index)
            .expect("Value does not contain key/index")
    }
}

impl Index<usize> for Value {
    type Output = Value;

    fn index(&self, index: usize) -> &Value {
        self.as_array()
            .expect("Value is not an array")
            .get(index)
            .expect("Index out of bounds")
    }
}

impl IndexMut<usize> for Value {
    fn index_mut(&mut self, index: usize) -> &mut Value {
        self.as_array_mut()
            .expect("Value is not an array")
            .get_mut(index)
            .expect("Index out of bounds")
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl<T> From<T> for Value
where
    Number: From<T>,
{
    fn from(n: T) -> Self {
        Value::Number(Number::from(n))
    }
}

impl From<IndexMap<String, Value>> for Value {
    fn from(m: IndexMap<String, Value>) -> Self {
        Value::Struct(m)
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Value::Array(iter.into_iter().map(Into::into).collect())
    }
}
