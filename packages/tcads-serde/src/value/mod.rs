use indexmap::IndexMap;
use indexmap::map::Slice;
use std::ops::{Index, IndexMut};

pub mod de;
pub mod number;
pub mod ser;
pub mod visitors;

pub use number::{Float, Integer, Number, SignedInteger, UnsignedInteger};

/// A dynamically typed TwinCAT Runtime ADS value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Struct(IndexMap<String, Value>),
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

    pub const fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    pub const fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub const fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub const fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    pub const fn is_struct(&self) -> bool {
        matches!(self, Value::Struct(_))
    }

    pub const fn is_enum(&self) -> bool {
        matches!(self, Value::Enum(_))
    }

    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub const fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    pub const fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub const fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    pub fn as_struct(&self) -> Option<&Slice<String, Value>> {
        match self {
            Value::Struct(fields) => Some(fields.as_slice()),
            _ => None,
        }
    }

    pub fn as_enum(&self) -> Option<&str> {
        match self {
            Value::Enum(name) => Some(name),
            _ => None,
        }
    }

    pub const fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub const fn as_number_mut(&mut self) -> Option<&mut Number> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    pub const fn as_array_mut(&mut self) -> Option<&mut [Value]> {
        match self {
            Value::Array(arr) => Some(arr.as_mut_slice()),
            _ => None,
        }
    }

    pub fn as_struct_mut(&mut self) -> Option<&mut Slice<String, Value>> {
        match self {
            Value::Struct(fields) => Some(fields.as_mut_slice()),
            _ => None,
        }
    }

    pub fn as_enum_mut(&mut self) -> Option<&mut String> {
        match self {
            Value::Enum(name) => Some(name),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Struct(map) => map.get(key),
            Value::Array(arr) => key.parse::<usize>().ok().and_then(|idx| arr.get(idx)),
            _ => None,
        }
    }

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
