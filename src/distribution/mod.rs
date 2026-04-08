pub mod full;
pub mod single;
use std::{
    cmp::Ordering,
    hash::Hash,
    iter::Sum,
    ops::{Mul, MulAssign},
};
pub trait Distribution<Value: 'static>:
    MulAssign<f32> + From<full::Distribution<Value>> + IntoIterator<Item = (Value, f32)>
{
    type Inner<V: 'static>: Distribution<V, Inner<Value> = Self>;

    #[must_use]
    fn single_value(value: Value) -> Self;
    #[must_use]
    fn equal_chance(values: impl IntoIterator<Item = Value>) -> Self;

    #[must_use]
    fn from_duplicates(values: impl IntoIterator<Item = (Value, usize)>) -> Self;

    #[must_use]
    fn len(&self) -> usize;

    #[must_use]
    fn is_empty(&self) -> bool;

    #[must_use]
    fn collapse(self) -> Value;

    #[must_use]
    fn fix_odds(self) -> Self;

    fn retain_no_chance_fix(&mut self, filter: impl FnMut(&Value) -> bool);

    #[must_use]
    fn map<T: 'static>(self, fun: impl FnMut(Value) -> T) -> Self::Inner<T>;

    #[must_use]
    fn flat_map<T: 'static>(self, fun: impl FnMut(Value) -> Self::Inner<T>) -> Self::Inner<T>;

    #[must_use]
    fn flat_map_simple(self, fun: impl FnMut(Value) -> Self) -> Self;

    fn into_values(self) -> impl Iterator<Item = Value>;

    fn iter_with_odds(&self) -> impl Iterator<Item = (&Value, f32)>;

    fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&(Value, f32), &(Value, f32)) -> Ordering;

    fn dedup(&mut self)
    where
        Value: PartialEq + Eq + Hash;

    fn all_unique(&self) -> bool
    where
        Value: Eq + Hash;

    fn expected_value(&self) -> Value
    where
        Value: Copy + Sum<Value> + Mul<f32, Output = Value>;

    fn flatten<T: 'static>(self) -> Self::Inner<T>
    where
        Value: Distribution<T>;
}

pub trait Flatten<Value: 'static, D: Distribution<Value> + 'static>: Distribution<D> {
    fn flatten(self) -> Self::Inner<Value>;
}
