use std::io::{Write, Read};

use byteorder::{WriteBytesExt, ReadBytesExt};
use err_derive::Error;

pub(crate) type ByteOrder = byteorder::BigEndian;

pub mod se;
pub mod de;

pub(crate) mod data_ids {
    pub const NULL_ID: u8 = 0;
    pub const BOOL_ID: u8 = 1;
    pub const I8_ID: u8 = 2;
    pub const I16_ID: u8 = 3;
    pub const I32_ID: u8 = 4;
    pub const I64_ID: u8 = 5;
    pub const U8_ID: u8 = 6;
    pub const U16_ID: u8 = 7;
    pub const U32_ID: u8 = 8;
    pub const U64_ID: u8 = 9;
    pub const F32_ID: u8 = 10;
    pub const F64_ID: u8 = 11;
    pub const CHAR_ID: u8 = 12;
    pub const STR_ID: u8 = 13;
    pub const BYTES_ID: u8 = 14;
    pub const SEQ_ID: u8 = 15;
    pub const TUPLE_ID: u8 = 16;
    pub const UNIT_VARIANT_ID: u8 = 17;
    pub const NEWTYPE_VARIANT_ID: u8 = 18;
    pub const TUPLE_VARIANT_ID: u8 = 19;
    pub const STRUCT_VARIANT_ID: u8 = 20;
    pub const TUPLE_STRUCT_ID: u8 = 21;
    pub const MAP_ID: u8 = 22;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "IO error: {}", _0)]
    IoError(#[source] std::io::Error),
    #[error(display = "From utf8 error: {}", _0)]
    FromUtf8Error(#[source] std::string::FromUtf8Error),
    #[error(display = "'{}' is not a valid compression format", _0)]
    InvalidCompression(u8),
    #[error(display = "{}", _0)]
    Custom(String),
    #[error(display = "Lengths are required for the sbif format")]
    LengthRequired,
    #[error(display = "Unexpected string")]
    UnexpectedString,
    #[error(display = "Invalid access order. You cannot access 2 map keys or 2 map values in a row")]
    InvalidMapAccess,
    #[error(display = "Invalid sbif header: expected 'SBIF', found {}", _0)]
    InvalidHeader(String),
    #[error(display = "Invalid data id: expected {}, found {}", expected, found)]
    InvalidDataId {
        expected: String,
        found: u8,
    },
    #[error(display = "Invalid sbif version: expected {}, found {}", expected, found)]
    InvalidVersion {
        expected: u8,
        found: u8,
    },
    #[error(display = "{}: expected {}, actual {}", message, expected, actual)]
    InvalidLength {
        expected: usize,
        actual: usize,
        message: String
    },
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self where T:std::fmt::Display {
        Self::Custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self where T:std::fmt::Display {
        Self::Custom(msg.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Deflate(u32),
    Gzip(u32),
    Zlib(u32)
}

impl Default for Compression {
    fn default() -> Self {
        Self::Gzip(6)
    }
}

pub(crate) struct FileHeader {
    pub(crate) compression: Compression,
    pub(crate) version: u8,
    pub(crate) header_name: String,
}

impl Default for FileHeader {
    fn default() -> Self {
        Self::new(Compression::default())
    }
}

impl FileHeader {
    pub fn new(compression: Compression) -> Self {
        Self {
            compression,
            version: 1,
            header_name: String::from("SBIF"),
        }
    }

    pub fn to_writer<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        let name_bytes = self.header_name.as_bytes();
        writer.write_u16::<ByteOrder>(name_bytes.len() as u16).map_err(Error::IoError)?;
        writer.write(name_bytes).map_err(Error::IoError)?;
        writer.write_u8(self.version).map_err(Error::IoError)?;

        match self.compression {
            Compression::None => writer.write_u8(0).map_err(Error::IoError)?,
            Compression::Deflate(v) => {
                writer.write_u8(1).map_err(Error::IoError)?;
                writer.write_u32::<ByteOrder>(v).map_err(Error::IoError)?;
            },
            Compression::Gzip(v) => {
                writer.write_u8(2).map_err(Error::IoError)?;
                writer.write_u32::<ByteOrder>(v).map_err(Error::IoError)?;
            },
            Compression::Zlib(v) => {
                writer.write_u8(3).map_err(Error::IoError)?;
                writer.write_u32::<ByteOrder>(v).map_err(Error::IoError)?;
            },
        };

        Ok(())
    }

    #[cfg(test)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = Vec::new();
        self.to_writer(&mut buffer)?;
        Ok(buffer)
    }

    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let header_name = {
            let name_length = reader.read_u16::<ByteOrder>().map_err(Error::IoError)? as usize;
            let mut buffer = vec![0_u8; name_length];
            reader.read_exact(&mut buffer).map_err(Error::IoError)?;
            String::from_utf8(buffer).map_err(Error::FromUtf8Error)?
        };

        let version = reader.read_u8().map_err(Error::IoError)?;
        let compression = match reader.read_u8().map_err(Error::IoError)? {
            0 => Compression::None,
            1 => Compression::Deflate(reader.read_u32::<ByteOrder>().map_err(Error::IoError)?),
            2 => Compression::Gzip(reader.read_u32::<ByteOrder>().map_err(Error::IoError)?),
            3 => Compression::Zlib(reader.read_u32::<ByteOrder>().map_err(Error::IoError)?),
            v => return Err(Error::InvalidCompression(v)),
        };

        Ok(Self { compression, version, header_name })
    }
}