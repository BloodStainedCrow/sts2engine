use std::{iter, usize};

use itertools::Either;
use rand::{rng, seq::IteratorRandom};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    combat_action::CombatAction,
    combat_state::{
        CombatState, RunInfo,
        cards::{Card, CardPrototype, Rarity},
        encounter::EncounterPrototype,
        relics::{FullRelicState, RelicPrototype},
    },
    distribution::{self, Distribution, DistributionFamily},
    run_state::{
        card_rarity_odds::CardRarityOdds,
        encounter_generation::{
            EncounterGenerationKind, EncounterGenerationOptions, EncounterGenerationState,
        },
        event_generation::EventGenerationState,
        events::{Event, EventAction, EventLocationInfo},
        rewards::{CardReward, RelicReward, RewardAction},
    },
};

mod card_rarity_odds;
pub mod encounter_generation;
mod event_generation;
mod events;
mod mcts;
mod rewards;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunState {
    player_hp: u16,
    player_max_hp: u16,
    player_relics: FullRelicState,
    card_reward_pity_state: CardRarityOdds,

    current_act: Act,
    current_position: ActPosition,

    sub_state: Option<SubState>,

    deck: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SubState {
    Event {
        event: Event,
    },
    Shop,
    Combat {
        combat: CombatState,
    },
    RewardScreen {
        card_rewards: Vec<CardReward>,
        money_rewards: Vec<!>,
        relic_rewards: Vec<RelicReward>,
        potions_rewards: Vec<!>,
    },
}

impl RunState {
    // TODO: character, maybe ascension if I do not want to do that via feature flags
    fn start_run<Family: DistributionFamily>(map: Map) -> Family::Distribution<Self> {
        let starting_acts = ActPrototype::iter()
            .filter(|act| act.act_number() == 0)
            // TODO: Debug
            .filter(|&act| act != ActPrototype::Underdocks);

        Family::Distribution::<ActPrototype>::equal_chance(starting_acts).cartesian_product(
            Family::Distribution<Map>::single_value(map),
            |act, map| Self {
                player_hp: 70,
                player_max_hp: 70,
                player_relics: [(RelicPrototype::RingOfTheSnake, 0)].into_iter().collect(),
                card_reward_pity_state: CardRarityOdds::default(),

                current_act: Act {
                    prototype: act,
                    map,
                    encounter_generation_state: EncounterGenerationState::default(),
                    event_generation_state: EventGenerationState::default(),
                },
                current_position: ActPosition { row: usize::MAX, column: 0 },
                sub_state: None,
                deck: vec![
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Neutralize.get_normal_card(),
                    CardPrototype::Survivor.get_normal_card(),
                ],
            },
        )
    }

    fn pickup_relic_from_reward<Family: DistributionFamily>(
        mut self,
        relic: RelicPrototype,
    ) -> Family::Distribution<Self> {
        use RelicPrototype::*;

        let Some(SubState::RewardScreen {
            card_rewards,
            money_rewards,
            relic_rewards,
            potions_rewards,
        }) = &mut self.sub_state
        else {
            unreachable!()
        };

        self.player_relics.set_state(relic, 0);

        // FIXME: Set starting number
        match relic {
            CursedPearl => {
                self.deck.push(CardPrototype::Greed.get_normal_card());
                // TODO:
                // self.gold += 333;
                Family::Distribution::single_value(self)
            }
            GoldenPearl => {
                // self.gold += 150;
                Family::Distribution::single_value(self)
            }
            NutritiousOyster => {
                self.player_max_hp += 11;
                Family::Distribution::single_value(self)
            }
            NeowsTalisman => {
                if let Some(card) = self
                    .deck
                    .iter_mut()
                    .find(|card| card.prototype == CardPrototype::Strike && !card.upgraded)
                {
                    card.upgraded = true;
                }
                if let Some(card) = self
                    .deck
                    .iter_mut()
                    .find(|card| card.prototype == CardPrototype::Defend && !card.upgraded)
                {
                    card.upgraded = true;
                }
                Family::Distribution::single_value(self)
            }
            HeftyTablet => {
                self.deck.push(CardPrototype::Injury.get_normal_card());

                CardReward::from_rarity::<Family>(Rarity::Rare).cartesian_product(
                    Family::Distribution::single_value(self),
                    |reward, mut state| {
                        let Some(SubState::RewardScreen { card_rewards, .. }) =
                            &mut state.sub_state
                        else {
                            unreachable!()
                        };
                        card_rewards.push(reward);

                        state
                    },
                )
            }
            _ => Distribution::single_value(self),
        }
    }

