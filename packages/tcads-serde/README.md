# TwinCAT ADS Serde

This crate provides a [`serde`](https://serde.rs) `Serializer`/`Deserializer` pair for
**TwinCAT ADS** byte layouts, driven entirely by the PLC's own type metadata
(`AdsTypeInfo`). Give it any `#[derive(serde::Serialize, serde::Deserialize)]` type and the
type information for a PLC symbol, and it reads and writes the exact byte layout TwinCAT
expects.

For situations where you don't have, or don't want, a static Rust type, the crate also
provides `Value`: a dynamically typed value that mirrors whatever PLC data you read
without losing width or shape information.

## Features

- Read and write any `#[derive(serde::Serialize, serde::Deserialize)]` type directly
  against raw ADS bytes: primitives, structs, tuples, tuple structs, arrays, `Option`
  (with a caveat, see below), and unit-only enums.
- Struct fields match the PLC type by declared position, not by name.
- Arrays of any dimension are supported, including nested (`ARRAY[*] OF ARRAY[*]`) and
  multi-dimensional (`ARRAY[*, *]`) declarations, of primitives, strings, structs, or
  enums.
- `STRING`/`WSTRING` are decoded and encoded automatically (Windows-1252 and UTF-16LE
  respectively), with zero-copy reads for ASCII `STRING` values.
- Aliases resolve automatically. A `TYPE Temperature : LREAL; END_TYPE` reads and writes
  exactly like a plain `LREAL`.
- `Value`'s numeric types remember whether they came from a `BYTE` or a `DWORD`, so
  writing one back validates against the same field width it was read from.
- Type metadata comes from the `TypeProvider` trait, so you can plug in your own source.
  `AdsTypeCache` is included as a ready-made in-memory implementation.

## Documentation

Generate documentation with `cargo doc --open` and explore the API reference.

## Crate Layout

```text
tcads-serde/
  ├── de/              # AdsDeserializer + the access types (array/struct/map/enum) it drives
  ├── ser/             # AdsSerializer + the access types it drives
  ├── value/           # The dynamic `Value` type and its Number/Integer/Float model
  ├── resolvers.rs     # Alias resolution, shared by ser and de
  ├── validators.rs    # Type/size/category validation, shared by ser and de
  ├── type_provider.rs # The `TypeProvider` trait
  ├── type_cache.rs    # `AdsTypeCache`, a ready-made in-memory `TypeProvider`
  └── error.rs
```

## Quick Start

### Reading a symbol into a typed struct

```rust
use tcads::client::devices::blocking::RuntimeDevice;
use tcads::core::*;
use tcads_serde::{AdsTypeCache, TypeProvider};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LibVersion {
  pub major: u16,
  pub minor: u16,
  pub build: u16,
  pub revision: u16,
  pub flags: u32,
  pub version_string: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let device = RuntimeDevice::connect(AmsAddr::from_local(851), None)?;

  let mut provider =
          AdsTypeCache::new(device.get_upload_info()?.platform_ptr_size().unwrap_or(8));
  provider.insert_all(device.get_all_type_infos()?.filter_map(|res| res.ok()));

  let sym_info = device.get_symbol_info("MAIN.stVersion")?;
  let type_info = provider.get_type_info(sym_info.type_name()).unwrap();

  let bytes = device.read_bytes_by_info(&sym_info)?;
  let version: LibVersion = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;

  println!("{version:#?}");

  Ok(())
}
```

Note that `LibVersion` doesn't try to match the PLC struct's field names at all. Only the
field count, order, and types need to line up. That's the position-based matching
mentioned above, not a coincidence.

### Writing a value back

```rust
use tcads_serde::to_vec;

let bytes = to_vec(&version, &type_info, &provider)?;
device.write_bytes_by_info(&sym_info, bytes)?;
```

`to_vec` allocates a fresh, zero-initialized buffer on every call. If you're writing
repeatedly and want to reuse one buffer instead, use `to_bytes(&value, &type_info,
&provider, &mut buf)`.

### Reading into a dynamic `Value`

When you don't have, or don't want, a matching Rust type:

```rust
use tcads_serde::Value;

let value: Value = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;

println!("{:#?}", value);
// version["iMajor"].as_number()  -> width-preserving access, e.g. Some(UInt(3))
// version["sVersion"].as_str()   -> Some("3.4.5.0")
```

### Arrays

```rust
// ARRAY[0..4] OF LREAL
let readings: Vec<f64> = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;

// ARRAY[0..2] OF ARRAY[0..3] OF INT
let grid: Vec<Vec<i16>> = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;

// Fixed-size arrays work too, matched by declared length
let readings: [f64; 5] = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;
```

### Enums

```rust
#[derive(Debug, Serialize, Deserialize)]
enum State {
    Idle,
    Running,
    Faulted,
}
```

PLC enums carry no payload, so only unit variants are supported. `State::Idle` maps
directly to the matching PLC enum member by name. Serializing or deserializing a variant
with data (`Faulted(String)`) returns an error rather than silently dropping the payload.

> [!NOTE]
> `Option<T>` has the same restriction in the other direction: PLC memory has no
> representation for "no value," so `None` fails to serialize rather than being written as
> a zero that would be indistinguishable from a real value on read-back. Model an
> optional PLC value with an explicit presence field (a `BOOL`, a status/quality byte)
> instead of `Option`.

## Renaming and skipping fields

A couple of serde's field and variant attributes are worth knowing about, since how they
behave here depends on the position-based matching described above.

`#[serde(rename = "...")]` on an enum variant works exactly as you'd expect, since enum
variants are already matched by name:

```rust
#[derive(Debug, Serialize, Deserialize)]
enum State {
    #[serde(rename = "eIdle")]
    Idle,
    #[serde(rename = "eRunning")]
    Running,
}
```

Struct fields work differently. Since matching is positional, `#[serde(rename = "...")]`
on a struct field has no effect, matching depends only on position and type, never on a
name. `#[serde(skip)]` still behaves as you'd hope, though: a skipped field is removed
from the sequence entirely rather than leaving a gap behind, so you can add a Rust-only
field anywhere in the struct without disturbing the PLC fields around it:

```rust
#[derive(Debug, Serialize, Deserialize)]
struct Motor {
    speed: f32,
    #[serde(skip)]
    last_seen: Option<std::time::Instant>,
    running: bool,
}
```

If you want to address struct fields by their actual PLC name instead, for example
because your Rust and PLC field orders have drifted apart, deserialize into `Value`
first. `Value::Struct` keeps the PLC's real field names as map keys, and from there you
can bridge into a renamed, name-matched struct through any ordinary serde format:

```rust
#[derive(Debug, Serialize, Deserialize)]
struct LibVersion {
    #[serde(rename = "iMajor")]
    major: u16,
    #[serde(rename = "iMinor")]
    minor: u16,
    #[serde(rename = "sVersion")]
    version_string: String,
}

let value: tcads_serde::Value = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;
let version: LibVersion = serde_json::from_value(serde_json::to_value(&value)?)?;
```

This costs an extra allocation and a JSON round trip (add `serde_json` as a dependency to
use it), so it's worth reaching for only when position-based matching genuinely doesn't
fit.