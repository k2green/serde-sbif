use std::{io::{Read, Cursor}, vec};

use byteorder::ReadBytesExt;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use peekread::{BufPeekReader, PeekRead, PeekCursor};
use serde::de::{DeserializeOwned, IntoDeserializer};

use crate::{FileHeader, Error, Compression, data_ids, ByteOrder};

pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    let mut cursor = Cursor::new(bytes);
    let mut deserializer = Deserializer::new(&mut cursor)?;
    T::deserialize(&mut deserializer)
}

enum Reader<'a> {
    Borrowed(BufPeekReader<&'a mut dyn Read>),
    Owned(BufPeekReader<Box<dyn Read + 'a>>),
}

impl<'a> Read for Reader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Reader::Borrowed(r) => r.read(buf),
            Reader::Owned(r) => r.read(buf),
        }
    }
}

impl<'a> Reader<'a> {
    fn peek(&mut self) -> PeekCursor {
        match self {
            Reader::Borrowed(r) => r.peek(),
            Reader::Owned(r) => r.peek(),
        }
    }
}

pub struct Deserializer<'a>(Reader<'a>);

impl<'a> Deserializer<'a> {
    pub fn new<R: Read + 'a>(reader: &'a mut R) -> Result<Self, Error> {
        let header = FileHeader::from_reader(reader)?;
        
        if header.header_name != "SBIF" {
            return Err(Error::InvalidHeader(header.header_name));
        } else if header.version != 1 {
            return Err(Error::InvalidVersion {
                expected: 1,
                found: header.version,
            });
        }

        let reader = match header.compression {
            Compression::None => Reader::Borrowed(BufPeekReader::new(reader)),
            Compression::Deflate(_) => Reader::Owned(BufPeekReader::new(Box::new(DeflateDecoder::new(reader)))),
            Compression::Gzip(_) => Reader::Owned(BufPeekReader::new(Box::new(GzDecoder::new(reader)))),
            Compression::Zlib(_) => Reader::Owned(BufPeekReader::new(Box::new(ZlibDecoder::new(reader)))),
        };

        Ok(Self(reader))
    }
}

impl<'de, 'a, 'b> serde::de::Deserializer<'de> for &'a mut Deserializer<'b> {
    type Error = Error;

    fn deserialize_any<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
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
            },
            data_ids::UNIT_VARIANT_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                let variant = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?;
                visitor.visit_enum(variant.into_deserializer())
            },
            data_ids::NEWTYPE_VARIANT_ID | data_ids::TUPLE_VARIANT_ID | data_ids::STRUCT_VARIANT_ID => {
                visitor.visit_enum(EnumAccess { de: self })
            },
            data_ids::TUPLE_STRUCT_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
                visitor.visit_seq(SeqAccess::new(self, length))
            },
            found => Err(Error::InvalidDataId { expected: format!("from {} to {}", data_ids::NULL_ID, data_ids::MAP_ID), found })
        }
    }

    fn deserialize_bool<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::BOOL_ID)?;
        visitor.visit_bool(self.0.read_u8().map_err(Error::IoError)? != 0)
    }

    fn deserialize_i8<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I8_ID)?;
        visitor.visit_i8(self.0.read_i8().map_err(Error::IoError)?)
    }

    fn deserialize_i16<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I16_ID)?;
        visitor.visit_i16(self.0.read_i16::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_i32<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I32_ID)?;
        visitor.visit_i32(self.0.read_i32::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_i64<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::I64_ID)?;
        visitor.visit_i64(self.0.read_i64::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_u8<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U8_ID)?;
        visitor.visit_u8(self.0.read_u8().map_err(Error::IoError)?)
    }

    fn deserialize_u16<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U16_ID)?;
        visitor.visit_u16(self.0.read_u16::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_u32<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U32_ID)?;
        visitor.visit_u32(self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_u64<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::U64_ID)?;
        visitor.visit_u64(self.0.read_u64::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_f32<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::F32_ID)?;
        visitor.visit_f32(self.0.read_f32::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_f64<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::F64_ID)?;
        visitor.visit_f64(self.0.read_f64::<ByteOrder>().map_err(Error::IoError)?)
    }

    fn deserialize_char<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::CHAR_ID)?;
        let length = self.0.read_u8().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        let string = String::from_utf8(buffer).map_err(Error::FromUtf8Error)?;
        visitor.visit_char(string.chars().next().unwrap())
    }

    fn deserialize_str<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::STR_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        let string = String::from_utf8(buffer).map_err(Error::FromUtf8Error)?;
        visitor.visit_str(&string)
    }

    fn deserialize_string<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::STR_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        visitor.visit_string(String::from_utf8(buffer).map_err(Error::FromUtf8Error)?)
    }

    fn deserialize_bytes<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::BYTES_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        visitor.visit_bytes(&buffer)
    }

    fn deserialize_byte_buf<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::BYTES_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        let mut buffer = vec![0_u8; length];
        self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
        visitor.visit_byte_buf(buffer)
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let peek_id = self.0.peek().read_u8().map_err(Error::IoError)?;
        match peek_id {
            data_ids::NULL_ID => {
                self.0.read_u8().map_err(Error::IoError)?;
                visitor.visit_none()
            },
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
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

    fn deserialize_seq<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::SEQ_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        visitor.visit_seq(SeqAccess::new(self, length))
    }

    fn deserialize_tuple<V: serde::de::Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        read_id(&mut self.0, data_ids::TUPLE_ID)?;
        let length = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        if length != len {
            return Err(Error::InvalidLength {
                expected: len,
                actual: length,
                message: String::from("Invalid tuple length")
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
                message: String::from("Invalid tuple struct length")
            });
        } else {
            visitor.visit_seq(SeqAccess::new(self, length))
        }
    }

    fn deserialize_map<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
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
            },
            data_ids::NEWTYPE_VARIANT_ID | data_ids::TUPLE_VARIANT_ID | data_ids::STRUCT_VARIANT_ID => {
                visitor.visit_enum(EnumAccess { de: self })
            },
            found => Err(Error::InvalidDataId { expected: format!("one of ({})'", [
                data_ids::UNIT_VARIANT_ID.to_string(),
                data_ids::NEWTYPE_VARIANT_ID.to_string(),
                data_ids::TUPLE_VARIANT_ID.to_string(),
                data_ids::STRUCT_VARIANT_ID.to_string(),
            ].join(", ")), found }),
        }
    }

    fn deserialize_identifier<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {        let data_id = self.0.read_u8().map_err(Error::IoError)?;
        let argument = self.0.read_u32::<ByteOrder>().map_err(Error::IoError)?;
        
        match data_id {
            data_ids::STR_ID => {
                let mut buffer = vec![0_u8; argument as usize];
                self.0.read_exact(&mut buffer).map_err(Error::IoError)?;
                let string = String::from_utf8(buffer).map_err(Error::FromUtf8Error)?;
                visitor.visit_str(&string)
            },
            data_ids::UNIT_VARIANT_ID | data_ids::TUPLE_VARIANT_ID | data_ids::STRUCT_VARIANT_ID | data_ids::NEWTYPE_VARIANT_ID => {
                visitor.visit_u32(argument)
            },
            v => Err(Error::InvalidDataId { expected: String::from("an identifier"), found: v }),
        }
    }

    fn deserialize_ignored_any<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {        self.deserialize_any(visitor)
    }
}

