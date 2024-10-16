////////////////////////////////////////////////////////////////////
//  NumberKind enumeration
////////////////////////////////////////////////////////////////////

use serde::{Deserialize, Serialize};

use crate::number_kind::NumberKind::*;

// Represents a numeric type or kind of value
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum NumberKind {
    F32Kind = 0,
    F64Kind = 1,
    I8Kind = 2,
    I16Kind = 3,
    I32Kind = 4,
    I64Kind = 5,
    I128Kind = 6,
    U8Kind = 7,
    U16Kind = 8,
    U32Kind = 9,
    U64Kind = 10,
    U128Kind = 11,
    NaNKind = 12,
}

pub const NUMBER_KINDS: [NumberKind; 13] = [
    F32Kind, F64Kind,
    I8Kind, I16Kind, I32Kind, I64Kind, I128Kind,
    U8Kind, U16Kind, U32Kind, U64Kind, U128Kind,
    NaNKind,
];

impl NumberKind {
    pub fn from_u8(index: u8) -> NumberKind {
        NUMBER_KINDS[index as usize]
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}