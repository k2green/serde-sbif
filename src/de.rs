use std::{
    io::{Cursor, Read},
    vec,
};

use byteorder::ReadBytesExt;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use peekread::{BufPeekReader, PeekRead};
use serde::{de::IntoDeserializer, Deserialize};

use crate::{data_ids, ByteOrder, Compression, Error, FileHeader};

/// Deserializes a value from a byte slice.
pub fn from_slice<'a, T: Deserialize<'a>>(bytes: &[u8]) -> Result<T, Error> {
    let mut cursor = Cursor::new(bytes);
    let mut deserializer = Deserializer::new(&mut cursor)?;
    T::deserialize(&mut deserializer)
}

/// Deserializes a value from a reader.
pub fn from_reader<'a, R: Read, T: Deserialize<'a>>(reader: R) -> Result<T, Error> {
    let mut deserializer = Deserializer::new(reader)?;
    T::deserialize(&mut deserializer)
}

enum Reader<R: Read> {
    None(R),
    Deflate(DeflateDecoder<R>),
    GZip(GzDecoder<R>),
    ZLib(ZlibDecoder<R>),
}

impl<R: Read> Read for Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::None(r) => r.read(buf),
            Self::Deflate(r) => r.read(buf),
            Self::GZip(r) => r.read(buf),
            Self::ZLib(r) => r.read(buf),
        }
    }
}

/// A deserializer for the SBIF format.
pub struct Deserializer<R: Read>(BufPeekReader<Reader<R>>);

impl<R: Read> Deserializer<R> {
    /// Creates a new deserializer from a reader, the reader must be at the start of the SBIF file and the method will return an error if the header is invalid.
    /// The compression type will be obtained from the header.
    /// 
    /// Example
    /// ```
    /// use serde_sbif::Deserializer;
    /// fn deserialize_from_bytes<'a, T: serde::Deserialize<'a>>(bytes: &[u8]) -> T {
    ///     let mut cursor = std::io::Cursor::new(bytes);
    ///     let mut deserializer = Deserializer::new(&mut cursor).unwrap();
    ///     T::deserialize(&mut deserializer)
    /// }
    /// ```
    pub fn new(mut reader: R) -> Result<Self, Error> {
        let header = FileHeader::from_reader(&mut reader)?;

        if header.header_name != "SBIF" {
            return Err(Error::InvalidHeader(header.header_name));
        } else if header.version != 1 {
            return Err(Error::InvalidVersion {
                expected: 1,
                found: header.version,
            });
        }

        let reader = match header.compression {
            Compression::None => BufPeekReader::new(Reader::None(reader)),
            Compression::Deflate(_) => {
                BufPeekReader::new(Reader::Deflate(DeflateDecoder::new(reader)))
            }
            Compression::GZip(_) => BufPeekReader::new(Reader::GZip(GzDecoder::new(reader))),
            Compression::ZLib(_) => BufPeekReader::new(Reader::ZLib(ZlibDecoder::new(reader))),
        };

        Ok(Self(reader))
    }
}

