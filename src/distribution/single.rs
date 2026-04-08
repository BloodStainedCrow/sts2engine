use rand::{random_range, rng, seq::IteratorRandom};
use std::{
    cmp::Ordering,
    hash::Hash,
    iter::{self, Sum},
    ops::{Mul, MulAssign},
};

#[derive(Debug, Clone)]
pub struct Distribution<Value> {
    value: Value,
}

impl<Value> MulAssign<f32> for Distribution<Value> {
    fn mul_assign(&mut self, _rhs: f32) {}
}

impl<Value> Distribution<Distribution<Value>> {
    fn flatten(self) -> Distribution<Value> {
        let Self { value, .. } = self;

        Distribution::<Value> { value: value.value }
    }
}

impl<Value: 'static> super::Distribution<Value> for Distribution<Value> {
    type Inner<V: 'static> = Distribution<V>;

    fn single_value(value: Value) -> Self {
        Self { value }
    }

    fn equal_chance(values: impl IntoIterator<Item = Value>) -> Self {
        let value = values
            .into_iter()
            .choose(&mut rng())
            .expect("Distribution::equal_chance needs at least one value");

        Self { value }
    }

    fn from_duplicates(values: impl IntoIterator<Item = (Value, usize)>) -> Self {
        let entries: Vec<(Value, _)> = values.into_iter().collect();

        let sum = entries.iter().map(|v| v.1).sum();

        let mut v = random_range(0..sum);

        let entry = entries
            .into_iter()
            .find(|(_value, count)| {
                if count >= &v {
                    true
                } else {
                    v -= count;
                    false
                }
            })
            .expect("The range is the sum");

        Self { value: entry.0 }
    }

    fn len(&self) -> usize {
        1
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn collapse(self) -> Value {
        self.value
    }

    fn fix_odds(self) -> Self {
        self
    }

    fn retain_no_chance_fix(&mut self, mut filter: impl FnMut(&Value) -> bool) {
        assert!(
            (filter)(&self.value),
            "Removed last value from distribution"
        );
    }

    fn map<T>(self, mut fun: impl FnMut(Value) -> T) -> Distribution<T> {
        let Self { value, .. } = self;

        Distribution {
            value: (fun)(value),
        }
    }

    fn flat_map<T>(self, mut fun: impl FnMut(Value) -> Distribution<T>) -> Distribution<T> {
        let Self { value, .. } = self;

        let dis = (fun)(value);

        Distribution { value: dis.value }
    }

    fn flat_map_simple(self, mut fun: impl FnMut(Value) -> Self) -> Self {
        Self {
            value: (fun)(self.value).value,
        }
    }

    fn into_values(self) -> impl Iterator<Item = Value> {
        iter::once(self.value)
    }

    fn iter_with_odds(&self) -> impl Iterator<Item = (&Value, f32)> {
        iter::once((&self.value, 1.0))
    }

    fn sort_by<F>(&mut self, _compare: F)
    where
        F: FnMut(&(Value, f32), &(Value, f32)) -> Ordering,
    {
    }

    fn dedup(&mut self)
    where
        Value: PartialEq + Eq + Hash,
    {
    }

    fn all_unique(&self) -> bool
    where
        Value: Eq + Hash,
    {
        true
    }

    fn expected_value(&self) -> Value
    where
        Value: Copy + Sum<Value> + Mul<f32, Output = Value>,
    {
        self.value
    }

    fn flatten<T: 'static>(self) -> Self::Inner<T>
    where
        Value: super::Distribution<T>,
    {
        Distribution {
            value: self.value.collapse(),
        }
    }
}

impl<Value: 'static> From<super::full::Distribution<Value>> for Distribution<Value> {
    fn from(value: super::full::Distribution<Value>) -> Self {
        Self {
            value: super::Distribution::collapse(value),
        }
    }
}

impl<Value: 'static> IntoIterator for Distribution<Value> {
    type Item = (Value, f32);

    type IntoIter = iter::Once<(Value, f32)>;

    fn into_iter(self) -> Self::IntoIter {
        iter::once((self.value, 1.0))
    }
}

impl<Value: 'static> super::Flatten<Value, Distribution<Value>>
    for Distribution<Distribution<Value>>
{
    fn flatten(self) -> Self::Inner<Value> {
        let Self { value } = self;

        Distribution { value: value.value }
    }
}
