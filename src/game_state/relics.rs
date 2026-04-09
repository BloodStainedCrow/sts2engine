use enum_map::{Enum, EnumMap};

#[derive(Debug, Clone, Copy, Enum, PartialEq, Eq, serde::Deserialize)]
pub enum RelicPrototype {
    RingOfTheSnake,
    RingOfTheDrake,
    CursedPearl,
    Pomander,
    ToxicEgg,
    OddlySmoothStone,
    NutritiousSoup,
    Gorget,
    MealTicket,
    BoomingConch,
    Vajra,
    JewelryBox,
    BronzeScales,
    Anchor,
    BagOfMarbles,
    Orichalcum,
    BagOfPreparation,
    StrikeDummy,
    StoneHumidifier,
    Bellows,
    NutritiousOyster,
    GoldenPearl,
    Vanbrace,
    LostCoffer,
    BloodVial,
    Lantern,
    Shuriken,
    Whetstone,
    BiiigHug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullRelicState {
    numbers: EnumMap<RelicPrototype, u8>,
}

impl FromIterator<RelicPrototype> for FullRelicState {
    fn from_iter<T: IntoIterator<Item = RelicPrototype>>(iter: T) -> Self {
        let mut ret = Self::default();

        for proto in iter {
            ret.numbers[proto] = 0;
        }

        ret
    }
}

impl Default for FullRelicState {
    fn default() -> Self {
        Self {
            numbers: EnumMap::from_fn(|_| u8::MAX),
        }
    }
}

pub type SingleRelicState = Option<u8>;

impl FullRelicState {
    pub fn get_state(&self, relic: RelicPrototype) -> SingleRelicState {
        let val = self.numbers[relic];

        (val != u8::MAX).then_some(val)
    }

    pub fn set_state(&mut self, relic: RelicPrototype, v: u8) {
        self.numbers[relic] = v;
    }

    pub fn set_state_if_present(&mut self, relic: RelicPrototype, v: u8) {
        if self.contains(relic) {
            self.numbers[relic] = v;
        }
    }

    pub fn contains(&self, relic: RelicPrototype) -> bool {
        let val = self.numbers[relic];

        val != u8::MAX
    }
}