impl<'de, 'a, R: Read> serde::de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let id = self.0.peek().read_u8().map_err(Error::IoError)?;
        match id {
            data_ids::NULL_ID => self.deserialize_option(visitor),
            data_ids::BOOL_ID => self.deserialize_bool(visitor),
            data_ids::I8_ID => self.deserialize_i8(visitor),
            data_ids::I16_ID => self.deserialize_i16(visitor),
            data_ids::I32_ID => self.deserialize_i32(visitor),
            data_ids::I64_ID => self.deserialize_i64(visitor),
            data_ids::U8_ID => self.deserialize_u8(visitor),
            data_ids::U16_ID => self.deserialize_u16(visitor),
            data_ids::U32_ID => self.deserialize_u32(visitor),
            data_ids::U64_ID => self.deserialize_u64(visitor),
            data_ids::F32_ID => self.deserialize_f32(visitor),
            data_ids::F64_ID => self.deserialize_f64(visitor),
            data_ids::CHAR_ID => self.deserialize_char(visitor),
            data_ids::STR_ID => self.deserialize_str(visitor),
            data_ids::BYTES_ID => self.deserialize_bytes(visitor),
            data_ids::SEQ_ID => self.deserialize_seq(visitor),
            data_ids::MAP_ID => self.deserialize_map(visitor),
            data_ids::TUPLE_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
                visitor.visit_seq(SeqAccess::new(self, length))
            }
            data_ids::UNIT_VARIANT_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                let variant = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?;
                visitor.visit_enum(variant.into_deserializer())
            }
            data_ids::ENUM_VARIANT_ID => visitor.visit_enum(EnumAccess { de: self }),
            data_ids::TUPLE_STRUCT_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
                visitor.visit_seq(SeqAccess::new(self, length))
            }
            found => Err(Error::InvalidDataId {
                expected: format!("from {} to {}", data_ids::NULL_ID, data_ids::MAP_ID),
                found,
            }),
        }
    }

    fn deserialize_bool<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::BOOL_ID)?;
        visitor.visit_bool(self.0.read_u8().map_err(Error::IoError)? != 0)
    }

    fn deserialize_i8<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I8_ID)?;
        visitor.visit_i8(self.0.read_i8().map_err(Error::IoError)?)
    }

    fn deserialize_i16<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I16_ID)?;
        visitor.visit_i16(self.0.read_i16::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_i32<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I32_ID)?;
        visitor.visit_i32(self.0.read_i32::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_i64<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I64_ID)?;
        visitor.visit_i64(self.0.read_i64::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_u8<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U8_ID)?;
        visitor.visit_u8(self.0.read_u8().map_err(Error::IoError)?)
    }

    fn deserialize_u16<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U16_ID)?;
        visitor.visit_u16(self.0.read_u16::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_u32<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U32_ID)?;
        visitor.visit_u32(self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_u64<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U64_ID)?;
        visitor.visit_u64(self.0.read_u64::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_f32<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::F32_ID)?;
        visitor.visit_f32(self.0.read_f32::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_f64<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::F64_ID)?;
        visitor.visit_f64(self.0.read_f64::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_char<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::CHAR_ID)?;
        let mut bytes = vec![self.0.read_u8().map_err(Error::IoError)?];

        if bytes[0] & 0b1110_0000 == 0b1100_0000 {
            bytes.push(self.0.read_u8().map_err(Error::IoError)?);
        } else if bytes[0] & 0b1111_0000 == 0b1110_0000 {
            bytes.push(self.0.read_u8().map_err(Error::IoError)?);
            bytes.push(self.0.read_u8().map_err(Error::IoError)?);
        } else if bytes[0] & 0b1111_1000 == 0b1111_0000 {
            bytes.push(self.0.read_u8().map_err(Error::IoError)?);
            bytes.push(self.0.read_u8().map_err(Error::IoError)?);
            bytes.push(self.0.read_u8().map_err(Error::IoError)?);
        }

        let string = String::from_utf8(bytes).map_err(Error::FromUtf8Error)?;
        visitor.visit_char(string.chars().next().unwrap())
    }

    fn deserialize_str<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::STR_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        let string = String::from_utf8(buffer).map_err(Error::FromUtf8Error)?;
        visitor.visit_str(&string)
    }

    fn deserialize_string<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::STR_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        visitor.visit_string(String::from_utf8(buffer).map_err(Error::FromUtf8Error)?)
    }

    fn deserialize_bytes<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::BYTES_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        visitor.visit_bytes(&buffer)
    }

    fn deserialize_byte_buf<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::BYTES_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        visitor.visit_byte_buf(buffer)
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let peek_id = self.0.peek().read_u8().map_err(Error::IoError)?;
        match peek_id {
            data_ids::NULL_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::NULL_ID)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::SEQ_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        visitor.visit_seq(SeqAccess::new(self, length))
    }

    fn deserialize_tuple<V: serde::de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::TUPLE_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        if length != len {
            return Err(Error::InvalidLength {
                expected: len,
                actual: length,
                message: String::from("Invalid tuple length"),
            });
        } else {
            visitor.visit_seq(SeqAccess::new(self, length))
        }
    }

    fn deserialize_tuple_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::TUPLE_STRUCT_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        if length != len {
            return Err(Error::InvalidLength {
                expected: len,
                actual: length,
                message: String::from("Invalid tuple struct length"),
            });
        } else {
            visitor.visit_seq(SeqAccess::new(self, length))
        }
    }

    fn deserialize_map<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::MAP_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        visitor.visit_map(MapAccess::new(self, length))
    }

    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::MAP_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        visitor.visit_map(MapAccess::new(self, length))
    }

    fn deserialize_enum<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let mut peek = self.0.peek();
        let data_id = peek.read_u8().map_err(Error::IoError)?;
        let variant_index = peek.read_u32::<ByteOrder>().map_err(Error::IoError)?;
        drop(peek);

        match data_id {
            data_ids::UNIT_VARIANT_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?;
                visitor.visit_enum(variants[variant_index as usize].into_deserializer())
            }
            data_ids::ENUM_VARIANT_ID => visitor.visit_enum(EnumAccess { de: self }),
            found => Err(Error::InvalidDataId {
                expected: format!(
                    "{} or {}",
                    data_ids::UNIT_VARIANT_ID,
                    data_ids::ENUM_VARIANT_ID
                ),
                found,
            }),
        }
    }

    fn deserialize_identifier<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let data_id = self.0.read_u8().map_err(Error::IoError)?;
        let argument = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?;

        match data_id {
            data_ids::STR_ID => {
                let mut buffer = vec![0_u8; argument as usize];
                self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
                let string = String::from_utf8(buffer).map_err(Error::FromUtf8Error)?;
                visitor.visit_str(&string)
            }
            data_ids::UNIT_VARIANT_ID | data_ids::ENUM_VARIANT_ID => visitor.visit_u32(argument),
            v => Err(Error::InvalidDataId {
                expected: String::from("an identifier"),
                found: v,
            }),
        }
    }

    fn deserialize_ignored_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }
}

