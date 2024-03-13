////////////////////////////////////////////////////////////////////
// typed values module
////////////////////////////////////////////////////////////////////

use std::{i32, io};
use std::error::Error;
use std::ops::{Add, Div, Mul, Sub};

use chrono::{DateTime, NaiveDateTime};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec;
use crate::data_types::DataType;
use crate::data_types::DataType::*;
use crate::typed_values::TypedValue::*;

const ISO_DATE_FORMAT: &str = r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?(([+-]\d\d:\d\d)|Z)?$";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TypedValue {
    BLOBValue(Vec<u8>),
    BooleanValue(bool),
    CLOBValue(Vec<char>),
    DateValue(i64),
    Float32Value(f32),
    Float64Value(f64),
    Int8Value(u8),
    Int16Value(i16),
    Int32Value(i32),
    Int64Value(i64),
    Null,
    RecordNumber(usize),
    StringValue(String),
    Undefined,
    UUIDValue([u8; 16]),
}

impl TypedValue {
    /// decodes the typed value based on the supplied data type and buffer
    pub fn decode(data_type: &DataType, buffer: &Vec<u8>, offset: usize) -> TypedValue {
        match data_type {
            BLOBType(_size) => BLOBValue(vec![]),
            BooleanType => codec::decode_u8(buffer, offset, |b| BooleanValue(b == 1)),
            CLOBType(_size) => CLOBValue(vec![]),
            DateType => codec::decode_u8x8(buffer, offset, |b| DateValue(i64::from_be_bytes(b))),
            EnumType(_) => todo!(),
            Int8Type => codec::decode_u8(buffer, offset, |b| Int8Value(b)),
            Int16Type => codec::decode_u8x2(buffer, offset, |b| Int16Value(i16::from_be_bytes(b))),
            Int32Type => codec::decode_u8x4(buffer, offset, |b| Int32Value(i32::from_be_bytes(b))),
            Int64Type => codec::decode_u8x8(buffer, offset, |b| Int64Value(i64::from_be_bytes(b))),
            Float32Type => codec::decode_u8x4(buffer, offset, |b| Float32Value(f32::from_be_bytes(b))),
            Float64Type => codec::decode_u8x8(buffer, offset, |b| Float64Value(f64::from_be_bytes(b))),
            RecordNumberType => Int64Value(codec::decode_row_id(&buffer, 1) as i64),
            StringType(size) => StringValue(codec::decode_string(buffer, offset, *size).to_string()),
            StructType(_) => todo!(),
            TableType(_) => todo!(),
            UUIDType => codec::decode_u8x16(buffer, offset, |b| UUIDValue(b))
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        Self::encode_value(self)
    }

    pub fn encode_value(value: &TypedValue) -> Vec<u8> {
        match value {
            BLOBValue(bytes) => codec::encode_u8x_n(bytes.to_vec()),
            BooleanValue(v) => [if *v { 1 } else { 0 }].to_vec(),
            CLOBValue(chars) => codec::encode_chars(chars.to_vec()),
            DateValue(number) => number.to_be_bytes().to_vec(),
            Float32Value(number) => number.to_be_bytes().to_vec(),
            Float64Value(number) => number.to_be_bytes().to_vec(),
            Int8Value(number) => number.to_be_bytes().to_vec(),
            Int16Value(number) => number.to_be_bytes().to_vec(),
            Int32Value(number) => number.to_be_bytes().to_vec(),
            Int64Value(number) => number.to_be_bytes().to_vec(),
            Null | Undefined => [0u8; 0].to_vec(),
            RecordNumber(id) => id.to_be_bytes().to_vec(),
            StringValue(string) => codec::encode_string(string),
            UUIDValue(guid) => guid.to_vec(),
        }
    }

    fn intercept_unknowns(a: &TypedValue, b: &TypedValue) -> Option<TypedValue> {
        match (a, b) {
            (Undefined, _) => Some(Undefined),
            (_, Undefined) => Some(Undefined),
            (Null, _) => Some(Null),
            (_, Null) => Some(Null),
            _ => None
        }
    }

    fn millis_to_iso_date(millis: i64) -> String {
        let seconds = millis / 1000;
        let nanoseconds = (millis % 1000) * 1_000_000;
        let datetime = NaiveDateTime::from_timestamp_opt(seconds, nanoseconds as u32).unwrap();
        let iso_date = datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        iso_date
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            BLOBValue(bytes) => serde_json::json!(bytes),
            BooleanValue(b) => serde_json::json!(b),
            CLOBValue(chars) => serde_json::json!(chars),
            DateValue(millis) => serde_json::json!(Self::millis_to_iso_date(*millis)),
            Float32Value(number) => serde_json::json!(number),
            Float64Value(number) => serde_json::json!(number),
            Int8Value(number) => serde_json::json!(number),
            Int16Value(number) => serde_json::json!(number),
            Int32Value(number) => serde_json::json!(number),
            Int64Value(number) => serde_json::json!(number),
            Null => serde_json::Value::Null,
            RecordNumber(number) => serde_json::json!(number),
            StringValue(string) => serde_json::json!(string),
            Undefined => serde_json::Value::Null,
            UUIDValue(guid) => serde_json::json!(Uuid::from_bytes(*guid).to_string()),
        }
    }

