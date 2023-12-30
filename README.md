# Serde SBIF &emsp; [![Build](https://github.com/k2green/serde-sbif/actions/workflows/build.yml/badge.svg)](https://github.com/k2green/serde-sbif/actions/workflows/build.yml)

**Serde SBIF is a crate that serializes data into a compact format inspired by the [NBT format](https://minecraft.fandom.com/wiki/NBT_format) from the game Minecraft.**

## Usage
To use the crate add it to your Cargo.toml file along with the latest version of the Serde crate.

```toml
[dependencies]
serde_sbif = { git = "https://github.com/k2green/serde-sbif/" }
```

Data can then be serialized into SBIF using the serde_sbif::to_bytes and serde_sbif::to_writer functions. Data can also be deserialized using the serde_sbif::from_slice and serde_sbif::from_reader functions.

```rust
use serde::{Serialize, Deserialize};
use serde_sbif::{to_bytes, Result, Compression};

#[derive(Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
}

fn serialize() -> Result<Vec<u8>> {
    // Some data structure.
    let address = Address {
        street: "10 Downing Street".to_owned(),
        city: "London".to_owned(),
    };

    // Serialize it to a SBIF byte vec.
    let serialized = serde_sbif::to_bytes(&address, Compression::default())?;
    
    Ok(serialized)
}
```

## SBIF Format
The Structured Binary Interchange Format (SBIF) is a format intended to store large amounts of structured data in either a compressed or uncompressed state.

An SBIF file consists of a short header of 8-12 bytes that hold the version number and compression format followed by blocks of data marked by an id. The id is a single byte which identifies what the following bytes represent and are laid out as follows:

| ID | Name | Description |
| ----------- | ----------- | ----------- |
| 0 | Null | This is a single byte block which represents a variety of "null" like objects in rust including Option::None, () and unit structs.|
| 1 | Bool | 1 marks a bool and should be followed by a byte that is either 0 for false or any non zero number for true|
| 2 | i8 | This ID marks the following byte as a signed 8 bit value. |
| 3 | i16 | This ID marks the following 2 bytes as a signed 16 bit value in big endean byte order. |
| 4 | i32 | This ID marks the following 4 bytes as a signed 32 bit value in big endean byte order. |
| 5 | i64 | This ID marks the following 8 bytes as a signed 64 bit value in big endean byte order. |
| 6 | ui8 | This ID marks the following byte as an unsigned 8 bit value. |
| 7 | u16 | This ID marks the following 2 bytes as an unsigned 16 bit value in big endean byte order. |
| 8 | u32 | This ID marks the following 4 bytes as an unsigned 32 bit value in big endean byte order. |
| 9 | u64 | This ID marks the following 8 bytes as an unsigned 64 bit value in big endean byte order. |
| 10 | f32 | This ID marks the following 4 bytes as a 32 bit floating point value in big endean byte order. |
| 11 | f64 | This ID marks the following 8 bytes as a 64 bit floating point in big endean byte order. |
| 12 | char | This is a Utf8 character represented by the following 1-4 bytes (based on the Utf8 specification) |
| 13 | str | Strings should be followed by the length of the string in bytes as a u32 in big endean byte order. The length should then be followed by the string. |
| 14 | bytes | Raw byte sequences are represented in the same way as strings. The ID should be followed by the length of the sequence as a big endean u32 and the sequence of bytes should follow after that. |
| 15 | seq | Sequences follow a similar pattern. The ID should be followed by a u32 length like in strings however this length is the number of distinct items in the sequence, not the length in bytes. This should be followed by a sequence of nested serialized objects. |
| 16 | Tuple | Tuples follow the same pattern as sequences. The ID is followed by the number of items and the length is followed by each item serialized in sequence. |
| 17 | Unit variant | Unit enum variants use a unique ID to make deserialization easier. The ID should be followed by a big endean u32 which represents the specific variant of the enum. |
| 18 | Enum variant | Enum variants start the same as a unit variant with the id followed by the variant as a u32 however the following data depends on the variant type. See Seq or Map. |
| 19 | Tuple struct | This structure is similar to a tuple, the ID should be followed by a big endean u32 which represents the number of elements which should be followed by a sequence of serialized items. |
| 20 | Map | Maps and structs are both represented by the map id. The ID should be followed by the number of key value pairs as a big endean u32. This should then be followed by the key value pairs serialized in sequence. |