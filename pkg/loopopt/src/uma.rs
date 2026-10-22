//! Roster indices and tables whose rows follow `Data::umas`.
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

/// An uma's position in `Data::umas`, distinct from a candidate or legacy-pool position.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub(crate) struct UmaIndex(pub(crate) usize);

impl std::fmt::Display for UmaIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One entry per roster position. Nested tables use the same roster on each axis.
#[repr(transparent)]
#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub(crate) struct UmaTable<T>(pub(crate) Vec<T>);

impl<T> UmaTable<T> {
    pub(crate) const fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    pub(crate) fn indices(&self) -> impl DoubleEndedIterator<Item = UmaIndex> + use<T> {
        (0..self.len()).map(UmaIndex)
    }

    pub(crate) fn indexed(&self) -> impl DoubleEndedIterator<Item = (UmaIndex, &T)> {
        self.0.iter().enumerate().map(|(index, value)| (UmaIndex(index), value))
    }

    pub(crate) fn get(&self, index: UmaIndex) -> Option<&T> {
        self.0.get(index.0)
    }
}

impl<T: Clone> UmaTable<T> {
    pub(crate) fn resize(&mut self, len: usize, value: T) {
        self.0.resize(len, value);
    }
}

impl<T> Default for UmaTable<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> IntoIterator for UmaTable<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> Index<UmaIndex> for UmaTable<T> {
    type Output = T;

    fn index(&self, index: UmaIndex) -> &Self::Output {
        &self.0[index.0]
    }
}

impl<T> IndexMut<UmaIndex> for UmaTable<T> {
    fn index_mut(&mut self, index: UmaIndex) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

impl<T> FromIterator<T> for UmaTable<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'table, T> IntoIterator for &'table UmaTable<T> {
    type Item = &'table T;
    type IntoIter = std::slice::Iter<'table, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A square roster table in row-major storage, retaining the packed ceiling table's layout.
pub(crate) struct UmaPairTable<T> {
    /// Consecutive rows, each containing `roster_len` entries.
    pub(crate) values: Vec<T>,
    /// Number of umas on each axis.
    pub(crate) roster_len: usize,
}

impl<T> Index<(UmaIndex, UmaIndex)> for UmaPairTable<T> {
    type Output = T;

    fn index(&self, (row, column): (UmaIndex, UmaIndex)) -> &Self::Output {
        &self.values[row.0 * self.roster_len + column.0]
    }
}
