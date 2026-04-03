use std::hash::{Hash, Hasher};

use itertools::Itertools;
use rapidhash::fast::RapidHasher;

use bumpalo::{
    Bump,
    collections::{CollectIn, FromIteratorIn, Vec},
};

#[derive(Debug, Clone, Eq)]
pub struct UnorderedCardSet<'bump> {
    pub cards: Vec<'bump, (Card, u8)>,
}

impl<'bump> FromIteratorIn<Card> for UnorderedCardSet<'bump> {
    type Alloc = &'bump Bump;

    fn from_iter_in<I>(iter: I, bump: Self::Alloc) -> Self
    where
        I: IntoIterator<Item = Card>,
    {
        let counts = iter.into_iter().counts();

        Self {
            cards: counts
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.try_into().expect("More than u8::MAX identical cards???"),
                    )
                })
                .collect_in(bump),
        }
    }
}

impl UnorderedCardSet<'_> {
    pub fn launder(self, new_bump: &Bump) -> UnorderedCardSet<'_> {
        UnorderedCardSet {
            cards: self.cards.into_iter().collect_in(new_bump),
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        for (new_card, count) in other.cards.drain(..) {
            match self
                .cards
                .iter()
                .position(|(card, _count)| *card == new_card)
            {
                Some(idx) => self.cards[idx].1 += count,
                None => self.cards.push((new_card, count)),
            }
        }
    }

    pub fn add_card(&mut self, new_card: Card) {
        match self
            .cards
            .iter()
            .position(|(card, _count)| *card == new_card)
        {
            Some(idx) => self.cards[idx].1 += 1,
            None => self.cards.push((new_card, 1)),
        }
    }

    pub fn remove_card(&mut self, removed_card: Card) {
        match self
            .cards
            .iter()
            .position(|(card, _count)| *card == removed_card)
        {
            Some(idx) => {
                self.cards[idx].1 = self.cards[idx]
                    .1
                    .checked_sub(1)
                    .expect("Tried to remove a card that was not in the set");
            }
            None => unreachable!("Tried to remove a card that was not in the set"),
        }
    }

    pub fn num_cards(&self) -> usize {
        self.cards.iter().map(|(_k, v)| usize::from(*v)).sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Card> {
        self.cards.iter().filter_map(|(k, v)| (*v > 0).then_some(k))
    }

    pub fn iter_counts(&self) -> impl Iterator<Item = (&Card, u8)> {
        self.cards
            .iter()
            .filter_map(|(k, v)| (*v > 0).then_some((k, *v)))
    }

    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
}

impl PartialEq for UnorderedCardSet<'_> {
    fn eq(&self, other: &Self) -> bool {
        if self.iter().count() != self.iter().count() {
            return false;
        }
        // We have the same number of cards with non-zero count
        for v in &self.cards {
            if !other.cards.contains(v) {
                return false;
            }
        }
        return true;
    }
}

