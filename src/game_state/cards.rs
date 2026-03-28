use std::{
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use rapidhash::{HashMapExt, RapidHashMap, fast::RapidHasher};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnorderedCardSet {
    pub cards: RapidHashMap<Card, u8>,
}

impl From<Vec<Card>> for UnorderedCardSet {
    fn from(value: Vec<Card>) -> Self {
        Self {
            cards: {
                let mut counts: RapidHashMap<Card, u8> = RapidHashMap::new();
                for item in value {
                    *counts.entry(item).or_default() += 1;
                }
                counts
            },
        }
    }
}

impl UnorderedCardSet {
    pub fn append(&mut self, other: &mut Self) {
        for (k, v) in other.drain() {
            match self.entry(k) {
                std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                    *occupied_entry.get_mut() += v;
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(v);
                }
            }
        }
    }

    pub fn add_card(&mut self, card: Card) {
        match self.entry(card) {
            std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() += 1;
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(1);
            }
        }
    }

    pub fn remove_card(&mut self, card: Card) {
        // let error = format!("Cannot remove card: {card:?}, that is not in the set {self:?}");
        let count = self
            .get_mut(&card)
            .expect("Cannot remove card this is not in the set");

        *count = count
            .checked_sub(1)
            .expect("Cannot remove card this is not in the set");
    }

    pub fn iter(&self) -> impl Iterator<Item = &Card> {
        self.cards.iter().filter_map(|(k, v)| (*v > 0).then_some(k))
    }

    pub fn is_empty(&self) -> bool {
        self.values().all(|v| *v == 0)
    }
}

impl Deref for UnorderedCardSet {
    type Target = RapidHashMap<Card, u8>;

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

impl DerefMut for UnorderedCardSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cards
    }
}

impl Hash for UnorderedCardSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut res: u64 = 1;
        for card in &self.cards {
            let mut card_hasher = RapidHasher::default_const();

            card.hash(&mut card_hasher);
            let card_hash = card_hasher.finish();

            res = res.wrapping_add(card_hash);
        }
        res.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Card {
    pub prototype: CardPrototype,
    pub upgraded: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Cost {
    pub energy: CostVal,
    pub stars: CostVal,
}

#[derive(Debug, Clone, Copy)]
pub enum CostVal {
    X,
    Val(u8),
}

const ENERGY: [Cost; 5] = [
    Cost {
        energy: CostVal::Val(0),
        stars: CostVal::Val(0),
    },
    Cost {
        energy: CostVal::Val(1),
        stars: CostVal::Val(0),
    },
    Cost {
        energy: CostVal::Val(2),
        stars: CostVal::Val(0),
    },
    Cost {
        energy: CostVal::Val(3),
        stars: CostVal::Val(0),
    },
    Cost {
        energy: CostVal::Val(4),
        stars: CostVal::Val(0),
    },
];

impl Card {
    #[allow(clippy::match_same_arms)]
    pub const fn get_cost(self) -> Cost {
        match (self.prototype, self.upgraded) {
            (CardPrototype::Strike, _) => ENERGY[1],
            (CardPrototype::Defend, _) => ENERGY[1],
            (CardPrototype::Neutralize, _) => ENERGY[0],

            // FIXME: DEBUG: Do not play Survivor since we cannot block yet
            (CardPrototype::Survivor, _) => ENERGY[4],
            (CardPrototype::PoisonedStab, _) => ENERGY[1],
            (CardPrototype::Backflip, _) => ENERGY[1],
            (CardPrototype::DeadlyPoison, _) => ENERGY[1],
            (CardPrototype::CorrosiveWave, _) => ENERGY[1],
            (CardPrototype::Footwork, _) => ENERGY[1],
        }
    }

    #[allow(clippy::match_same_arms)]
    pub fn get_legal_targets(self) -> impl Iterator<Item = LegalTarget> {
        match self.prototype {
            CardPrototype::Strike => [LegalTarget::Enemy],
            CardPrototype::Defend => [LegalTarget::OwnPlayer],
            CardPrototype::Neutralize => [LegalTarget::Enemy],
            CardPrototype::Survivor => [LegalTarget::OwnPlayer],
            CardPrototype::PoisonedStab => [LegalTarget::Enemy],
            CardPrototype::Backflip => [LegalTarget::OwnPlayer],
            CardPrototype::DeadlyPoison => [LegalTarget::Enemy],
            CardPrototype::CorrosiveWave => [LegalTarget::OwnPlayer],
            CardPrototype::Footwork => [LegalTarget::OwnPlayer],
        }
        .into_iter()
    }

    #[allow(clippy::match_same_arms)]
    #[allow(clippy::enum_glob_use)]
    pub fn get_rarity(self) -> Rarity {
        use Rarity::*;
        match self.prototype {
            CardPrototype::Strike => Basic,
            CardPrototype::Defend => Basic,
            CardPrototype::Neutralize => Common,
            CardPrototype::Survivor => Common,
            CardPrototype::PoisonedStab => Common,
            CardPrototype::Backflip => Common,
            CardPrototype::DeadlyPoison => Common,
            CardPrototype::CorrosiveWave => Rare,
            CardPrototype::Footwork => Uncommon,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Basic,
    Common,
    Uncommon,
    Rare,
}

pub enum LegalTarget {
    OwnPlayer,
    OtherPlayer,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum CardPrototype {
    #[serde(alias = "STRIKE_SILENT")]
    Strike,
    #[serde(alias = "DEFEND_SILENT")]
    Defend,
    Neutralize,
    Survivor,
    PoisonedStab,
    Backflip,
    DeadlyPoison,
    CorrosiveWave,
    Footwork,
}

impl CardPrototype {
    pub const fn get_normal_card(self) -> Card {
        Card {
            prototype: self,
            upgraded: false,
        }
    }

    #[allow(clippy::enum_glob_use)]
    #[allow(clippy::match_same_arms)]
    pub const fn get_kind(self) -> CardKind {
        use CardKind::*;
        match self {
            Self::Strike => Attack,
            Self::Defend => Skill,
            Self::Neutralize => Attack,
            Self::Survivor => Skill,
            Self::PoisonedStab => Attack,
            Self::Backflip => Skill,
            Self::DeadlyPoison => Skill,
            Self::CorrosiveWave => Skill,
            Self::Footwork => Power,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}
