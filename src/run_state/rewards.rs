use std::array;

use strum::IntoEnumIterator;

use crate::{
    combat_state::{
        cards::{Card, CardPrototype, Rarity},
        relics::RelicPrototype,
    },
    distribution::{Distribution, DistributionFamily},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CardReward {
    pub options: Vec<Card>,
}

impl CardReward {
    pub fn from_rarity<Family: DistributionFamily>(rarity: Rarity) -> Family::Distribution<Self> {
        let options: [_; 3] = array::from_fn(|_| {
            Family::Distribution::<CardPrototype>::equal_chance(
                CardPrototype::iter().filter(|card| card.get_rarity() == rarity),
            )
        });

        let [first, second, third] = options;

        first.cartesian_product(
            second.cartesian_product(third, |second, third| [second, third]),
            |first, [second, third]: [CardPrototype; 2]| Self {
                options: vec![
                    first.get_normal_card(),
                    second.get_normal_card(),
                    third.get_normal_card(),
                ],
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelicReward {
    pub relic: RelicPrototype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RewardAction {
    Continue {},
    Card { reward: usize, card: Card },
    Relic { reward: usize },
}
