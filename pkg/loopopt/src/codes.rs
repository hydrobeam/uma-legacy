//! The game's integer codes as enums: each written as its integer (`serde_repr`), so data.json and the replies keep their
//! numbers, and each generated as a TypeScript `export enum` of the same integers (`#[ts(repr(enum))]`: ts-rs reads no
//! serde_repr attribute, and without it writes a union of the names for a value the wire carries as an integer). A file
//! with a code outside its enum does not parse. Beside them, what the codes group into: an aptitude's kind (`Kind`) and
//! a uma's grade at each aptitude (`Grades`).
use serde::{Deserialize, Serialize};
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
    /// Every grade, lowest first.
    pub(crate) const ALL: [Self; 8] = [Self::G, Self::F, Self::E, Self::D, Self::C, Self::B, Self::A, Self::S];

    /// The grade `steps` above it, at most S (a pink spark's firing raises one grade).
    pub(crate) const fn raise(self, steps: usize) -> Self {
        let position = self as usize - 1 + steps;
        if position < Self::ALL.len() { Self::ALL[position] } else { Self::S }
    }

    /// What the grade costs a race's aptitude basis (110 less its distance's and its surface's costs; MODEL.md, win
    /// chance in IT): S and A 0, B 10, C 20, D 30, E 40, F 60, G 90.
    pub(crate) const fn cost(self) -> i32 {
        match self {
            Self::S | Self::A => 0,
            Self::B => 10,
            Self::C => 20,
            Self::D => 30,
            Self::E => 40,
            Self::F => 60,
            Self::G => 90,
        }
    }
}

/// A race's distance class, data.json's `race_cat`, and the place of its grade in `Grades::distance`.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Distance {
    Short = 0,
    Mile = 1,
    Medium = 2,
    Long = 3,
}

/// A race's surface, data.json's `race_ground` (the exporter's numbering, not the game's 1 turf, 2 dirt), and the place
/// of its grade in `Grades::surface`.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Surface {
    Turf = 0,
    Dirt = 1,
}

/// A running style: the place of its grade in `Grades::style`.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Style {
    FrontRunner = 0,
    PaceChaser = 1,
    LateSurger = 2,
    EndCloser = 3,
}

/// One of a uma's ten aptitudes, as the game's aptitude factor group codes it (master.mdb `succession_factor`,
/// factor_type 2: a pink spark's factor_id is the group x 100 + its stars). Declared in code order, so the derived
/// order is the codes'.
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, ts_rs::TS)]
#[repr(u8)]
#[ts(repr(enum))]
pub(crate) enum Aptitude {
    Turf = 11,
    Dirt = 12,
    FrontRunner = 21,
    PaceChaser = 22,
    LateSurger = 23,
    EndCloser = 24,
    Sprint = 31,
    Mile = 32,
    Medium = 33,
    Long = 34,
}

/// What an aptitude is of: a distance, a surface or a running style (`Aptitude::kind`; the page reads it from
/// constants.ts `aptitude_kinds`).
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Kind {
    Distance { distance: Distance },
    Surface { surface: Surface },
    Style { style: Style },
}

impl Aptitude {
    /// Every aptitude in the solver's order (distances, surfaces, styles, each in its own order): the order of its
    /// per-aptitude tables and draws (`index`), and of a reply's `pink_gen`.
    pub(crate) const ORDER: [Self; 10] = [
        Self::Sprint,
        Self::Mile,
        Self::Medium,
        Self::Long,
        Self::Turf,
        Self::Dirt,
        Self::FrontRunner,
        Self::PaceChaser,
        Self::LateSurger,
        Self::EndCloser,
    ];

    /// What it is of.
    pub(crate) const fn kind(self) -> Kind {
        match self {
            Self::Sprint => Kind::Distance { distance: Distance::Short },
            Self::Mile => Kind::Distance { distance: Distance::Mile },
            Self::Medium => Kind::Distance { distance: Distance::Medium },
            Self::Long => Kind::Distance { distance: Distance::Long },
            Self::Turf => Kind::Surface { surface: Surface::Turf },
            Self::Dirt => Kind::Surface { surface: Surface::Dirt },
            Self::FrontRunner => Kind::Style { style: Style::FrontRunner },
            Self::PaceChaser => Kind::Style { style: Style::PaceChaser },
            Self::LateSurger => Kind::Style { style: Style::LateSurger },
            Self::EndCloser => Kind::Style { style: Style::EndCloser },
        }
    }

    /// Its place in `ORDER`, for a table with an entry per aptitude in that order.
    pub(crate) const fn index(self) -> usize {
        match self.kind() {
            Kind::Distance { distance } => distance as usize,
            Kind::Surface { surface } => 4 + surface as usize,
            Kind::Style { style } => 6 + style as usize,
        }
    }
}

impl From<Distance> for Aptitude {
    fn from(distance: Distance) -> Self {
        match distance {
            Distance::Short => Self::Sprint,
            Distance::Mile => Self::Mile,
            Distance::Medium => Self::Medium,
            Distance::Long => Self::Long,
        }
    }
}

impl From<Surface> for Aptitude {
    fn from(surface: Surface) -> Self {
        match surface {
            Surface::Turf => Self::Turf,
            Surface::Dirt => Self::Dirt,
        }
    }
}