    fn start_combat<Family: DistributionFamily>(
        self,
        kind: EncounterGenerationKind,
    ) -> Family::Distribution<Self> {
        let next_encounter: Family::Distribution<(EncounterGenerationState, EncounterPrototype)> =
            self.current_act
                .encounter_generation_state
                .clone()
                .next_encounter::<Family>(EncounterGenerationOptions {
                    kind,
                    act: self.current_act.prototype,
                });

        Family::Distribution::single_value(self)
            .cartesian_product(
                next_encounter,
                |mut slf: Self, (new_state, next_encounter)| {
                    slf.current_act.encounter_generation_state = new_state;
                    let combats = CombatState::get_starting_states(
                        next_encounter,
                        &RunInfo {
                            hp: slf.player_hp,
                            max_hp: slf.player_max_hp,
                            deck: slf.deck.as_slice(),
                            relic_state: &slf.player_relics,
                        },
                        |_| true,
                    );

                    Family::Distribution::single_value(slf)
                        .cartesian_product(combats, |mut slf: Self, combat| {
                            slf.sub_state = Some(SubState::Combat { combat });

                            Family::Distribution::single_value(slf)
                        })
                        .flatten::<Self>()
                },
            )
            .flatten::<Self>()
    }

    fn start_event<Family: DistributionFamily>(
        self,
        kind: EventLocationInfo,
    ) -> Family::Distribution<Self> {
        let next_event: Family::Distribution<(_, _)> = self
            .current_act
            .event_generation_state
            .clone()
            .next_event::<Family>(&self.current_act, kind);

        Family::Distribution::single_value(self)
            .cartesian_product(next_event, |mut slf: Self, (new_state, next_event)| {
                slf.current_act.event_generation_state = new_state;
                let event_states = Event::from_prototype::<Family>(next_event);

                Family::Distribution::single_value(slf)
                    .cartesian_product(event_states, |mut slf: Self, event| {
                        slf.sub_state = Some(SubState::Event { event });

                        Distribution::single_value(slf)
                    })
                    .flatten::<Self>()
            })
            .flatten::<Self>()
    }

    fn apply_action<Family: DistributionFamily>(
        mut self,
        action: RunAction,
    ) -> Family::Distribution<Self> {
        match action {
            RunAction::ChooseNode { column } => {
                assert!(
                    self.sub_state.is_none(),
                    "Tried to choose node while not in map"
                );

                let next_node = &self.current_act.map.layers[self.current_position.row + 1][column];

                self.current_position = ActPosition {
                    row: self.current_position.row + 1,
                    column,
                };

                match next_node.kind {
                    NodeKind::Ancient => self.start_event(EventLocationInfo::Ancient),
                    NodeKind::Unknown => {
                        // TODO: Other types of nodes behind questionmarks
                        self.start_event(EventLocationInfo::NormalQuestionmark)
                    }
                    NodeKind::Merchant => todo!(),
                    NodeKind::Treasure => todo!(),
                    NodeKind::Rest => todo!(),

                    NodeKind::Enemy => self.start_combat(EncounterGenerationKind::Hallway),
                    NodeKind::Elite => self.start_combat(EncounterGenerationKind::Elite),
                    NodeKind::Boss => self.start_combat(EncounterGenerationKind::Boss),
                }
            }
            RunAction::EventAction { action } => Event::apply_action(self, action),
            RunAction::ShopAction { action } => todo!(),
            RunAction::RestSiteAction { action } => todo!(),
            RunAction::CombatAction { action } => {
                Family::Distribution::single_value(self).flat_map_simple(|state: Self| {
                    // FIXME: I hate this clone
                    let Some(SubState::Combat { combat }) = state.sub_state.clone() else {
                        unreachable!("Tried using combat action when not in combat");
                    };

                    combat.apply::<Family>(action).cartesian_product(
                        Family::Distribution::single_value(state),
                        |combat, mut state: Self| {
                            if let Some(post_combat) = combat.get_post_game_state() {
                                if post_combat.victory {
                                    // todo!("Apply post combat state to main state and continue: {:?}", post_combat);

                                    state.player_hp = post_combat.hp;
                                    state.player_max_hp = post_combat.max_hp;
                                    state.player_relics = post_combat.relic_state;

                                    state.sub_state = Some(SubState::RewardScreen {
                                        card_rewards: (0..(post_combat.bonus_card_rewards + 1))
                                            .map(|_| {
                                                CardReward {
                                                    // FIXME: This is all kinds of wrong.
                                                    // Use distribution, and pity!
                                                    options: vec![
                                                        CardPrototype::iter()
                                                            .choose(&mut rng())
                                                            .unwrap()
                                                            .get_normal_card(),
                                                        CardPrototype::iter()
                                                            .choose(&mut rng())
                                                            .unwrap()
                                                            .get_normal_card(),
                                                        CardPrototype::iter()
                                                            .choose(&mut rng())
                                                            .unwrap()
                                                            .get_normal_card(),
                                                    ],
                                                }
                                            })
                                            .collect(),
                                        money_rewards: vec![],
                                        relic_rewards: vec![],
                                        potions_rewards: vec![],
                                    });
                                } else {
                                    // We died in this combat. Stay here and the eval will return a value
                                    state.sub_state = Some(SubState::Combat { combat });
                                }
                            } else {
                                state.sub_state = Some(SubState::Combat { combat });
                            }

                            state
                        },
                    )
                })
            }
            RunAction::RewardAction { action } => {
                let Some(SubState::RewardScreen {
                    card_rewards,
                    money_rewards,
                    relic_rewards,
                    potions_rewards,
                }) = &mut self.sub_state
                else {
                    unreachable!("Tried using reward action when not in reward screen");
                };

                match action {
                    RewardAction::Continue {} => {
                        self.sub_state = None;

                        Distribution::single_value(self)
                    }

                    RewardAction::Card { reward, card } => {
                        assert!(card_rewards[reward].options.contains(&card));

                        card_rewards.remove(reward);

                        self.deck.push(card);

                        Distribution::single_value(self)
                    }

                    RewardAction::Relic { reward } => {
                        let relic = relic_rewards.remove(reward);

                        self.pickup_relic_from_reward(relic.relic)
                    }
                }

                // Triggers like book of five rings
            }
        }
    }

