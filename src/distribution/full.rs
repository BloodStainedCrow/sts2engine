use std::{
    cmp::Ordering,
    hash::Hash,
    iter::Sum,
    ops::{Mul, MulAssign},
    vec,
};

use itertools::Itertools;
use rapidhash::{HashMapExt, RapidHashMap};

#[derive(Debug, Clone)]
pub struct Distribution<Value> {
    entries: Vec<(Value, f32)>,
}

impl<Value> MulAssign<f32> for Distribution<Value> {
    fn mul_assign(&mut self, rhs: f32) {
        for (_val, chance) in &mut self.entries {
            *chance *= rhs;
        }
    }
}

impl<Value: 'static> super::Distribution<Value> for Distribution<Value> {
    type Inner<V: 'static> = Distribution<V>;

    fn single_value(value: Value) -> Self {
        Self {
            entries: vec![(value, 1.0)],
        }
    }

    fn equal_chance(values: impl IntoIterator<Item = Value>) -> Self {
        let mut entries: Vec<(Value, f32)> = values.into_iter().map(|v| (v, 0.0)).collect();

        let count = entries.len() as f32;

        for (_, chance) in &mut entries {
            *chance = 1.0 / count;
        }

        Self { entries }
    }

    fn from_duplicates(values: impl IntoIterator<Item = (Value, usize)>) -> Self {
        let mut entries: Vec<(Value, f32)> = values
            .into_iter()
            .map(|(v, count)| (v, count as f32))
            .collect();

        let count: f32 = entries.iter().map(|(_, count)| count).sum();

        for (_, chance) in &mut entries {
            *chance /= count;
        }

        Self { entries }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn collapse(self) -> Value {
        let random: f32 = rand::random_range(0.0..1.0);
        // let random: f32 = 0.0;

        let mut done = 0.0;

        assert!(!self.entries.is_empty());

        let len = self.entries.len();

        for (i, (val, chance)) in self.entries.into_iter().enumerate() {
            let new_done = done + chance;
            if new_done > random || i == len - 1 {
                return val;
            }
            done = new_done;
        }
        unreachable!("")
    }

    fn fix_odds(mut self) -> Self {
        let sum: f32 = self.entries.iter().map(|(_, chance)| *chance).sum();
        for (_, chance) in &mut self.entries {
            *chance /= sum;
        }

        self
    }

    fn retain_no_chance_fix(&mut self, mut filter: impl FnMut(&Value) -> bool) {
        self.entries.retain(|(v, _)| (filter)(v));
    }

    fn map<T: 'static>(self, mut fun: impl FnMut(Value) -> T) -> Self::Inner<T> {
        let Self { entries } = self;

        Distribution {
            entries: entries
                .into_iter()
                .map(|(val, chance)| ((fun)(val), chance))
                .collect(),
        }
    }

    fn flat_map<T: 'static>(self, mut fun: impl FnMut(Value) -> Self::Inner<T>) -> Self::Inner<T> {
        let Self { mut entries } = self;

        if entries.len() == 1 {
            let Some((value, _chance)) = entries.pop() else {
                unreachable!()
            };
            (fun)(value)
        } else {
            // Note: This does not deduplicate
            let reduced = entries
                .into_iter()
                .flat_map(|(entry, chance)| {
                    let mapped = (fun)(entry);
                    mapped
                        .entries
                        .into_iter()
                        .map(move |(v, inner)| (v, inner * chance))
                })
                .collect();

            Distribution { entries: reduced }
        }
    }

    fn flat_map_simple(self, mut fun: impl FnMut(Value) -> Self) -> Self {
        let Self { mut entries } = self;

        if entries.len() == 1 {
            let Some((value, _chance)) = entries.pop() else {
                unreachable!()
            };
            (fun)(value)
        } else {
            // Note: This does not deduplicate
            let reduced = entries
                .into_iter()
                .flat_map(|(entry, chance)| {
                    let mapped = (fun)(entry);
                    mapped
                        .entries
                        .into_iter()
                        .map(move |(v, inner)| (v, inner * chance))
                })
                .collect();

            Distribution { entries: reduced }
        }
    }

    fn into_values(self) -> impl Iterator<Item = Value> {
        self.entries.into_iter().map(|(v, _)| v)
    }

    fn iter_with_odds(&self) -> impl Iterator<Item = (&Value, f32)> {
        self.entries.iter().map(|(a, b)| (a, *b))
    }

    fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&(Value, f32), &(Value, f32)) -> Ordering,
    {
        self.entries.sort_by(|a, b| compare(a, b));
    }

    fn dedup(&mut self)
    where
        Value: Eq + Hash,
    {
        let mut new_entries = RapidHashMap::new();

        for (val, chance) in self.entries.drain(..) {
            match new_entries.entry(val) {
                std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                    *occupied_entry.get_mut() += chance;
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(chance);
                }
            }
        }

        self.entries.extend(new_entries);
    }

    fn all_unique(&self) -> bool
    where
        Value: Eq + Hash,
    {
        self.entries.iter().map(|v| &v.0).all_unique()
    }

    fn expected_value(&self) -> Value
    where
        Value: Copy + Sum<Value> + Mul<f32, Output = Value>,
    {
        self.entries.iter().map(|(v, chance)| *v * *chance).sum()
    }

    fn flatten<T: 'static>(self) -> Self::Inner<T>
    where
        Value: super::Distribution<T>,
    {
        Distribution {
            entries: self
                .entries
                .into_iter()
                .flat_map(|(v, odds)| {
                    v.into_iter()
                        .map(move |(inner, inner_chance)| (inner, inner_chance * odds))
                })
                .collect(),
        }
    }
}

impl<Value: 'static> IntoIterator for Distribution<Value> {
    type Item = (Value, f32);

    type IntoIter = vec::IntoIter<(Value, f32)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<Value: 'static> super::Flatten<Value, Distribution<Value>>
    for Distribution<Distribution<Value>>
{
    fn flatten(self) -> Self::Inner<Value> {
        let Self { entries } = self;

        // Note: This does not deduplicate
        let reduced = entries
            .into_iter()
            .flat_map(|(mut entry, chance)| {
                entry *= chance;
                entry.entries
            })
            .collect();

        Distribution { entries: reduced }
    }
}
