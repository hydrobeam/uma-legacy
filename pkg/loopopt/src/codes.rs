//! The game's integer codes as enums: each written as its integer (`serde_repr`), so data.json and the replies keep their
//! numbers, and each generated as a TypeScript `export enum` of the same integers (`#[ts(repr(enum))]`: ts-rs reads no
//! serde_repr attribute, and without it writes a union of the names for a value the wire carries as an integer). A file
//! with a code outside its enum does not parse.
use serde_repr::{Deserialize_repr, Serialize_repr};

/// An aptitude grade, data.json's `apt` codes and the replies' (`Aptitudes`): G = 1 .. S = 8.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Grade {
    G = 1,
    F = 2,
    E = 3,
    D = 4,
    C = 5,
    B = 6,
    A = 7,
    S = 8,
}

impl Grade {
    /// The grade of `code`, 1..8: the solver's grades are codes inside it (data.json's are read as grades, and a pink
    /// spark raises one to S at most), so another code is a bug.
    pub(crate) const fn of(code: u8) -> Self {
        match code {
            1 => Self::G,
            2 => Self::F,
            3 => Self::E,
            4 => Self::D,
            5 => Self::C,
            6 => Self::B,
            7 => Self::A,
            8 => Self::S,
            _ => panic!("an aptitude grade is a code 1..8"),
        }
    }
}

/// A race's distance class, data.json's `race_cat`: its aptitude is the grade at that index of a list of grades.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Distance {
    Short = 0,
    Mile = 1,
    Medium = 2,
    Long = 3,
}

impl Distance {
    /// Its aptitude's index in a list of grades (data.json's `apt` order: short, mile, medium, long, turf, dirt, ...).
    pub(crate) const fn aptitude(self) -> usize {
        self as usize
    }
}

/// A race's surface, data.json's `race_ground`.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Surface {
    Turf = 1,
    Dirt = 2,
}

impl Surface {
    /// Its aptitude's index in a list of grades (turf 4, dirt 5).
    pub(crate) const fn aptitude(self) -> usize {
        match self {
            Self::Turf => 4,
            Self::Dirt => 5,
        }
    }
}