    pub fn unwrap_value(&self) -> String {
        match self {
            BLOBValue(bytes) => hex::encode(bytes),
            BooleanValue(b) => (if *b { "true" } else { "false" }).into(),
            CLOBValue(chars) => chars.into_iter().collect(),
            DateValue(millis) => Self::millis_to_iso_date(*millis),
            Float32Value(number) => number.to_string(),
            Float64Value(number) => number.to_string(),
            Int8Value(number) => number.to_string(),
            Int16Value(number) => number.to_string(),
            Int32Value(number) => number.to_string(),
            Int64Value(number) => number.to_string(),
            Null => "null".into(),
            RecordNumber(number) => number.to_string(),
            StringValue(string) => string.into(),
            Undefined => "undef".into(),
            UUIDValue(guid) => Uuid::from_bytes(*guid).to_string(),
        }
    }

    pub fn wrap_value(raw_value: impl Into<String>) -> io::Result<Self> {
        let decimal_regex = Regex::new(r"^-?\d+\.\d+$")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let int_regex = Regex::new(r"^-?\d+$")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let iso_date_regex = Regex::new(ISO_DATE_FORMAT)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let uuid_regex = Regex::new("^[0-9a-fA-F]{8}\\b-[0-9a-fA-F]{4}\\b-[0-9a-fA-F]{4}\\b-[0-9a-fA-F]{4}\\b-[0-9a-fA-F]{12}$")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let result = match raw_value.into().as_str() {
            s if s == "false" => BooleanValue(false),
            s if s == "null" => Null,
            s if s == "true" => BooleanValue(true),
            s if s.is_empty() => Null,
            s if int_regex.is_match(s) => Int64Value(s.parse::<i64>()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?),
            s if decimal_regex.is_match(s) => Float64Value(s.parse::<f64>()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?),
            s if iso_date_regex.is_match(s) =>
                DateValue(DateTime::parse_from_rfc3339(s)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?.timestamp_millis()),
            s if uuid_regex.is_match(s) => UUIDValue(codec::decode_uuid(s)?),
            s => StringValue(s.to_string()),
        };
        Ok(result)
    }

    pub fn wrap_value_opt(opt_value: &Option<String>) -> io::Result<Self> {
        match opt_value {
            Some(value) => Self::wrap_value(value),
            None => Ok(Null)
        }
    }

    ///////////////////////////////////////////////////////////////
    //      CONDITIONAL OPERATIONS
    ///////////////////////////////////////////////////////////////