impl Hash for UnorderedCardSet<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut res: u64 = 1;
        for card in &self.cards {
            let mut card_hasher = RapidHasher::default_const();

            card.hash(&mut card_hasher);
            let card_hash = card_hasher.finish();

            res = res.wrapping_add(card_hash);
        }
        res.hash(state);
        // self.cards.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Card {
    pub prototype: CardPrototype,
    pub upgraded: bool,
    pub enchantment: Option<CardEnchantment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CardEnchantment {
    TezcatarasEmber,
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
    pub fn get_cost(self) -> Cost {
        let cost_without_enchantment = match (self.prototype, self.upgraded) {
            (CardPrototype::Strike, _) => ENERGY[1],
            (CardPrototype::Defend, _) => ENERGY[1],
            (CardPrototype::Neutralize, _) => ENERGY[0],

            // FIXME: DEBUG: Do not play Survivor since we cannot block yet
            (CardPrototype::Survivor, _) => ENERGY[1],
            (CardPrototype::PoisonedStab, _) => ENERGY[1],
            (CardPrototype::Backflip, _) => ENERGY[1],
            (CardPrototype::DeadlyPoison, _) => ENERGY[1],
            (CardPrototype::CorrosiveWave, _) => ENERGY[1],
            (CardPrototype::Footwork, _) => ENERGY[1],
            (CardPrototype::LegSweep, _) => ENERGY[2],
            (CardPrototype::AscendersBane, _) => ENERGY[0],
            (CardPrototype::Dazed, _) => ENERGY[0],
            (CardPrototype::Infection, _) => ENERGY[0],
            (CardPrototype::Wound, _) => ENERGY[0],
            (CardPrototype::Greed, _) => ENERGY[0],
            (CardPrototype::PreciseCut, _) => ENERGY[0],
            (CardPrototype::Anticipate, _) => ENERGY[0],
            (CardPrototype::NoxiousFumes, _) => ENERGY[1],
            (CardPrototype::Fasten, _) => ENERGY[1],
            (CardPrototype::DodgeAndRoll, _) => ENERGY[1],
            (CardPrototype::Shiv, _) => ENERGY[0],
            (CardPrototype::CloakAndDagger, _) => ENERGY[1],
            (CardPrototype::LeadingStrike, _) => ENERGY[1],
            (CardPrototype::Tracking, false) => ENERGY[2],
            (CardPrototype::Tracking, true) => ENERGY[1],
            (CardPrototype::SuckerPunch, _) => ENERGY[1],
            (CardPrototype::Haze, _) => ENERGY[3],
            (CardPrototype::Accuracy, _) => ENERGY[1],
            (CardPrototype::Squash, _) => ENERGY[1],
            (CardPrototype::Dash, _) => ENERGY[2],
            (CardPrototype::Burst, _) => ENERGY[1],
            (CardPrototype::BladeDance, _) => ENERGY[1],
        };

        if self.enchantment == Some(CardEnchantment::TezcatarasEmber) {
            Cost {
                energy: CostVal::Val(0),
                stars: cost_without_enchantment.stars,
            }
        } else {
            cost_without_enchantment
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
            CardPrototype::LegSweep => [LegalTarget::Enemy],
            CardPrototype::AscendersBane => [LegalTarget::OwnPlayer],
            CardPrototype::Dazed => [LegalTarget::OwnPlayer],
            CardPrototype::Infection => [LegalTarget::OwnPlayer],
            CardPrototype::Wound => [LegalTarget::OwnPlayer],
            CardPrototype::Greed => [LegalTarget::OwnPlayer],
            CardPrototype::PreciseCut => [LegalTarget::Enemy],
            CardPrototype::Anticipate => [LegalTarget::OwnPlayer],
            CardPrototype::NoxiousFumes => [LegalTarget::OwnPlayer],
            CardPrototype::Fasten => [LegalTarget::OwnPlayer],
            CardPrototype::DodgeAndRoll => [LegalTarget::OwnPlayer],
            CardPrototype::Shiv => [LegalTarget::Enemy],
            CardPrototype::CloakAndDagger => [LegalTarget::OwnPlayer],
            CardPrototype::LeadingStrike => [LegalTarget::Enemy],
            CardPrototype::Tracking => [LegalTarget::OwnPlayer],
            CardPrototype::SuckerPunch => [LegalTarget::Enemy],
            CardPrototype::Haze => [LegalTarget::OwnPlayer],
            CardPrototype::Accuracy => [LegalTarget::OwnPlayer],
            CardPrototype::Squash => [LegalTarget::Enemy],
            CardPrototype::Dash => [LegalTarget::Enemy],
            CardPrototype::Burst => [LegalTarget::OwnPlayer],
            CardPrototype::BladeDance => [LegalTarget::OwnPlayer],
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
            CardPrototype::LegSweep => Uncommon,
            CardPrototype::AscendersBane => Special,
            CardPrototype::Dazed => Special,
            CardPrototype::Infection => Special,
            CardPrototype::Wound => Special,
            CardPrototype::Greed => Special,
            CardPrototype::PreciseCut => Uncommon,
            CardPrototype::Anticipate => Common,
            CardPrototype::NoxiousFumes => Uncommon,
            CardPrototype::Fasten => Uncommon,
            CardPrototype::DodgeAndRoll => Common,
            CardPrototype::Shiv => Common,
            CardPrototype::CloakAndDagger => Common,
            CardPrototype::LeadingStrike => Common,
            CardPrototype::Tracking => Rare,
            CardPrototype::SuckerPunch => Common,
            CardPrototype::Haze => Uncommon,
            CardPrototype::Accuracy => Uncommon,
            CardPrototype::Squash => Special,
            CardPrototype::Dash => Uncommon,
            CardPrototype::Burst => Rare,
            CardPrototype::BladeDance => Common,
        }
    }

    pub fn has_unplayable(self) -> bool {
        match self.prototype {
            CardPrototype::AscendersBane => true,
            CardPrototype::Dazed => true,
            CardPrototype::Infection => true,
            CardPrototype::Greed => true,
            CardPrototype::Wound => true,
            _ => false,
        }
    }

    pub fn has_exhaust(self) -> bool {
        match self.prototype {
            CardPrototype::Shiv => true,
            CardPrototype::BladeDance => true,
            _ => false,
        }
    }

    pub fn has_sly(self) -> bool {
        match self.prototype {
            CardPrototype::Haze => true,
            _ => false,
        }
    }

    pub fn has_ethereal(self) -> bool {
        match self.prototype {
            CardPrototype::Dazed => true,
            CardPrototype::AscendersBane => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
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
    LegSweep,
    AscendersBane,
    Dazed,
    Infection,
    Wound,
    Greed,
    PreciseCut,
    Anticipate,
    NoxiousFumes,
    Fasten,
    DodgeAndRoll,
    Shiv,
    CloakAndDagger,
    LeadingStrike,
    Tracking,
    SuckerPunch,
    Haze,
    Accuracy,
    Squash,
    Dash,
    Burst,
    BladeDance,
}

impl CardPrototype {
    pub const fn get_normal_card(self) -> Card {
        Card {
            prototype: self,
            upgraded: false,
            enchantment: None,
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
            Self::LegSweep => Skill,
            Self::PreciseCut => Attack,
            Self::AscendersBane => Curse,
            Self::Dazed => Status,
            Self::Infection => Status,
            Self::Wound => Status,
            Self::Greed => Curse,
            Self::Anticipate => Skill,
            Self::NoxiousFumes => Power,
            Self::Fasten => Power,
            Self::DodgeAndRoll => Skill,
            Self::Shiv => Attack,
            Self::CloakAndDagger => Skill,
            Self::LeadingStrike => Attack,
            Self::Tracking => Power,
            Self::SuckerPunch => Attack,
            Self::Haze => Skill,
            Self::Accuracy => Power,
            Self::Squash => Attack,
            Self::Dash => Attack,
            Self::Burst => Skill,
            Self::BladeDance => Skill,
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
