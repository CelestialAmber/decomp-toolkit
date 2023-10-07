use std::{cmp::max, collections::BTreeMap, ops::RangeBounds};

use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::{
    obj::{ObjInfo, ObjSection},
    util::{nested::NestedVec, split::default_section_align},
};

/// Marks a split point within a section.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObjSplit {
    pub unit: String,
    pub end: u32,
    pub align: Option<u32>,
    /// Whether this is a part of common BSS.
    pub common: bool,
    /// Generated, replaceable by user.
    pub autogenerated: bool,
    /// Skip when emitting the split object.
    pub skip: bool,
    /// Override the section name in the split object. (e.g. `.ctors$10`)
    pub rename: Option<String>,
}

impl ObjSplit {
    pub fn alignment(
        &self,
        obj: &ObjInfo,
        section_index: usize,
        section: &ObjSection,
        split_addr: u32,
    ) -> u32 {
        self.align.unwrap_or_else(|| {
            let default_align = default_section_align(section) as u32;
            max(
                // Maximum alignment of any symbol in this split
                obj.symbols
                    .for_section_range(section_index, split_addr..self.end)
                    .filter(|&(_, s)| s.size_known && s.size > 0)
                    .filter_map(|(_, s)| s.align)
                    .max()
                    .unwrap_or(default_align),
                default_align,
            )
        })
    }
}

/// Splits within a section.
#[derive(Debug, Clone, Default)]
pub struct ObjSplits {
    splits: BTreeMap<u32, Vec<ObjSplit>>,
}

impl ObjSplits {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (u32, &ObjSplit)> {
        self.splits.iter().flat_map(|(addr, v)| v.iter().map(move |u| (*addr, u)))
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (u32, &mut ObjSplit)> {
        self.splits.iter_mut().flat_map(|(addr, v)| v.iter_mut().map(move |u| (*addr, u)))
    }

    pub fn has_split_at(&self, address: u32) -> bool { self.splits.contains_key(&address) }

    /// Locate an existing split for the given address.
    pub fn for_address(&self, address: u32) -> Option<(u32, &ObjSplit)> {
        match self.for_range(..=address).next_back() {
            Some((addr, split)) if split.end == 0 || split.end > address => Some((addr, split)),
            _ => None,
        }
    }

    pub fn at_mut(&mut self, address: u32) -> Option<&mut ObjSplit> {
        match self.for_range_mut(..=address).next_back() {
            Some((_, split)) if split.end == 0 || split.end > address => Some(split),
            _ => None,
        }
    }

    /// Locate existing splits within the given address range.
    pub fn for_range<R>(&self, range: R) -> impl DoubleEndedIterator<Item = (u32, &ObjSplit)>
    where R: RangeBounds<u32> {
        self.splits.range(range).flat_map(|(addr, v)| v.iter().map(move |u| (*addr, u)))
    }

    /// Locate existing splits within the given address range.
    pub fn for_range_mut<R>(
        &mut self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = (u32, &mut ObjSplit)>
    where
        R: RangeBounds<u32>,
    {
        self.splits.range_mut(range).flat_map(|(addr, v)| v.iter_mut().map(move |u| (*addr, u)))
    }

    pub fn for_unit(&self, unit: &str) -> Result<Option<(u32, &ObjSplit)>> {
        self.splits
            .iter()
            .flat_map(|(addr, v)| v.iter().map(move |u| (*addr, u)))
            .filter(|&(_, split)| split.unit == unit)
            .at_most_one()
            .map_err(|_| anyhow!("Multiple splits for unit {}", unit))
    }

    pub fn push(&mut self, address: u32, split: ObjSplit) {
        self.splits.nested_push(address, split);
    }

    pub fn remove(&mut self, address: u32) -> Option<Vec<ObjSplit>> { self.splits.remove(&address) }
}