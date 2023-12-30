use std::io::Write;

use byteorder::WriteBytesExt;
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use serde::Serialize;

use crate::{ByteOrder, Compression, Error, FileHeader};

/// Serializes a value into a byte vector.
pub fn to_bytes<T: serde::Serialize>(
    value: &T,
    compression: Compression,
) -> Result<Vec<u8>, Error> {
    let mut buffer = Vec::new();
    let mut serializer = Serializer::new(&mut buffer, compression)?;
    value.serialize(&mut serializer)?;
    drop(serializer);

    Ok(buffer)
}

/// Serializes a value into a writer.
pub fn to_writer<W: Write, T: serde::Serialize>(
    writer: W,
    value: &T,
    compression: Compression,
) -> Result<(), Error> {
    let mut serializer = Serializer::new(writer, compression)?;
    value.serialize(&mut serializer)?;
    drop(serializer);

    Ok(())
}

enum Writer<W: Write> {
    None(W),
    Deflate(DeflateEncoder<W>),
    GZip(GzEncoder<W>),
    ZLib(ZlibEncoder<W>),
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::None(w) => w.write(buf),
            Self::Deflate(w) => w.write(buf),
            Self::GZip(w) => w.write(buf),
            Self::ZLib(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::None(w) => w.flush(),
            Self::Deflate(w) => w.flush(),
            Self::GZip(w) => w.flush(),
            Self::ZLib(w) => w.flush(),
        }
    }
}

/// Serializer for SBIF format.
pub struct Serializer<W: Write>(Writer<W>);

impl<W: Write> Serializer<W> {
    /// Creates a new serializer from a writer. The serializer will automatically write the header to the writer based on the compression type.
    /// 
    /// Example:
    /// ```
    /// use serde_sbif::Serializer;
    /// fn serialize_to_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    ///     let mut buffer = Vec::new();
    ///     let mut serializer = Serializer::new(&mut buffer, Compression::default()).unwrap();
    ///     value.serialize(&mut serializer).unwrap();
    /// 
    ///     buffer
    /// }
    /// ```
    pub fn new(mut writer: W, compression: Compression) -> Result<Self, Error> {
        FileHeader::new(compression).to_writer(&mut writer)?;
        let writer: Writer<W> = match compression {
            Compression::None => Writer::None(writer),
            Compression::Deflate(v) => {
                Writer::Deflate(DeflateEncoder::new(writer, flate2::Compression::new(v)))
            }
            Compression::GZip(v) => {
                Writer::GZip(GzEncoder::new(writer, flate2::Compression::new(v)))
            }
            Compression::ZLib(v) => {
                Writer::ZLib(ZlibEncoder::new(writer, flate2::Compression::new(v)))
            }
        };

        Ok(Self(writer))
    }
}