struct SeqAccess<'a, R: Read> {
    de: &'a mut Deserializer<R>,
    len: usize,
    current: usize,
}

impl<'a, R: Read> SeqAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, len: usize) -> Self {
        Self {
            de,
            len,
            current: 0,
        }
    }
}

impl<'de, 'a, R: Read> serde::de::SeqAccess<'de> for SeqAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.current < self.len {
            self.current += 1;
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len - self.current)
    }
}

struct MapAccess<'a, R: Read> {
    de: &'a mut Deserializer<R>,
    len: usize,
    current_key: usize,
    current_value: usize,
}

impl<'a, R: Read> MapAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, len: usize) -> Self {
        Self {
            de,
            len,
            current_key: 0,
            current_value: 0,
        }
    }
}

impl<'de, 'a, R: Read> serde::de::MapAccess<'de> for MapAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if self.current_key < self.len {
            self.current_key += 1;
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        if self.current_value < self.len {
            self.current_value += 1;
            seed.deserialize(&mut *self.de)
        } else {
            Err(Error::InvalidMapAccess)
        }
    }
}

struct EnumAccess<'a, R: Read> {
    de: &'a mut Deserializer<R>,
}

impl<'de, 'a, R: Read> serde::de::EnumAccess<'de> for EnumAccess<'a, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: serde::de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de, 'a, R: Read> serde::de::VariantAccess<'de> for EnumAccess<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Err(Error::UnexpectedString)
    }

    fn newtype_variant_seed<T: serde::de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: serde::de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let length = self.de.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        if length != len {
            return Err(Error::InvalidLength {
                expected: len,
                actual: length,
                message: String::from("Invalid tuple variant length"),
            });
        } else {
            visitor.visit_seq(SeqAccess::new(&mut *self.de, length))
        }
    }

    fn struct_variant<V: serde::de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let length = self.de.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        visitor.visit_map(MapAccess::new(&mut *self.de, length))
    }
}