struct SeqAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
    len: usize,
    current: usize
}

impl<'a, 'b> SeqAccess<'a, 'b> {
    fn new(de: &'a mut Deserializer<'b>, len: usize) -> Self {
        Self {
            de,
            len,
            current: 0,
        }
    }
}

impl<'de, 'a, 'b> serde::de::SeqAccess<'de> for SeqAccess<'a, 'b> {
    type Error = Error;

    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
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

struct MapAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
    len: usize,
    current_key: usize,
    current_value: usize
}

impl<'a, 'b> MapAccess<'a, 'b> {
    fn new(de: &'a mut Deserializer<'b>, len: usize) -> Self {
        Self {
            de,
            len,
            current_key: 0,
            current_value: 0,
        }
    }
}

impl<'de, 'a, 'b> serde::de::MapAccess<'de> for MapAccess<'a, 'b> {
    type Error = Error;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {        if self.current_key < self.len {
            self.current_key += 1;
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {        if self.current_value < self.len {
            self.current_value += 1;
            seed.deserialize(&mut *self.de)
        } else {
            Err(Error::InvalidMapAccess)
        }
    }
}

struct EnumAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
}

impl<'de, 'a, 'b> serde::de::EnumAccess<'de> for EnumAccess<'a, 'b> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: serde::de::DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de, 'a, 'b> serde::de::VariantAccess<'de> for EnumAccess<'a, 'b> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Err(Error::UnexpectedString)
    }

    fn newtype_variant_seed<T: serde::de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Self::Error> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: serde::de::Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        let length = self.de.0.read_u32::<ByteOrder>().map_err(Error::IoError)? as usize;
        if length != len {
            return Err(Error::InvalidLength {
                expected: len,
                actual: length,
                message: String::from("Invalid tuple variant length")
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
        Err(Error::InvalidDataId { expected: expected.to_string(), found })
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use serde::{Serialize, de::DeserializeOwned, Deserialize};

    use crate::{se::to_bytes, Compression};

    fn deserialization_test_base<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T, compression: Compression) {
        let serialized = to_bytes(&value, compression).unwrap();
        let deserialized: T = crate::de::from_bytes(&serialized).unwrap();
        assert_eq!(value, &deserialized);
    }

    fn deserialization_test<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: T) {
        deserialization_test_base(&value, Compression::None);
        deserialization_test_base(&value, Compression::Deflate(6));
        deserialization_test_base(&value, Compression::Gzip(6));
        deserialization_test_base(&value, Compression::Zlib(6));
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
        deserialization_test('a');
        deserialization_test('-');
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
        deserialization_test(Struct { a: 1, b: 'a', c: "Hello World!".to_string() });
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
        deserialization_test(TestEnum::Struct { a: 1, b: 'a', c: "Hello World!".to_string() });
    }

    #[test]
    fn test_option_deserialization() {
        deserialization_test(None::<u8>);
        deserialization_test(Some(1_u8));
    }
}