impl<'a, W: Write> serde::ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::BOOL_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u8(if v { 1 } else { 0 })
            .map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::I8_ID)
            .map_err(Error::IoError)?;
        self.0.write_i8(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::I16_ID)
            .map_err(Error::IoError)?;
        self.0.write_i16::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::I32_ID)
            .map_err(Error::IoError)?;
        self.0.write_i32::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::I64_ID)
            .map_err(Error::IoError)?;
        self.0.write_i64::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::U8_ID)
            .map_err(Error::IoError)?;
        self.0.write_u8(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::U16_ID)
            .map_err(Error::IoError)?;
        self.0.write_u16::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::U32_ID)
            .map_err(Error::IoError)?;
        self.0.write_u32::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::U64_ID)
            .map_err(Error::IoError)?;
        self.0.write_u64::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::F32_ID)
            .map_err(Error::IoError)?;
        self.0.write_f32::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::F64_ID)
            .map_err(Error::IoError)?;
        self.0.write_f64::<ByteOrder>(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let utf8_bytes = v.to_string().into_bytes();
        self.0
            .write_u8(crate::data_ids::CHAR_ID)
            .map_err(Error::IoError)?;
        self.0.write(&utf8_bytes).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let bytes = v.as_bytes();
        self.0
            .write_u8(crate::data_ids::STR_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(bytes.len() as u32)
            .map_err(Error::IoError)?;
        self.0.write(bytes).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::BYTES_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(v.len() as u32)
            .map_err(Error::IoError)?;
        self.0.write(v).map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::NULL_ID)
            .map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_some<T: ?Sized + serde::Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::UNIT_VARIANT_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(variant_index)
            .map_err(Error::IoError)?;
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.0
            .write_u8(crate::data_ids::ENUM_VARIANT_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(variant_index)
            .map_err(Error::IoError)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let len = len.ok_or(Error::LengthRequired)?;
        self.0
            .write_u8(crate::data_ids::SEQ_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.0
            .write_u8(crate::data_ids::TUPLE_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.0
            .write_u8(crate::data_ids::TUPLE_STRUCT_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.0
            .write_u8(crate::data_ids::ENUM_VARIANT_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(variant_index)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let len = len.ok_or(Error::LengthRequired)?;
        self.0
            .write_u8(crate::data_ids::MAP_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.0
            .write_u8(crate::data_ids::MAP_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.0
            .write_u8(crate::data_ids::ENUM_VARIANT_ID)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(variant_index)
            .map_err(Error::IoError)?;
        self.0
            .write_u32::<ByteOrder>(len as u32)
            .map_err(Error::IoError)?;
        Ok(self)
    }
}

impl<'a, W: Write> serde::ser::SerializeSeq for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeTuple for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeTupleStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeTupleVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeMap for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + serde::Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(&mut **self)?;
        Ok(())
    }

    fn serialize_value<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: Write> serde::ser::SerializeStructVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Serialize;

    use crate::data_ids;

    use super::*;

    fn no_compression_serialization_test<T: serde::Serialize>(value: &T) -> Vec<u8> {
        let compression = Compression::None;
        let default_hdr_bytes = FileHeader::new(compression).to_bytes().unwrap();
        let serialized = to_bytes(value, compression).unwrap();
        assert!(serialized.len() >= default_hdr_bytes.len());
        assert_eq!(&serialized[0..8], default_hdr_bytes.as_slice());

        (&serialized[8..]).to_vec()
    }

    #[test]
    fn test_bool_serialization() {
        let test = no_compression_serialization_test(&true);
        assert_eq!(test.as_slice(), &[data_ids::BOOL_ID, 1]);
        let test = no_compression_serialization_test(&false);
        assert_eq!(test.as_slice(), &[data_ids::BOOL_ID, 0]);
    }

    #[test]
    fn test_integer_serialization() {
        let test = no_compression_serialization_test(&1_u8);
        assert_eq!(test.as_slice(), &[data_ids::U8_ID, 1]);
        let test = no_compression_serialization_test(&1_u16);
        assert_eq!(test.as_slice(), &[data_ids::U16_ID, 0, 1]);
        let test = no_compression_serialization_test(&1_u32);
        assert_eq!(test.as_slice(), &[data_ids::U32_ID, 0, 0, 0, 1]);
        let test = no_compression_serialization_test(&1_u64);
        assert_eq!(test.as_slice(), &[data_ids::U64_ID, 0, 0, 0, 0, 0, 0, 0, 1]);

        let test = no_compression_serialization_test(&1_i8);
        assert_eq!(test.as_slice(), &[data_ids::I8_ID, 1]);
        let test = no_compression_serialization_test(&1_i16);
        assert_eq!(test.as_slice(), &[data_ids::I16_ID, 0, 1]);
        let test = no_compression_serialization_test(&1_i32);
        assert_eq!(test.as_slice(), &[data_ids::I32_ID, 0, 0, 0, 1]);
        let test = no_compression_serialization_test(&1_i64);
        assert_eq!(test.as_slice(), &[data_ids::I64_ID, 0, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn test_float_serialization() {
        let test = no_compression_serialization_test(&1_f32);
        assert_eq!(test.as_slice(), &[data_ids::F32_ID, 63, 128, 0, 0]);
        let test = no_compression_serialization_test(&1_f64);
        assert_eq!(
            test.as_slice(),
            &[data_ids::F64_ID, 63, 240, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_char_serialization() {
        let test = no_compression_serialization_test(&'a');
        assert_eq!(test.as_slice(), &[data_ids::CHAR_ID, 97]);
    }

    #[test]
    fn test_string_serialization() {
        let test = no_compression_serialization_test(&"hello world");
        assert_eq!(
            test.as_slice(),
            &[
                data_ids::STR_ID,
                0,
                0,
                0,
                11,
                104,
                101,
                108,
                108,
                111,
                32,
                119,
                111,
                114,
                108,
                100
            ]
        );
    }

    #[test]
    fn test_null_serialization() {
        #[derive(Serialize)]
        struct UnitStruct;

        let test = no_compression_serialization_test(&Option::<u32>::None);
        assert_eq!(test.as_slice(), &[data_ids::NULL_ID]);
        let test = no_compression_serialization_test(&());
        assert_eq!(test.as_slice(), &[data_ids::NULL_ID]);
        let test = no_compression_serialization_test(&UnitStruct);
        assert_eq!(test.as_slice(), &[data_ids::NULL_ID]);
    }

    #[test]
    fn test_enum_serialization() {
        #[derive(Serialize)]
        enum TestEnum {
            Unit,
            NewType(u8),
            Tuple(u8, u8),
            Struct { a: u8, b: u8 },
        }

        let test = no_compression_serialization_test(&TestEnum::Unit);
        assert_eq!(test.as_slice(), &[data_ids::UNIT_VARIANT_ID, 0, 0, 0, 0]);
        let test = no_compression_serialization_test(&TestEnum::NewType(1));
        assert_eq!(
            test.as_slice(),
            &[data_ids::ENUM_VARIANT_ID, 0, 0, 0, 1, data_ids::U8_ID, 1]
        );
        let test = no_compression_serialization_test(&TestEnum::Tuple(1, 2));
        assert_eq!(
            test.as_slice(),
            &[
                data_ids::ENUM_VARIANT_ID,
                0,
                0,
                0,
                2,
                0,
                0,
                0,
                2,
                data_ids::U8_ID,
                1,
                data_ids::U8_ID,
                2
            ]
        );
        let test = no_compression_serialization_test(&TestEnum::Struct { a: 1, b: 2 });
        assert_eq!(
            test.as_slice(),
            &[
                data_ids::ENUM_VARIANT_ID,
                0,
                0,
                0,
                3, // variant index
                0,
                0,
                0,
                2, // length
                data_ids::STR_ID,
                0,
                0,
                0,
                1,
                97,
                data_ids::U8_ID,
                1, // a
                data_ids::STR_ID,
                0,
                0,
                0,
                1,
                98,
                data_ids::U8_ID,
                2 // b
            ]
        );
    }

    #[test]
    fn test_newtype_struct_serialization() {
        #[derive(Serialize)]
        struct NewtypeTest(u8);

        let test = no_compression_serialization_test(&NewtypeTest(1));
        assert_eq!(test.as_slice(), &[data_ids::U8_ID, 1]);
    }

    #[test]
    fn test_tuple_struct_serialization() {
        #[derive(Serialize)]
        struct TupleTest(u8, u8);

        let test = no_compression_serialization_test(&TupleTest(1, 2));
        assert_eq!(
            test.as_slice(),
            &[
                data_ids::TUPLE_STRUCT_ID,
                0,
                0,
                0,
                2,
                data_ids::U8_ID,
                1,
                data_ids::U8_ID,
                2
            ]
        );
    }

    #[test]
    fn test_struct_serialization() {
        #[derive(Serialize)]
        struct StructTest {
            a: u8,
            b: u8,
        }

        let test = no_compression_serialization_test(&StructTest { a: 1, b: 2 });
        assert_eq!(
            test.as_slice(),
            &[
                data_ids::MAP_ID,
                0,
                0,
                0,
                2, // length
                data_ids::STR_ID,
                0,
                0,
                0,
                1,
                97,
                data_ids::U8_ID,
                1, // a
                data_ids::STR_ID,
                0,
                0,
                0,
                1,
                98,
                data_ids::U8_ID,
                2 // b
            ]
        );
    }

    #[test]
    fn test_map_serialization() {
        let mut map = HashMap::<u8, u8>::new();
        map.insert(1, 2);
        map.insert(3, 4);

        let test = no_compression_serialization_test(&map);
        assert_eq!(&test[..5], &[data_ids::MAP_ID, 0, 0, 0, 2]);

        let mut slices = Vec::new();
        for i in 0..(test.len() - 5) / 4 {
            slices.push(&test[5 + i * 4..5 + (i + 1) * 4]);
        }

        slices.sort_by(|a, b| a[1].cmp(&b[1]));
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0], &[data_ids::U8_ID, 1, data_ids::U8_ID, 2]);
        assert_eq!(slices[1], &[data_ids::U8_ID, 3, data_ids::U8_ID, 4]);
    }

    #[test]
    fn test_option_serialization() {
        let test = no_compression_serialization_test(&Option::<u8>::None);
        assert_eq!(test.as_slice(), &[data_ids::NULL_ID]);
        let test = no_compression_serialization_test(&Option::<u8>::Some(1));
        assert_eq!(test.as_slice(), &[data_ids::U8_ID, 1]);
    }
}