fn read_id<R: Read>(reader: &mut R, expected: u8) -> Result<(), Error> {
    let found = reader.read_u8().map_err(Error::IoError)?;
    if found == expected {
        Ok(())
    } else {
        Err(Error::InvalidDataId {
            expected: expected.to_string(),
            found,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use crate::{se::to_bytes, Compression};

    fn deserialization_test_base<T: Serialize + DeserializeOwned + PartialEq + Debug>(
        value: &T,
        compression: Compression,
    ) {
        let serialized = to_bytes(&value, compression).unwrap();
        let deserialized: T = crate::de::from_slice(&serialized).unwrap();
        assert_eq!(value, &deserialized);
    }

    fn deserialization_test<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: T) {
        deserialization_test_base(&value, Compression::None);
        deserialization_test_base(&value, Compression::Deflate(6));
        deserialization_test_base(&value, Compression::GZip(6));
        deserialization_test_base(&value, Compression::ZLib(6));
    }

    #[test]
    fn test_bool_deserialization() {
        deserialization_test(true);
        deserialization_test(false);
    }

    #[test]
    fn test_integer_deserialization() {
        deserialization_test(100_i8);
        deserialization_test(100_i16);
        deserialization_test(100_i32);
        deserialization_test(100_i64);
        deserialization_test(100_u8);
        deserialization_test(100_u16);
        deserialization_test(100_u32);
        deserialization_test(100_u64);
    }

    #[test]
    fn test_float_deserialization() {
        deserialization_test(100.0_f32);
        deserialization_test(100.0_f64);
    }

    #[test]
    fn test_char_deserialization() {
        deserialization_test('a'); // 1 byte
        deserialization_test('©'); // 2 bytes
        deserialization_test('थ'); // 3 bytes
        deserialization_test('🎨'); // 4 bytes
    }

    #[test]
    fn test_string_deserialization() {
        deserialization_test("Hello World!".to_string());
    }

    #[test]
    fn test_bytes_deserialization() {
        deserialization_test(b"Hello World!".to_vec());
    }

    #[test]
    fn test_seq_deserialization() {
        deserialization_test((0..10_u8).collect::<Vec<_>>());
        deserialization_test((0..10_i8).collect::<Vec<_>>());
        deserialization_test((0..10).collect::<Vec<_>>());
    }

    #[test]
    fn test_tuple_deserialization() {
        deserialization_test((0_u8, 'a', "Hello World!".to_string()));
    }

    #[test]
    fn test_struct_deserialization() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct UnitStruct;

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct NewtypeStruct(u8);

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct TupleStruct(u8, char, String);

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Struct {
            a: u8,
            b: char,
            c: String,
        }

        deserialization_test(UnitStruct);
        deserialization_test(NewtypeStruct(1));
        deserialization_test(TupleStruct(1, 'a', "Hello World!".to_string()));
        deserialization_test(Struct {
            a: 1,
            b: 'a',
            c: "Hello World!".to_string(),
        });
    }

    #[test]
    fn test_enum_deserialization() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum TestEnum {
            Unit,
            Newtype(u8),
            Tuple(u8, char, String),
            Struct { a: u8, b: char, c: String },
        }

        deserialization_test(TestEnum::Unit);
        deserialization_test(TestEnum::Newtype(1));
        deserialization_test(TestEnum::Tuple(1, 'a', "Hello World!".to_string()));
        deserialization_test(TestEnum::Struct {
            a: 1,
            b: 'a',
            c: "Hello World!".to_string(),
        });
    }

    #[test]
    fn test_option_deserialization() {
        deserialization_test(None::<u8>);
        deserialization_test(Some(1_u8));
    }
}