/// A uma's grade at each aptitude (data.json's `apt` and `outfit_apt`, a reply's `Aptitudes`), each kind's grades at
/// their codes' places: `distance[Distance]`, `surface[Surface]`, `style[Style]`.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub(crate) struct Grades {
    pub(crate) distance: [Grade; 4],
    pub(crate) surface: [Grade; 2],
    pub(crate) style: [Grade; 4],
}

impl Grades {
    /// Her grade at `aptitude`.
    pub(crate) const fn get(&self, aptitude: Aptitude) -> Grade {
        match aptitude.kind() {
            Kind::Distance { distance } => self.distance[distance as usize],
            Kind::Surface { surface } => self.surface[surface as usize],
            Kind::Style { style } => self.style[style as usize],
        }
    }

    /// Set her grade at `aptitude`.
    pub(crate) const fn set(&mut self, aptitude: Aptitude, grade: Grade) {
        match aptitude.kind() {
            Kind::Distance { distance } => self.distance[distance as usize] = grade,
            Kind::Surface { surface } => self.surface[surface as usize] = grade,
            Kind::Style { style } => self.style[style as usize] = grade,
        }
    }

    /// Her grades in `Aptitude::ORDER` (a key's words: `CareerSetup::schedule_key`).
    pub(crate) fn in_order(&self) -> [Grade; 10] {
        Aptitude::ORDER.map(|aptitude| self.get(aptitude))
    }

    /// Whether the grades a race reads, its distance's and its surface's, are `other`'s.
    pub(crate) fn same_race_grades(&self, other: &Self) -> bool {
        self.distance == other.distance && self.surface == other.surface
    }

    /// The grades of codes 1..8 listed in `Aptitude::ORDER` (tests write them in that order).
    #[cfg(test)]
    pub(crate) fn of_codes(codes: [u8; 10]) -> Self {
        let mut grades = Self { distance: [Grade::G; 4], surface: [Grade::G; 2], style: [Grade::G; 4] };
        for (aptitude, code) in Aptitude::ORDER.into_iter().zip(codes) {
            grades.set(aptitude, Grade::ALL[usize::from(code) - 1]);
        }
        grades
    }
}

#[cfg(test)]
mod tests {
    use super::{Aptitude, Distance, Grade, Grades, Kind, Surface};

    /// `ORDER` and `index` agree, `kind` names each aptitude's distance, surface or style (as `From` does back), each
    /// code parses to its aptitude, and a code outside the enum does not parse.
    #[test]
    fn aptitude_order_kinds_and_codes() {
        for (index, aptitude) in Aptitude::ORDER.into_iter().enumerate() {
            assert_eq!(aptitude.index(), index, "{aptitude:?}");
            match aptitude.kind() {
                Kind::Distance { distance } => assert_eq!(Aptitude::from(distance), aptitude, "{distance:?}"),
                Kind::Surface { surface } => assert_eq!(Aptitude::from(surface), aptitude, "{surface:?}"),
                Kind::Style { .. } => assert!(index >= 6, "the styles are the last four"),
            }
            let code = serde_json::to_string(&aptitude).unwrap();
            assert_eq!(serde_json::from_str::<Aptitude>(&code).unwrap(), aptitude, "code {code}");
        }
        let codes: Vec<String> =
            Aptitude::ORDER.iter().map(|aptitude| serde_json::to_string(aptitude).unwrap()).collect();
        assert_eq!(codes, ["31", "32", "33", "34", "11", "12", "21", "22", "23", "24"], "the game's factor groups");
        for unknown in [0, 10, 13, 25, 35, 99] {
            assert!(serde_json::from_str::<Aptitude>(&unknown.to_string()).is_err(), "code {unknown}");
        }
        assert_eq!(Aptitude::from(Distance::Long), Aptitude::Long, "long");
        assert_eq!(Aptitude::from(Surface::Dirt), Aptitude::Dirt, "dirt");
        assert_eq!(
            serde_json::to_string(&Aptitude::Dirt.kind()).unwrap(),
            r#"{"kind":"surface","surface":1}"#,
            "a kind as the page reads it"
        );
    }

    /// Grades read each kind's list, a code reaches its grade, and a raise stops at S.
    #[test]
    fn grades_read_by_kind() {
        let grades: Grades =
            serde_json::from_str(r#"{"distance": [1, 2, 3, 4], "surface": [5, 6], "style": [7, 8, 1, 2]}"#).unwrap();
        assert_eq!(grades.get(Aptitude::Sprint), Grade::G, "short");
        assert_eq!(grades.get(Aptitude::Dirt), Grade::B, "dirt");
        assert_eq!(grades.get(Aptitude::EndCloser), Grade::F, "end closer");
        assert_eq!(Grades::of_codes([1, 2, 3, 4, 5, 6, 7, 8, 1, 2]), grades, "the same grades in `Aptitude::ORDER`");
        assert!(
            serde_json::from_str::<Grades>(r#"{"distance": [1, 2, 3], "surface": [5, 6], "style": [7, 8, 1, 2]}"#)
                .is_err()
        );
        assert_eq!(Grade::B.raise(1), Grade::A, "one step");
        assert_eq!(Grade::A.raise(4), Grade::S, "at most S");
        assert_eq!(Grade::G.raise(0), Grade::G, "no step");
    }
}