    pub fn and(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_bool()?, rhs.assume_bool()?);
        Some(BooleanValue(a && b))
    }

    pub fn between(&self, lhs: &TypedValue, rhs: &TypedValue) -> Option<TypedValue> {
        let (c, b, a) = (self.assume_numeric()?, lhs.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue((c >= a) && (c <= b)))
    }

    pub fn eq(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue(a == b))
    }

    pub fn gt(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue(a > b))
    }

    pub fn gte(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue(a >= b))
    }

    pub fn lt(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue(a < b))
    }

    pub fn lte(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue(a <= b))
    }

    pub fn ne(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let (a, b) = (self.assume_numeric()?, rhs.assume_numeric()?);
        Some(BooleanValue(a != b))
    }

    pub fn or(&self, rhs: &TypedValue) -> Option<TypedValue> {
        let a = self.assume_bool()?;
        let b = rhs.assume_bool()?;
        Some(BooleanValue(a || b))
    }

    ///////////////////////////////////////////////////////////////
    //      UTILITY METHODS
    ///////////////////////////////////////////////////////////////

    fn assume_bool(&self) -> Option<bool> {
        match &self {
            BooleanValue(b) => Some(*b),
            _ => None
        }
    }

    fn assume_numeric(&self) -> Option<f64> {
        match &self {
            Float32Value(n) => Some(*n as f64),
            Float64Value(n) => Some(*n),
            Int8Value(n) => Some(*n as f64),
            Int16Value(n) => Some(*n as f64),
            Int32Value(n) => Some(*n as f64),
            Int64Value(n) => Some(*n as f64),
            _ => None
        }
    }
}

impl Add for TypedValue {
    type Output = TypedValue;

    fn add(self, rhs: Self) -> Self::Output {
        TypedValue::intercept_unknowns(&self, &rhs).unwrap_or(
            match (&self, &rhs) {
                (Float64Value(a), Float64Value(b)) => Float64Value(a + b),
                (Float32Value(a), Float32Value(b)) => Float32Value(a + b),
                (Int64Value(a), Int64Value(b)) => Int64Value(a + b),
                (Int32Value(a), Int32Value(b)) => Int32Value(a + b),
                (Int16Value(a), Int16Value(b)) => Int16Value(a + b),
                (Int8Value(a), Int8Value(b)) => Int8Value(a + b),
                (StringValue(a), StringValue(b)) => StringValue(a.to_string() + b),
                _ => Undefined
            })
    }
}

impl Div for TypedValue {
    type Output = TypedValue;

    fn div(self, rhs: Self) -> Self::Output {
        TypedValue::intercept_unknowns(&self, &rhs).unwrap_or(
            match (&self, &rhs) {
                (Float64Value(a), Float64Value(b)) => Float64Value(a / b),
                (Float32Value(a), Float32Value(b)) => Float32Value(a / b),
                (Int64Value(a), Int64Value(b)) => Int64Value(a / b),
                (Int32Value(a), Int32Value(b)) => Int32Value(a / b),
                (Int16Value(a), Int16Value(b)) => Int16Value(a / b),
                (Int8Value(a), Int8Value(b)) => Int8Value(a / b),
                _ => Undefined
            })
    }
}

impl Mul for TypedValue {
    type Output = TypedValue;

    fn mul(self, rhs: Self) -> Self::Output {
        TypedValue::intercept_unknowns(&self, &rhs).unwrap_or(
            match (&self, &rhs) {
                (Float64Value(a), Float64Value(b)) => Float64Value(a * b),
                (Float32Value(a), Float32Value(b)) => Float32Value(a * b),
                (Int64Value(a), Int64Value(b)) => Int64Value(a * b),
                (Int32Value(a), Int32Value(b)) => Int32Value(a * b),
                (Int16Value(a), Int16Value(b)) => Int16Value(a * b),
                (Int8Value(a), Int8Value(b)) => Int8Value(a * b),
                _ => Undefined
            })
    }
}

impl Sub for TypedValue {
    type Output = TypedValue;

    fn sub(self, rhs: Self) -> Self::Output {
        TypedValue::intercept_unknowns(&self, &rhs).unwrap_or(
            match (&self, &rhs) {
                (Float64Value(a), Float64Value(b)) => Float64Value(a - b),
                (Float32Value(a), Float32Value(b)) => Float32Value(a - b),
                (Int64Value(a), Int64Value(b)) => Int64Value(a - b),
                (Int32Value(a), Int32Value(b)) => Int32Value(a - b),
                (Int16Value(a), Int16Value(b)) => Int16Value(a - b),
                (Int8Value(a), Int8Value(b)) => Int8Value(a - b),
                _ => Undefined
            })
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(Float64Value(45.0) + Float64Value(32.7), Float64Value(77.7));
        assert_eq!(Float32Value(45.7) + Float32Value(32.0), Float32Value(77.7));
        assert_eq!(Int64Value(45) + Int64Value(32), Int64Value(77));
        assert_eq!(Int32Value(45) + Int32Value(32), Int32Value(77));
        assert_eq!(Int16Value(45) + Int16Value(32), Int16Value(77));
        assert_eq!(Int8Value(45) + Int8Value(32), Int8Value(77));
        assert_eq!(StringValue("Hello".into()) + StringValue(" World".into()), StringValue("Hello World".into()));
    }

