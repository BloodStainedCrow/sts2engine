use std::{
    collections::HashMap,
    hash::Hash,
    iter::Sum,
    ops::{ControlFlow, Mul, MulAssign},
};

#[derive(Debug)]
pub struct Distribution<Value> {
    pub entries: Vec<(Value, f32)>,
}

impl<Value: Copy + Sum<Value> + Mul<f32, Output = Value>> Distribution<Value> {
    #[must_use]
    pub(crate) fn expected_value(&self) -> Value {
        self.entries.iter().map(|(v, chance)| *v * *chance).sum()
    }
}

impl<Value> MulAssign<f32> for Distribution<Value> {
    fn mul_assign(&mut self, rhs: f32) {
        for (_val, chance) in &mut self.entries {
            *chance *= rhs;
        }
    }
}

impl<Value> Distribution<Distribution<Value>> {
    #[must_use]
    pub(crate) fn flatten(self) -> Distribution<Value> {
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

impl<Value: PartialEq + Eq + Hash> Distribution<Value> {
    pub(crate) fn dedup(&mut self) {
        let mut new_entries = HashMap::new();

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
}

impl<Value> Distribution<Value> {
    #[must_use]
    pub(crate) fn single_value(value: Value) -> Self {
        Self {
            entries: vec![(value, 1.0)],
        }
    }

    #[must_use]
    pub(crate) fn equal_chance(values: impl IntoIterator<Item = Value>) -> Self {
        let mut entries: Vec<(Value, f32)> = values.into_iter().map(|v| (v, 0.0)).collect();

        let count = entries.len() as f32;

        for (_, chance) in &mut entries {
            *chance = 1.0 / count;
        }

        Self { entries }
    }

    #[must_use]
    pub(crate) fn from_duplicates(values: impl IntoIterator<Item = (Value, usize)>) -> Self {
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

    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub(crate) fn collapse(self) -> Value {
        let random: f32 = rand::random_range(0.0..1.0);
        // let random: f32 = 0.0;

        let mut done = 0.0;

        assert!(!self.entries.is_empty());

        for (val, chance) in self.entries {
            let new_done = done + chance;
            if new_done > random {
                return val;
            }
            done = new_done;
        }
        unreachable!()
    }

    #[must_use]
    pub(crate) fn fix_odds(mut self) -> Self {
        let sum: f32 = self.entries.iter().map(|(_, chance)| *chance).sum();
        for (_, chance) in self.entries.iter_mut() {
            *chance = *chance / sum;
        }

        self
    }

    #[must_use]
    pub(crate) fn retain_no_chance_fix(&mut self, filter: impl Fn(&Value) -> bool) {
        self.entries.retain(|(v, _)| (filter)(v));
    }

    #[must_use]
    pub(crate) fn map<T>(self, mut fun: impl FnMut(Value) -> T) -> Distribution<T> {
        let Self { entries } = self;

        Distribution {
            entries: entries
                .into_iter()
                .map(|(val, chance)| ((fun)(val), chance))
                .collect(),
        }
    }

    pub(crate) fn try_map<B, T>(
        self,
        mut fun: impl FnMut(Value) -> ControlFlow<B, T>,
    ) -> ControlFlow<B, Distribution<T>> {
        let Self { entries } = self;

        let entries: Vec<_> = entries
            .into_iter()
            .map(|(val, chance)| (fun)(val).map_continue(|v| (v, chance)))
            .try_collect::<Vec<_>>()?;

        ControlFlow::Continue(Distribution { entries })
    }

    #[must_use]
    pub(crate) fn flat_map<T>(
        self,
        mut fun: impl FnMut(Value) -> Distribution<T>,
    ) -> Distribution<T> {
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
}