    fn legal_actions(&self) -> impl Iterator<Item = RunAction> {
        match &self.sub_state {
            Some(SubState::Combat { combat }) => Either::Left(Either::Left(
                combat
                    .legal_actions()
                    .map(|action| RunAction::CombatAction { action }),
            )),
            Some(SubState::Event { event }) => Either::Left(Either::Right(
                event
                    .legal_actions()
                    .map(|action| RunAction::EventAction { action }),
            )),
            Some(SubState::Shop {}) => todo!(),
            Some(SubState::RewardScreen {
                card_rewards,
                money_rewards,
                relic_rewards,
                potions_rewards,
            }) => Either::Right(Either::Left(
                iter::once(RunAction::RewardAction {
                    action: RewardAction::Continue {},
                })
                .chain(card_rewards.iter().enumerate().flat_map(
                    |(reward, cards)| {
                        cards
                            .options
                            .iter()
                            .copied()
                            .map(move |card| RunAction::RewardAction {
                                action: RewardAction::Card { reward, card },
                            })
                    },
                )),
            )),
            None => {
                // TODO:
                // if self.relics.contains(WingedBoots)
                if false {
                    todo!()
                } else {
                    Either::Right(Either::Right(
                        self.current_act.map.layers[self.current_position.row + 1]
                            .iter()
                            .enumerate()
                            .filter_map(|(column, node)| {
                                node.parents
                                    .contains(&self.current_position.column)
                                    .then_some(column)
                            })
                            .map(|idx| RunAction::ChooseNode { column: idx }),
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ActPosition {
    row: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunAction {
    ChooseNode { column: usize },
    EventAction { action: EventAction },
    ShopAction { action: ! },
    RestSiteAction { action: ! },
    CombatAction { action: CombatAction },
    RewardAction { action: RewardAction },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Act {
    prototype: ActPrototype,

    map: Map,
    encounter_generation_state: EncounterGenerationState,
    event_generation_state: EventGenerationState,
}

impl Act {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum ActPrototype {
    Overgrowth,
    Underdocks,
    Hive,
    Glory,
}

impl ActPrototype {
    fn act_number(self) -> usize {
        match self {
            ActPrototype::Overgrowth => 0,
            ActPrototype::Underdocks => 0,
            ActPrototype::Hive => 1,
            ActPrototype::Glory => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Map {
    layers: Vec<Vec<Node>>,
}

impl Default for Map {
    fn default() -> Self {
        let mut layers = vec![
            vec![Node {
                parents: vec![0],
                kind: NodeKind::Enemy
            }];
            14
        ];

        layers.insert(
            0,
            vec![Node {
                parents: vec![0],
                kind: NodeKind::Ancient,
            }],
        );

        Self { layers }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    // The Node indices of the next layer
    parents: Vec<usize>,

    kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeKind {
    Ancient,
    Unknown,
    Merchant,
    Treasure,
    Rest,
    Enemy,
    Elite,
    Boss,
}