    #[test]
    fn test_eq() {
        assert_eq!(Int8Value(0xCE), Int8Value(0xCE));
        assert_eq!(Int16Value(0x7ACE), Int16Value(0x7ACE));
        assert_eq!(Int32Value(0x1111_BEEF), Int32Value(0x1111_BEEF));
        assert_eq!(Int64Value(0x5555_FACE_CAFE_BABE), Int64Value(0x5555_FACE_CAFE_BABE));
        assert_eq!(Float32Value(45.0), Float32Value(45.0));
        assert_eq!(Float64Value(45.0), Float64Value(45.0));
    }

    #[test]
    fn test_sub() {
        assert_eq!(Float64Value(45.0) - Float64Value(32.0), Float64Value(13.0));
        assert_eq!(Float32Value(45.0) - Float32Value(32.0), Float32Value(13.0));
        assert_eq!(Int64Value(45) - Int64Value(32), Int64Value(13));
        assert_eq!(Int32Value(45) - Int32Value(32), Int32Value(13));
        assert_eq!(Int16Value(45) - Int16Value(32), Int16Value(13));
        assert_eq!(Int8Value(45) - Int8Value(32), Int8Value(13));
    }

    #[test]
    fn test_decode() {
        let buf: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 5, b'H', b'e', b'l', b'l', b'o'];
        assert_eq!(TypedValue::decode(&StringType(5), &buf, 0), StringValue("Hello".into()))
    }

    #[test]
    fn test_encode() {
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 5, b'H', b'e', b'l', b'l', b'o'];
        assert_eq!(TypedValue::encode_value(&StringValue("Hello".into())), expected);
    }

    #[test]
    fn test_to_json() {
        verify_to_json(BooleanValue(false), serde_json::json!(false));
        verify_to_json(BooleanValue(true), serde_json::json!(true));
        verify_to_json(DateValue(1709163679081), serde_json::Value::String("2024-02-28T23:41:19.081Z".into()));
        verify_to_json(Float64Value(100.1), serde_json::json!(100.1));
        verify_to_json(Int64Value(100), serde_json::json!(100));
        verify_to_json(Null, serde_json::Value::Null);
        verify_to_json(StringValue("Hello World".into()), serde_json::Value::String("Hello World".into()));
        verify_to_json(UUIDValue([
            0x67, 0x45, 0x23, 0x01, 0xAB, 0xCD, 0xEF, 0x89,
            0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF,
        ]), serde_json::Value::String("67452301-abcd-ef89-1234-567890abcdef".into()));
    }

    #[test]
    fn test_wrap_and_unwrap_value() {
        verify_wrap_unwrap("false", BooleanValue(false));
        verify_wrap_unwrap("true", BooleanValue(true));
        verify_wrap_unwrap("2024-02-28T23:41:19.081Z", DateValue(1709163679081));
        verify_wrap_unwrap("100.1", Float64Value(100.1));
        verify_wrap_unwrap("100", Int64Value(100));
        verify_wrap_unwrap("null", Null);
        verify_wrap_unwrap("Hello World", StringValue("Hello World".into()));
        verify_wrap_unwrap("67452301-abcd-ef89-1234-567890abcdef", UUIDValue([
            0x67, 0x45, 0x23, 0x01, 0xAB, 0xCD, 0xEF, 0x89,
            0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF,
        ]))
    }

    #[test]
    fn test_unwrap_optional_value() {
        assert_eq!(TypedValue::wrap_value_opt(&Some("123.45".into())).unwrap(), Float64Value(123.45))
    }

    fn verify_to_json(typed_value: TypedValue, expected: serde_json::Value) {
        assert_eq!(typed_value.to_json(), expected)
    }

    fn verify_wrap_unwrap(string_value: &str, typed_value: TypedValue) {
        assert_eq!(typed_value.unwrap_value(), string_value);
        match TypedValue::wrap_value(string_value) {
            Ok(value) => assert_eq!(value, typed_value),
            Err(message) => panic!("{}", message)
        };
    }
}