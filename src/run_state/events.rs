use std::iter;

use itertools::Itertools;
use strum::EnumIter;

use crate::{
    combat_state::{CombatState, relics::RelicPrototype},
    distribution::{self, Distribution, DistributionFamily},
    run_state::{Act, ActPrototype, RunState, SubState, rewards::RelicReward},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum EventPrototype {
    // Ancients
    Neow,
    Orobas,
    Pael,
    Tezcatara,
    Darv,
    Nonupeipe,
    Tanx,
    Vakuu,

    // Shared Events (All Acts)
    SelfHelpBook,
    SlipperyBridge,
    TheFutureOfPotions,
    ThisOrThat,

    // Overgrowth
    AromaOfChaos,
    ByrdonisNest,
    DenseVegetation,
    JungleMazeAdventure,
    LuminousChoir,
    MorphicGrove,
    SapphireSeed,
    TabletOfTruth,
    UnrestSite,
    Wellspring,
    WhisperingHollow,
    WoodCarvings,

    // Overgrowth + Underdocks
    TheLegendsWereTrue,

    // Overgrowth + Underdocks + Hive
    BrainLeech,
    RoomFullOfCheese,
    TeaMaster,

    // Hive
    Amalgamator,
    Bugslayer,
    ColorfulPhilosophers,
    ColossalFlower,
    DollRoom,
    FieldOfMansizedHoles,
    InfestedAutomaton,
    RanwidTheElder,
    SpiritGrafter,
    StoneOfAllTime,
    TheLanternKey,
    TheLostWisp,
    WelcomeToWongos,
    ZenWeaver,

    // Hive + Glory
    CrystalSphere,
    PotionCourier,
    RelicTrader,
    Symbiote,
    TheMerchant,
}

#[derive(Debug, Clone, Copy)]
pub enum EventLocationInfo {
    Ancient,
    NormalQuestionmark,
}

impl EventPrototype {
    #[allow(clippy::match_like_matches_macro)]
    #[allow(clippy::enum_glob_use)]
    pub fn can_spawn(self, current_act: &Act, info: EventLocationInfo) -> bool {
        use ActPrototype::*;
        use EventLocationInfo::*;
        use EventPrototype::*;

        let can_spawn_act = match (self, &current_act.prototype) {
            (Neow, Overgrowth | Underdocks) => true,
            (Orobas, Hive) => true,
            (Pael, Hive) => true,
            (Tezcatara, Hive) => true,
            (Darv, Hive | Glory) => true,
            (Nonupeipe, Glory) => true,
            (Tanx, Glory) => true,
            (Vakuu, Glory) => true,

            (SelfHelpBook, _) => true,
            (SlipperyBridge, _) => true,
            (TheFutureOfPotions, _) => true,
            (ThisOrThat, _) => true,

            (AromaOfChaos, Overgrowth) => true,
            (ByrdonisNest, Overgrowth) => true,
            (DenseVegetation, Overgrowth) => true,
            (JungleMazeAdventure, Overgrowth) => true,
            (LuminousChoir, Overgrowth) => true,
            (MorphicGrove, Overgrowth) => true,
            (SapphireSeed, Overgrowth) => true,
            (TabletOfTruth, Overgrowth) => true,
            (UnrestSite, Overgrowth) => true,
            (Wellspring, Overgrowth) => true,
            (WhisperingHollow, Overgrowth) => true,
            (WoodCarvings, Overgrowth) => true,

            (TheLegendsWereTrue, Overgrowth | Hive) => true,

            (BrainLeech, Overgrowth | Underdocks | Hive) => true,
            (RoomFullOfCheese, Overgrowth | Underdocks | Hive) => true,
            (TeaMaster, Overgrowth | Underdocks | Hive) => true,

            (Amalgamator, Hive) => true,
            (Bugslayer, Hive) => true,
            (ColorfulPhilosophers, Hive) => true,
            (ColossalFlower, Hive) => true,
            (DollRoom, Hive) => true,
            (FieldOfMansizedHoles, Hive) => true,
            (InfestedAutomaton, Hive) => true,
            (RanwidTheElder, Hive) => true,
            (SpiritGrafter, Hive) => true,
            (StoneOfAllTime, Hive) => true,
            (TheLanternKey, Hive) => true,
            (TheLostWisp, Hive) => true,
            (WelcomeToWongos, Hive) => true,
            (ZenWeaver, Hive) => true,

            (CrystalSphere, Glory) => true,
            (PotionCourier, Glory) => true,
            (RelicTrader, Glory) => true,
            (Symbiote, Glory) => true,
            (TheMerchant, Glory) => true,

            _ => false,
        };

        if !can_spawn_act {
            return false;
        }

        let can_spawn_location = match (self, info) {
            (Neow, Ancient) => true,
            (Orobas, Ancient) => true,
            (Pael, Ancient) => true,
            (Tezcatara, Ancient) => true,
            (Darv, Ancient) => true,
            (Nonupeipe, Ancient) => true,
            (Tanx, Ancient) => true,
            (Vakuu, Ancient) => true,

            (SelfHelpBook, NormalQuestionmark) => true,
            (SlipperyBridge, NormalQuestionmark) => true,
            (TheFutureOfPotions, NormalQuestionmark) => true,
            (ThisOrThat, NormalQuestionmark) => true,
            (AromaOfChaos, NormalQuestionmark) => true,
            (ByrdonisNest, NormalQuestionmark) => true,
            (DenseVegetation, NormalQuestionmark) => true,
            (JungleMazeAdventure, NormalQuestionmark) => true,
            (LuminousChoir, NormalQuestionmark) => true,
            (MorphicGrove, NormalQuestionmark) => true,
            (SapphireSeed, NormalQuestionmark) => true,
            (TabletOfTruth, NormalQuestionmark) => true,
            (UnrestSite, NormalQuestionmark) => true,
            (Wellspring, NormalQuestionmark) => true,
            (WhisperingHollow, NormalQuestionmark) => true,
            (WoodCarvings, NormalQuestionmark) => true,
            (TheLegendsWereTrue, NormalQuestionmark) => true,
            (BrainLeech, NormalQuestionmark) => true,
            (RoomFullOfCheese, NormalQuestionmark) => true,
            (TeaMaster, NormalQuestionmark) => true,
            (Amalgamator, NormalQuestionmark) => true,
            (Bugslayer, NormalQuestionmark) => true,
            (ColorfulPhilosophers, NormalQuestionmark) => true,
            (ColossalFlower, NormalQuestionmark) => true,
            (DollRoom, NormalQuestionmark) => true,
            (FieldOfMansizedHoles, NormalQuestionmark) => true,
            (InfestedAutomaton, NormalQuestionmark) => true,
            (RanwidTheElder, NormalQuestionmark) => true,
            (SpiritGrafter, NormalQuestionmark) => true,
            (StoneOfAllTime, NormalQuestionmark) => true,
            (TheLanternKey, NormalQuestionmark) => true,
            (TheLostWisp, NormalQuestionmark) => true,
            (WelcomeToWongos, NormalQuestionmark) => true,
            (ZenWeaver, NormalQuestionmark) => true,
            (CrystalSphere, NormalQuestionmark) => true,
            (PotionCourier, NormalQuestionmark) => true,
            (RelicTrader, NormalQuestionmark) => true,
            (Symbiote, NormalQuestionmark) => true,
            (TheMerchant, NormalQuestionmark) => true,

            _ => false,
        };

        if !can_spawn_location {
            return false;
        }

        // TODO: More conditions

        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    Ancient { options: [RelicPickupOption; 3] },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelicPickupOption {
    relic: RelicPrototype,
}

const NEOW_CURSED_RELIC_POOL: [RelicPrototype; 8] = [
    RelicPrototype::CursedPearl,
    RelicPrototype::HeftyTablet,
    RelicPrototype::LargeCapsule,
    RelicPrototype::LeafyPoultice,
    RelicPrototype::PrecariousShears,
    RelicPrototype::ScrollBoxes,
    RelicPrototype::NeowsBones,
    RelicPrototype::SilverCrucible,
];

const NEOW_POSITIVE_RELIC_POOL: [RelicPrototype; 18] = [
    RelicPrototype::BoomingConch,
    RelicPrototype::LostCoffer,
    RelicPrototype::GoldenPearl,
    RelicPrototype::NutritiousOyster,
    RelicPrototype::StoneHumidifier,
    RelicPrototype::LeadPaperweight,
    RelicPrototype::NeowsTorment,
    RelicPrototype::PhialHolster,
    RelicPrototype::WingedBoots,
    RelicPrototype::ArcaneScroll,
    RelicPrototype::NewLeaf,
    RelicPrototype::PreciseScissors,
    RelicPrototype::LavaRock,
    RelicPrototype::SmallCapsule,
    RelicPrototype::NutritiousOyster,
    RelicPrototype::StoneHumidifier,
    RelicPrototype::NeowsTalisman,
    RelicPrototype::Pomander,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventAction {
    Option { index: usize },
}

impl Event {
    // TODO: More information needed like deck, relics etc
    pub fn from_prototype<Family: DistributionFamily>(
        prototype: EventPrototype,
    ) -> Family::Distribution<Self> {
        match prototype {
            EventPrototype::Neow => Family::Distribution::<Self>::equal_chance(
                NEOW_CURSED_RELIC_POOL
                    .iter()
                    .copied()
                    .flat_map(|cursed| {
                        iter::once(cursed).cartesian_product(
                            NEOW_POSITIVE_RELIC_POOL
                                .iter()
                                .filter(move |pos| match pos {
                                    RelicPrototype::GoldenPearl => {
                                        cursed != RelicPrototype::CursedPearl
                                    }
                                    RelicPrototype::ArcaneScroll => {
                                        cursed != RelicPrototype::HeftyTablet
                                    }
                                    RelicPrototype::NewLeaf => {
                                        cursed != RelicPrototype::LeafyPoultice
                                    }
                                    RelicPrototype::PreciseScissors => {
                                        cursed != RelicPrototype::PrecariousShears
                                    }
                                    RelicPrototype::LavaRock | RelicPrototype::SmallCapsule => {
                                        cursed != RelicPrototype::LargeCapsule
                                    }

                                    _ => true,
                                })
                                .copied(),
                        )
                    })
                    .flat_map(|(cursed, pos)| {
                        iter::once((cursed, pos)).cartesian_product(
                            NEOW_POSITIVE_RELIC_POOL
                                .iter()
                                .filter(move |pos| match pos {
                                    RelicPrototype::GoldenPearl => {
                                        cursed != RelicPrototype::CursedPearl
                                    }
                                    RelicPrototype::ArcaneScroll => {
                                        cursed != RelicPrototype::HeftyTablet
                                    }
                                    RelicPrototype::NewLeaf => {
                                        cursed != RelicPrototype::LeafyPoultice
                                    }
                                    RelicPrototype::PreciseScissors => {
                                        cursed != RelicPrototype::PrecariousShears
                                    }
                                    RelicPrototype::LavaRock | RelicPrototype::SmallCapsule => {
                                        cursed != RelicPrototype::LargeCapsule
                                    }

                                    _ => true,
                                })
                                .copied(),
                        )
                    })
                    .map(|((cursed, pos1), pos2)| [cursed, pos1, pos2])
                    .map(|options| Self::Ancient {
                        options: options.map(|relic| RelicPickupOption { relic }),
                    }),
            ),

            e => todo!("{:?}", e),
        }
    }

    pub fn legal_actions(&self) -> impl Iterator<Item = EventAction> {
        match self {
            Event::Ancient { options } => (0..3).map(|idx| EventAction::Option { index: idx }),
        }
    }

    pub fn apply_action<Family: DistributionFamily>(
        mut state: RunState,
        action: EventAction,
    ) -> Family::Distribution<RunState> {
        // TODO: can I avoid this clone?
        let Some(SubState::Event { event }) = state.sub_state.clone() else {
            unreachable!()
        };

        match (event, action) {
            (Event::Ancient { options }, EventAction::Option { index }) => {
                state.sub_state = Some(SubState::RewardScreen {
                    card_rewards: vec![],
                    money_rewards: vec![],
                    relic_rewards: vec![RelicReward {
                        relic: options[index].relic,
                    }],
                    potions_rewards: vec![],
                });

                Family::Distribution::single_value(state)
            }
        }
    }
}
