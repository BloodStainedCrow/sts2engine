use std::cmp::max;
use std::os::linux::raw::stat;

use bumpalo::boxed::Box;
use bumpalo::collections::CollectIn;
use bumpalo::vec;
use bumpalo::{Bump, collections::Vec};
use enum_map::{Enum, EnumMap};
use itertools::Itertools;
use std::hash::Hash;

use crate::game_state::cards::{
    Card, CardKind, CardPrototype, Cost, CostVal, LegalTarget, UnorderedCardSet,
};
use crate::game_state::relics::{FullRelicState, RelicPrototype};
use crate::{combat_action::CombatAction, distribution::Distribution};

pub(crate) mod cards;
pub(crate) mod relics;

struct RunState {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatState<'bump> {
    pub turn_counter: u8,

    pub player: Player<'bump>,

    pub enemies: Vec<'bump, Enemy>,

    pub relic_state: FullRelicState,
}

impl Hash for CombatState<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // TODO: The turn counter does not matter (for now)
        // TODO: Test if that increases the transposition hit rate
        self.turn_counter.hash(state);
        self.player.hash(state);
        self.enemies.hash(state);
        self.relic_state.hash(state);
    }
}

impl CombatState<'_> {
    pub fn launder(self, new_bump: &Bump) -> CombatState<'_> {
        CombatState {
            turn_counter: self.turn_counter,
            player: self.player.launder(new_bump),

            enemies: self.enemies.into_iter().collect_in(new_bump),

            relic_state: self.relic_state,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PostCombatState {
    pub turn_counter: u8,

    pub hp: u16,
    pub max_hp: u16,
    // I use an array of bools here to make this cheaper to clone
    pub potions_used: [bool; 10],
    // TODO
    // lost_card: Option<Card>,
    pub bonus_card_rewards: u8,

    pub relic_state: FullRelicState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CharacterIndex {
    Player,
    Enemy(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum EncounterPrototype {
    FuzzyWurmCrawler,
    SingleNibbit,
    DoubleNibbit,
    SlimesWeak,
    ShrinkerBeetle,
    Byrdonis,
    BygoneEffigy,
    SingleCubexConstruct,
    BeetleAndFuzzy,
    RubyRaiders,
    Vantom,
    BowlbugsWeak,
    BowlbugsStrong,
    SoloTunneler,
    InfestedPrism,
    // Mr. Beeeees!!!
    Entomancer,
    Chompers,
    SlumberParty,
}

impl EncounterPrototype {
    #[allow(clippy::match_same_arms)]
    fn is_elite(self) -> bool {
        match self {
            EncounterPrototype::FuzzyWurmCrawler => false,
            EncounterPrototype::SingleNibbit => false,
            EncounterPrototype::DoubleNibbit => false,
            EncounterPrototype::SlimesWeak => false,
            EncounterPrototype::ShrinkerBeetle => false,
            EncounterPrototype::Byrdonis => true,
            EncounterPrototype::BygoneEffigy => true,
            EncounterPrototype::SingleCubexConstruct => false,
            EncounterPrototype::BeetleAndFuzzy => false,
            EncounterPrototype::RubyRaiders => false,
            EncounterPrototype::Vantom => false,
            EncounterPrototype::BowlbugsWeak => false,
            EncounterPrototype::BowlbugsStrong => false,
            EncounterPrototype::SoloTunneler => false,
            EncounterPrototype::InfestedPrism => true,
            EncounterPrototype::Entomancer => true,
            EncounterPrototype::Chompers => false,
            EncounterPrototype::SlumberParty => false,
        }
    }
}

// TODO:
pub struct RunInfo<'bump> {
    pub hp: u16,
    pub deck: Vec<'bump, Card>,

    pub relic_state: FullRelicState,
}

impl<'bump> CombatState<'bump> {
    pub(crate) fn get_starting_states(
        encounter: EncounterPrototype,
        run_info: &RunInfo,

        enemy_max_hp_filter: impl Fn(&[u16]) -> bool,

        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        let state = Distribution::single_value(
            Self {
                turn_counter: 0,
                player: Player {
                    hand: vec![in bump;].into_iter().collect_in(bump),
                    draw_pile: run_info.deck.clone().into_iter().collect_in(bump),
                    draw_pile_top_card: None,
                    discard_pile: vec![in bump;].into_iter().collect_in(bump),
                    exhaust_pile: vec![in bump;].into_iter().collect_in(bump),
                    play_pile: vec![in bump;].into_iter().collect_in(bump),
                    waiting_for_decision: None,
                    orbs: vec![in bump;],
                    num_orb_slots: 1,
                    energy: 0,
                    stars: 0,
                    creature: Creature {
                        hp: run_info.hp,
                        max_hp: 70,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                },
                enemies: vec![in bump;],

                relic_state: run_info.relic_state,
            },
            bump,
        );

        assert!(state.entries.iter().map(|(v, _)| v).all_unique());

        let mut state_with_enemy = match encounter {
            EncounterPrototype::FuzzyWurmCrawler => {
                let hp = 55..=57;

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|hp| {
                                let mut state = state.clone();

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::FuzzyWurmCrawler,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::SingleNibbit => {
                let hp = 42..=46;

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|hp| {
                                let mut state = state.clone();

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::Nibbit,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::DoubleNibbit => {
                let hps = (42..=46).cartesian_product(42..=46);

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hps.clone().map(|(first_hp, second_hp)| {
                                let mut state = state.clone();

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::Nibbit,
                                    creature: Creature {
                                        hp: first_hp,
                                        max_hp: first_hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::Nibbit,
                                    creature: Creature {
                                        hp: second_hp,
                                        max_hp: second_hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::SlimesWeak => todo!(),
            EncounterPrototype::ShrinkerBeetle => {
                let hp = 38..=40;

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|hp| {
                                let mut state = state.clone();

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::ShrinkerBeetle,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::Byrdonis => {
                let hp = 91..=94;

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|hp| {
                                let mut state = state.clone();

                                let mut status = EnumMap::default();

                                status[Status::Territorial] = 1;

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::Byrdonis,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: status,
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::BygoneEffigy => {
                let hp = 127..=127;

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|hp| {
                                let mut state = state.clone();

                                let mut status = EnumMap::default();

                                // TODO:
                                // status[Status::Slow] = 1;

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::BygoneEffigy,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: status,
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::SingleCubexConstruct => {
                let hp = 65..=65;

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|hp| {
                                let mut state = state.clone();

                                let mut status = EnumMap::default();

                                status[Status::Artifact] = 1;

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::CubexConstruct,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: status,
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::BeetleAndFuzzy => {
                let hp = (38..=40).cartesian_product(55..=57);

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|(beetle_hp, fuzzy_hp)| {
                                let mut state = state.clone();

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::ShrinkerBeetle,
                                    creature: Creature {
                                        hp: beetle_hp,
                                        max_hp: beetle_hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::FuzzyWurmCrawler,
                                    creature: Creature {
                                        hp: fuzzy_hp,
                                        max_hp: fuzzy_hp,
                                        block: 0,
                                        statuses: EnumMap::default(),
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine::default(),
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::RubyRaiders => {
                let typ = (0..5)
                    .cartesian_product(0..5)
                    .cartesian_product(0..5)
                    .map(|((a, b), c)| [a, b, c])
                    .filter(|v| v.iter().all_unique());

                let typ_and_hp_range = typ.map(|typs| {
                    typs.map(|ty| match ty {
                        0 => (EnemyPrototype::AxeRubyRaider, 20..=22),
                        1 => (EnemyPrototype::AssassinRubyRaider, 18..=23),
                        2 => (EnemyPrototype::BruteRubyRaider, 30..=33),
                        3 => (EnemyPrototype::CrossbowRubyRaider, 18..=21),
                        4 => (EnemyPrototype::TrackerRubyRaider, 21..=25),

                        _ => unreachable!(),
                    })
                });

                let typ_and_hp = typ_and_hp_range.flat_map(|[enemy_0, enemy_1, enemy_2]| {
                    enemy_0
                        .1
                        .cartesian_product(enemy_1.1)
                        .cartesian_product(enemy_2.1)
                        .map(move |((a, b), c)| [(enemy_0.0, a), (enemy_1.0, b), (enemy_2.0, c)])
                });

                dbg!(state.len());
                dbg!(typ_and_hp.clone().count());

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            typ_and_hp.clone().map(|enemies| {
                                let mut state = state.clone();

                                for (enemy, hp) in enemies {
                                    state.enemies.push(Enemy {
                                        prototype: enemy,
                                        creature: Creature {
                                            hp,
                                            max_hp: hp,
                                            block: 0,
                                            statuses: EnumMap::default(),
                                        },
                                        has_acted_this_turn: false,
                                        state_machine: EnemyStateMachine::default(),
                                        has_taken_unblocked_damage_this_turn: false,
                                    });
                                }

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::Vantom => state.map(
                |mut state| {
                    let mut status = EnumMap::default();

                    status[Status::Slippery] = 9;

                    state.enemies.push(Enemy {
                        prototype: EnemyPrototype::Vantom,
                        creature: Creature {
                            hp: 173,
                            max_hp: 173,
                            block: 0,
                            statuses: status,
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_damage_this_turn: false,
                    });

                    state
                },
                bump,
            ),
            EncounterPrototype::BowlbugsWeak => {
                let typ = (0..=0).cartesian_product(1..=2).map(|(a, b)| [a, b]);

                let typ_and_hp_range = typ.map(|typs| {
                    typs.map(|ty| match ty {
                        0 => (EnemyPrototype::BowlbugRock, 45..=48),
                        1 => (EnemyPrototype::BowlbugEgg, 21..=22),
                        2 => (EnemyPrototype::BowlbugNectar, 35..=38),

                        _ => unreachable!(),
                    })
                });

                let typ_and_hp = typ_and_hp_range.flat_map(|[enemy_0, enemy_1]| {
                    enemy_0
                        .1
                        .cartesian_product(enemy_1.1)
                        .map(move |(a, b)| [(enemy_0.0, a), (enemy_1.0, b)])
                });

                dbg!(state.len());
                dbg!(typ_and_hp.clone().count());

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            typ_and_hp.clone().map(|enemies| {
                                let mut state = state.clone();

                                let mut unbalanced = EnumMap::default();
                                unbalanced[Status::Imbalanced] = 1;

                                for (enemy, hp) in enemies {
                                    state.enemies.push(Enemy {
                                        prototype: enemy,
                                        creature: Creature {
                                            hp,
                                            max_hp: hp,
                                            block: 0,
                                            statuses: if enemy == EnemyPrototype::BowlbugRock {
                                                unbalanced
                                            } else {
                                                EnumMap::default()
                                            },
                                        },
                                        has_acted_this_turn: false,
                                        state_machine: EnemyStateMachine::default(),
                                        has_taken_unblocked_damage_this_turn: false,
                                    });
                                }

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::BowlbugsStrong => {
                let typ = (0..=0)
                    .cartesian_product(1..=3)
                    .cartesian_product(1..=3)
                    .map(|((a, b), c)| [a, b, c])
                    .filter(|[_a, b, c]| b != c);

                let typ_and_hp_range = typ.map(|typs| {
                    typs.map(|ty| match ty {
                        0 => (EnemyPrototype::BowlbugRock, 45..=48),
                        1 => (EnemyPrototype::BowlbugEgg, 21..=22),
                        2 => (EnemyPrototype::BowlbugNectar, 35..=38),
                        3 => (EnemyPrototype::BowlbugSilk, 40..=43),

                        _ => unreachable!(),
                    })
                });

                let typ_and_hp = typ_and_hp_range.flat_map(|[enemy_0, enemy_1, enemy_2]| {
                    enemy_0
                        .1
                        .cartesian_product(enemy_1.1)
                        .cartesian_product(enemy_2.1)
                        .map(move |((a, b), c)| [(enemy_0.0, a), (enemy_1.0, b), (enemy_2.0, c)])
                });

                dbg!(state.len());
                dbg!(typ_and_hp.clone().count());

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            typ_and_hp.clone().map(|enemies| {
                                let mut state = state.clone();

                                let mut unbalanced = EnumMap::default();
                                unbalanced[Status::Imbalanced] = 1;

                                for (enemy, hp) in enemies {
                                    state.enemies.push(Enemy {
                                        prototype: enemy,
                                        creature: Creature {
                                            hp,
                                            max_hp: hp,
                                            block: 0,
                                            statuses: if enemy == EnemyPrototype::BowlbugRock {
                                                unbalanced
                                            } else {
                                                EnumMap::default()
                                            },
                                        },
                                        has_acted_this_turn: false,
                                        state_machine: EnemyStateMachine::default(),
                                        has_taken_unblocked_damage_this_turn: false,
                                    });
                                }

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::SoloTunneler => {
                todo!("Adaptive state machine intent logic (check if we lost all block")
            }
            EncounterPrototype::InfestedPrism => state.map(
                |mut state| {
                    let mut status = EnumMap::default();

                    status[Status::VitalSpark] = 1;

                    state.enemies.push(Enemy {
                        prototype: EnemyPrototype::InfestedPrism,
                        creature: Creature {
                            hp: 200,
                            max_hp: 200,
                            block: 0,
                            statuses: status,
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_damage_this_turn: false,
                    });

                    state
                },
                bump,
            ),
            EncounterPrototype::Entomancer => state.map(
                |mut state| {
                    let mut status = EnumMap::default();

                    status[Status::PersonalHive] = 1;

                    state.enemies.push(Enemy {
                        prototype: EnemyPrototype::Entomancer,
                        creature: Creature {
                            hp: 145,
                            max_hp: 145,
                            block: 0,
                            statuses: status,
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_damage_this_turn: false,
                    });

                    state
                },
                bump,
            ),
            EncounterPrototype::Chompers => {
                let hp = (60..=64).cartesian_product(60..=64);

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|(first, second)| {
                                let mut state = state.clone();

                                let mut status = EnumMap::default();

                                status[Status::Artifact] = 2;

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::Chomper,
                                    creature: Creature {
                                        hp: first,
                                        max_hp: first,
                                        block: 0,
                                        statuses: status,
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine {
                                        current_state: 0,
                                        ..Default::default()
                                    },
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state.enemies.push(Enemy {
                                    prototype: EnemyPrototype::Chomper,
                                    creature: Creature {
                                        hp: second,
                                        max_hp: second,
                                        block: 0,
                                        statuses: status,
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine {
                                        current_state: 1,
                                        ..Default::default()
                                    },
                                    has_taken_unblocked_damage_this_turn: false,
                                });

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
            EncounterPrototype::SlumberParty => {
                let hp = (45..=58)
                    .cartesian_product(40..=43)
                    .cartesian_product(86..=86)
                    .map(|((a, b), c)| [a, b, c]);

                let state = state.flat_map(
                    |state, bump| {
                        Distribution::equal_chance(
                            hp.clone().map(|enemies| {
                                let mut state = state.clone();

                                let mut unbalanced = EnumMap::default();
                                unbalanced[Status::Imbalanced] = 1;

                                let mut slumbering = EnumMap::default();
                                slumbering[Status::Slumber] = 3;
                                slumbering[Status::Plating] = 15;

                                for (enemy, hp) in enemies.into_iter().enumerate() {
                                    let enemy = match enemy {
                                        0 => EnemyPrototype::BowlbugRock,
                                        1 => EnemyPrototype::BowlbugSilk,
                                        2 => EnemyPrototype::SlumberingBeetle,
                                        _ => unreachable!(),
                                    };

                                    state.enemies.push(Enemy {
                                        prototype: enemy,
                                        creature: Creature {
                                            hp,
                                            max_hp: hp,
                                            block: if enemy == EnemyPrototype::SlumberingBeetle {
                                                15
                                            } else {
                                                0
                                            },
                                            statuses: if enemy == EnemyPrototype::BowlbugRock {
                                                unbalanced
                                            } else if enemy == EnemyPrototype::SlumberingBeetle {
                                                slumbering
                                            } else {
                                                EnumMap::default()
                                            },
                                        },
                                        has_acted_this_turn: false,
                                        state_machine: EnemyStateMachine::default(),
                                        has_taken_unblocked_damage_this_turn: false,
                                    });
                                }

                                state
                            }),
                            bump,
                        )
                    },
                    bump,
                );

                state
            }
        };

        state_with_enemy.retain_no_chance_fix(|state| {
            (enemy_max_hp_filter)(
                &state
                    .enemies
                    .iter()
                    .map(|enemy| enemy.creature.max_hp)
                    .collect_vec(),
            )
        });
        let state = state_with_enemy.fix_odds();

        assert!(state.entries.iter().map(|(v, _)| v).all_unique());

        let mut state = state.flat_map(Self::on_start_player_turn, bump);

        let mut state = if encounter.is_elite()
            && run_info.relic_state.contains(RelicPrototype::BoomingConch)
        {
            for _ in 0..2 {
                state = state.flat_map(CombatState::draw_single_card, bump);
            }
            state
        } else {
            state
        };

        dbg!(state.len());
        state.dedup();
        dbg!(state.len());

        if run_info
            .relic_state
            .contains(RelicPrototype::OddlySmoothStone)
        {
            state = state.flat_map(
                |state, bump| {
                    state.apply_status_change(CharacterIndex::Player, Status::Dexterity, 1, bump)
                },
                bump,
            );
        }

        if run_info.relic_state.contains(RelicPrototype::Gorget) {
            state = state.flat_map(
                |state, bump| {
                    state.apply_status_change(CharacterIndex::Player, Status::Plating, 4, bump)
                },
                bump,
            );
        }

        if run_info.relic_state.contains(RelicPrototype::Vajra) {
            state = state.flat_map(
                |state, bump| {
                    state.apply_status_change(CharacterIndex::Player, Status::Strength, 1, bump)
                },
                bump,
            );
        }

        // assert!(state.entries.iter().map(|(v, _)| v).all_unique());
        assert!(!state.entries.is_empty());

        dbg!(state.len());
        state.dedup();
        dbg!(state.len());

        // assert!(state.entries.iter().map(|(v, _)| v).all_unique());

        state.dedup();
        assert!(!state.entries.is_empty());

        state
    }

    pub(crate) fn get_post_game_state(&self) -> Option<PostCombatState> {
        if self.enemies.is_empty() {
            Some(PostCombatState {
                turn_counter: self.turn_counter,

                hp: self.player.creature.hp,
                max_hp: self.player.creature.max_hp,
                potions_used: [false; 10],
                bonus_card_rewards: 0,

                relic_state: self.relic_state,
            })
        } else if self.player.creature.hp == 0 {
            // } else if self.player.is_dead() {
            Some(PostCombatState {
                turn_counter: self.turn_counter,

                hp: 0,
                max_hp: self.player.creature.max_hp,
                potions_used: [false; 10],
                bonus_card_rewards: 0,

                relic_state: self.relic_state,
            })
        } else {
            None
        }
    }

    pub(crate) fn legal_actions(
        &self,
        bump: &'bump Bump,
    ) -> impl Iterator<Item = CombatAction> + use<'_> {
        if let Some(required) = &self.player.waiting_for_decision {
            match required {
                RequiredPlayerDecision::ChooseCardInHand { filter, action: _ } => {
                    return self
                        .player
                        .hand
                        .iter()
                        .enumerate()
                        .filter(|(_, card)| (filter)(**card))
                        .map(|(index, card)| CombatAction::Choice { card: *card })
                        .collect_in::<Vec<_>>(bump)
                        .into_iter();
                }
            }
        }

        // TODO: Potions
        self.player
            .hand
            .iter()
            .enumerate()
            .filter(|(_, card)| !card.has_unplayable())
            .filter(|(_, card)| {
                let cost = card.get_cost();
                match (cost.energy, cost.stars) {
                    (CostVal::X, CostVal::X) => todo!(),
                    (CostVal::X, CostVal::Val(cost)) => self.player.stars >= cost,
                    (CostVal::Val(cost), CostVal::X) => self.player.energy >= cost,
                    (CostVal::Val(energy), CostVal::Val(stars)) => {
                        self.player.energy >= energy && self.player.stars >= stars
                    }
                }
            })
            .flat_map(move |(card_index, card)| {
                card.get_legal_targets()
                    .flat_map(move |target| match target {
                        LegalTarget::OwnPlayer => vec![in bump; CombatAction::PlayCard {
                            card: *card,
                            target: None,
                        }],
                        LegalTarget::OtherPlayer => todo!(),
                        LegalTarget::Enemy => self
                            .enemies
                            .iter()
                            .enumerate()
                            .map(|(enemy_index, enemy)| CombatAction::PlayCard {
                                card: *card,
                                target: Some(enemy_index as u8),
                            })
                            .collect_in(bump),
                    })
            })
            .chain(std::iter::repeat_n(
                CombatAction::EndTurn,
                // TODO: This is technically not correct. But it should drastically increase the speed of the engine
                // Only allow ending turn when no cards can be played
                usize::from(
                    !self
                        .player
                        .hand
                        .iter()
                        .filter(|card| !card.has_unplayable())
                        .any(|card| {
                            let cost = card.get_cost();
                            let can_afford = match (cost.energy, cost.stars) {
                                (CostVal::X, CostVal::X) => todo!(),
                                (CostVal::X, CostVal::Val(cost)) => self.player.stars >= cost,
                                (CostVal::Val(cost), CostVal::X) => self.player.energy >= cost,
                                (CostVal::Val(energy), CostVal::Val(stars)) => {
                                    self.player.energy >= energy && self.player.stars >= stars
                                }
                            };

                            // TODO: Ignore exhausting cards here, to allow not playing those when not needed
                            can_afford
                        }),
                ),
            ))
            .collect_in::<Vec<_>>(bump)
            .into_iter()
    }

    pub(crate) fn apply(
        &self,
        action: CombatAction,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        match action {
            CombatAction::PlayCard { card, target } => {
                let mut result = self.clone();

                result.player.hand.remove_card(card);

                // FIXME: state effects on cost
                let cost = card.get_cost();

                let result = result.pay_cost(cost, bump);
                // let result = Distribution::single_value(result);

                result.flat_map(
                    |state, bump| state.play_card(card, target.map(Into::into), true, bump),
                    bump,
                )
            }
            CombatAction::UsePotion { index } => todo!(),
            CombatAction::Choice { card } => {
                let mut state = self.clone();
                match state
                    .player
                    .waiting_for_decision
                    .take()
                    .expect("CombatAction::Choice is only valid if we have a pending choice")
                {
                    RequiredPlayerDecision::ChooseCardInHand { filter: _, action } => {
                        return (action)(Distribution::single_value(state, bump), bump, card);
                    }
                }
            }
            CombatAction::EndTurn => {
                let result = self.clone();

                result.handle_turn_transitions(bump)
            }
        }
    }

    fn pay_cost(mut self, cost: Cost, bump: &'bump Bump) -> Distribution<'bump, Self> {
        match cost.energy {
            CostVal::X => todo!(),
            CostVal::Val(cost) => {
                assert!(self.player.energy >= cost);
                self.player.energy -= cost;
            }
        }

        match cost.stars {
            CostVal::X => todo!(),
            CostVal::Val(cost) => {
                assert!(self.player.stars >= cost);
                self.player.stars -= cost;
            }
        }

        Distribution::single_value(self, bump)
    }

    fn draw_cards_for_turn(self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        // // TODO:
        // let mut num_cards = 5;

        // let mut res = Distribution::single_value(self, bump);

        // for _ in 0..num_cards {
        //     assert!(!res.entries.is_empty());
        //     res = res.flat_map(Self::draw_single_card, bump);
        // }

        // // This will produce lots of duplicated entries. Do reduce future work we dedup immediately
        // res.dedup();

        // res
        let res = self.draw_five_cards(bump);

        assert!(res.len() > 0);
        res
    }

    fn draw_single_card(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        let state = if let Some(top_card) = self.player.draw_pile_top_card.take() {
            self.player.hand.add_card(top_card);
            Distribution::single_value(self, bump)
        } else if self.player.draw_pile.is_empty() {
            if self.player.discard_pile.is_empty() {
                // Nothing to shuffle nor draw
                return Distribution::single_value(self, bump);
            }

            // Shuffle the discard pile into the draw pile
            // TODO: Triggers

            self.player.draw_pile.append(&mut self.player.discard_pile);

            self.draw_single_card(bump)
        } else {
            let cards = self.player.draw_pile.iter_counts().sorted_by_key(|v| v.0);

            Distribution::from_duplicates(
                cards.into_iter().map(|(card, count)| {
                    let mut new = self.clone();
                    new.player.draw_pile.remove_card(*card);
                    new.player.hand.add_card(*card);
                    (new, usize::from(count))
                }),
                bump,
            )
        };

        assert!(!state.entries.is_empty());

        state.flat_map(Self::on_draw_card, bump)
    }

    fn shuffle_discard_pile(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        // TODO: Triggers

        self.player.draw_pile.append(&mut self.player.discard_pile);

        Distribution::single_value(self, bump)
    }

    fn draw_five_cards(self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        if self.player.draw_pile_top_card.is_some()
            || self.player.draw_pile.num_cards() + self.player.discard_pile.num_cards() < 5
        {
            // Just do the simple thing for now, to ensure we draw the top card
            let num_cards = 5;

            let mut res = Distribution::single_value(self, bump);

            for _ in 0..num_cards {
                assert!(!res.entries.is_empty());
                res = res.flat_map(Self::draw_single_card, bump);
            }

            // This will produce lots of duplicated entries. Do reduce future work we dedup immediately
            res.dedup();
            assert!(res.len() > 0);

            return res;
        }

        if self.player.draw_pile.num_cards() < 5 {
            let num_draw_pile_uniques = self.player.draw_pile.iter().count();
            let num_draw_pile_cards = self.player.draw_pile.num_cards();
            let num_discard_pile_uniques = self.player.discard_pile.iter().count();
            let num_discard_pile_cards = self.player.discard_pile.num_cards();

            assert!(num_discard_pile_uniques > 0);
            let indices = (0..max(num_draw_pile_uniques, num_discard_pile_uniques))
                .cartesian_product(0..max(num_draw_pile_uniques, num_discard_pile_uniques))
                .cartesian_product(0..max(num_draw_pile_uniques, num_discard_pile_uniques))
                .cartesian_product(0..max(num_draw_pile_uniques, num_discard_pile_uniques))
                .cartesian_product(0..max(num_draw_pile_uniques, num_discard_pile_uniques))
                .map(|((((a, b), c), d), e)| [a, b, c, d, e])
                // This filter will mean only a single order is considered!
                // FIXME: This needs to be adjusted to only be internal to each stack (draw/discard)
                // .filter(|[a, b, c, d, e]| a <= b && b <= c && c <= d && d <= e)
                .filter_map(|v| {
                    if v.iter()
                        .enumerate()
                        .filter_map(|(i, card_index)| {
                            (i < num_draw_pile_cards).then_some(*card_index)
                        })
                        .any(|card_index| card_index >= self.player.draw_pile.iter().count())
                    {
                        return None;
                    }

                    if v.iter()
                        .enumerate()
                        .filter_map(|(i, card_index)| {
                            (i >= num_draw_pile_cards).then_some(*card_index)
                        })
                        .any(|card_index| card_index >= self.player.discard_pile.iter().count())
                    {
                        return None;
                    }

                    for (index, (_card, count)) in self.player.draw_pile.iter_counts().enumerate() {
                        if usize::from(count)
                            < v.iter()
                                .enumerate()
                                .filter_map(|(i, card_index)| {
                                    (i < num_draw_pile_cards).then_some(*card_index)
                                })
                                .filter(|card_index| *card_index == index)
                                .count()
                        {
                            return None;
                        }
                    }

                    for (index, (_card, count)) in
                        self.player.discard_pile.iter_counts().enumerate()
                    {
                        if usize::from(count)
                            < v.iter()
                                .enumerate()
                                .filter_map(|(i, card_index)| {
                                    (i >= num_draw_pile_cards).then_some(*card_index)
                                })
                                .filter(|card_index| *card_index == index)
                                .count()
                        {
                            return None;
                        }
                    }

                    Some(
                        v.into_iter()
                            .enumerate()
                            .map(|(i, index)| {
                                if i < num_draw_pile_cards {
                                    self.player
                                        .draw_pile
                                        .iter()
                                        .nth(index)
                                        .copied()
                                        .expect("Checked before")
                                } else {
                                    self.player
                                        .discard_pile
                                        .iter()
                                        .nth(index)
                                        .copied()
                                        .expect("Checked before")
                                }
                            })
                            .collect_array::<5>()
                            .expect("Array map"),
                    )
                });

            let mut indices = indices.peekable();

            assert!(indices.peek().is_some());

            return Distribution::equal_chance(
                indices.map(|v| {
                    let mut state = Distribution::single_value(self.clone(), bump);

                    for (index, card) in v.into_iter().enumerate() {
                        if index == num_draw_pile_cards {
                            // Shuffle
                            state = state.flat_map(Self::shuffle_discard_pile, bump);
                        }
                        state = state
                            .flat_map(|state, bump| state.draw_specific_card(card, bump), bump);
                    }

                    state
                }),
                bump,
            )
            .flatten(bump);
        }

        let num_unique_cards = self.player.draw_pile.iter().count();
        assert!(num_unique_cards > 0);
        let indices = (0..num_unique_cards)
            .cartesian_product(0..num_unique_cards)
            .cartesian_product(0..num_unique_cards)
            .cartesian_product(0..num_unique_cards)
            .cartesian_product(0..num_unique_cards)
            .map(|((((a, b), c), d), e)| [a, b, c, d, e])
            // This filter will mean only a single order is considered!
            .filter(|[a, b, c, d, e]| a <= b && b <= c && c <= d && d <= e)
            .filter_map(|v @ [a, b, c, d, e]| {
                // dbg!(v);
                for (index, (_card, count)) in self.player.draw_pile.iter_counts().enumerate() {
                    if count
                        < u8::from(a == index)
                            + u8::from(b == index)
                            + u8::from(c == index)
                            + u8::from(d == index)
                            + u8::from(e == index)
                    {
                        return None;
                    }
                }
                Some(v.map(|index| {
                    self.player
                        .draw_pile
                        .iter()
                        .nth(index)
                        .copied()
                        .expect("Checked before")
                }))
            });

        Distribution::equal_chance(
            indices.map(|[a, b, c, d, e]| {
                let mut state = Distribution::single_value(self.clone(), bump);

                state = state.flat_map(|state, bump| state.draw_specific_card(a, bump), bump);
                state = state.flat_map(|state, bump| state.draw_specific_card(b, bump), bump);
                state = state.flat_map(|state, bump| state.draw_specific_card(c, bump), bump);
                state = state.flat_map(|state, bump| state.draw_specific_card(d, bump), bump);
                state = state.flat_map(|state, bump| state.draw_specific_card(e, bump), bump);

                state
            }),
            bump,
        )
        .flatten(bump)
    }

    fn handle_turn_transitions(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        let mut state = Distribution::single_value(self, bump);

        state = state.flat_map(Self::on_end_player_turn, bump);

        state = state.flat_map(Self::on_start_enemy_turn, bump);

        state = state.flat_map(Self::handle_enemy_actions, bump);

        state = state.flat_map(Self::on_end_enemy_turn, bump);

        state = state.flat_map(Self::on_start_player_turn, bump);

        state
    }

    fn on_end_player_turn(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        self.player.creature.block += u16::try_from(self.player.creature.statuses[Status::Plating])
            .expect("Plating cannot be negative");

        // TODO: Handle Etheral and Cards with "if in hand at end of turn"
        self.player.discard_pile.append(&mut self.player.hand);

        let mut status_diff: EnumMap<Status, i16> = EnumMap::default();
        for (status, count) in &mut self.player.creature.statuses {
            match status {
                Status::Vulnerable => decrease_non_neg(count),
                Status::Weak => decrease_non_neg(count),
                Status::CorrosiveWave => {
                    *count = 0;
                }
                Status::Frail => decrease_non_neg(count),
                Status::Territorial => {
                    status_diff[Status::Strength] += *count;
                }
                Status::Anticipate => {
                    status_diff[Status::Dexterity] -= *count;
                    *count = 0;
                }
                Status::Burst => {
                    *count = 0;
                }

                _ => {}
            }
        }

        for (v, count) in self
            .player
            .creature
            .statuses
            .values_mut()
            .zip(status_diff.into_values())
        {
            *v += count;
        }

        Distribution::single_value(self, bump)
    }

    fn on_start_enemy_turn(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        for enemy in &mut self.enemies {
            enemy.creature.block = 0;

            let mut poison_dmg = enemy.creature.statuses[Status::Poison];

            if poison_dmg > 0 && enemy.creature.statuses[Status::Slippery] > 0 {
                poison_dmg = 1;
                enemy.creature.statuses[Status::Slippery] -= 1;
            }

            enemy.creature.hp = enemy.creature.hp.saturating_sub_signed(poison_dmg);
            if poison_dmg > 0 {
                if enemy.creature.statuses[Status::Slumber] > 0 {
                    enemy.creature.statuses[Status::Slumber] -= 1;
                    if enemy.creature.statuses[Status::Slumber] == 0 {
                        enemy.creature.statuses[Status::Plating] = 0;
                        enemy.state_machine.stunned = 1;
                    }
                }
                if enemy.creature.statuses[Status::Asleep] > 0 {
                    enemy.creature.statuses[Status::Slumber] = 0;
                    enemy.creature.statuses[Status::Plating] = 0;
                    enemy.state_machine.stunned = 1;
                }
            }
            decrease_non_neg(&mut enemy.creature.statuses[Status::Poison]);

            enemy.creature.block += u16::try_from(enemy.creature.statuses[Status::Plating])
                .expect("Plating cannot be negative");
            decrease_non_neg(&mut enemy.creature.statuses[Status::Plating]);
        }

        self.enemies.retain(|enemy| enemy.creature.hp > 0);

        Distribution::single_value(self, bump)
    }

    fn handle_enemy_actions(self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        // Enemy actions
        let mut state = Distribution::single_value(self, bump);

        loop {
            let mut did_act = false;
            state = state.flat_map(
                |mut state, bump| {
                    let enemy = state
                        .enemies
                        .iter_mut()
                        .find_position(|enemy| !enemy.has_acted_this_turn);

                    if let Some((index, enemy)) = enemy {
                        enemy.has_acted_this_turn = true;

                        let action = enemy
                            .prototype
                            .get_moveset(bump)
                            .eval(&enemy.state_machine, &enemy.creature.statuses);

                        did_act = true;

                        let mut state = Distribution::single_value(state, bump);
                        for action in action.actions {
                            state = match action {
                                EnemyAction::Attack {
                                    base_damage,
                                    repeat,
                                } => {
                                    for _ in 0..*repeat {
                                        state = state.flat_map(
                                            |state, bump| {
                                                state.apply_attack_damage(
                                                    CharacterIndex::Enemy(index),
                                                    *base_damage,
                                                    CharacterIndex::Player,
                                                    bump,
                                                )
                                            },
                                            bump,
                                        );
                                    }
                                    state
                                }
                                EnemyAction::Block { amount } => state.flat_map(
                                    |state, bump| {
                                        state.add_block_to_creature(
                                            CharacterIndex::Enemy(index),
                                            *amount,
                                            bump,
                                        )
                                    },
                                    bump,
                                ),
                                EnemyAction::ApplyStatusSelf { status, diff } => state.flat_map(
                                    |state, bump| {
                                        state.apply_status_change(
                                            CharacterIndex::Enemy(index),
                                            *status,
                                            *diff,
                                            bump,
                                        )
                                    },
                                    bump,
                                ),
                                EnemyAction::ApplyStatusPlayer { status, diff } => state.flat_map(
                                    |state, bump| {
                                        state.apply_status_change(
                                            CharacterIndex::Player,
                                            *status,
                                            *diff,
                                            bump,
                                        )
                                    },
                                    bump,
                                ),
                                EnemyAction::ShuffleCards { card, count, pile } => state.map(
                                    |mut state| {
                                        for _ in 0..*count {
                                            match pile {
                                                Pile::Draw => {
                                                    state.player.draw_pile.add_card(*card);
                                                }
                                                Pile::Hand => {
                                                    state.player.hand.add_card(*card);
                                                }
                                                Pile::Discard => {
                                                    state.player.discard_pile.add_card(*card);
                                                }
                                            }
                                        }
                                        state
                                    },
                                    bump,
                                ),
                            };
                        }

                        state
                    } else {
                        Distribution::single_value(state, bump)
                    }
                },
                bump,
            );

            if !did_act {
                break;
            }
        }

        // Next enemy intents
        let state = state.map(
            |mut state| {
                for enemy in &mut state.enemies {
                    enemy
                        .prototype
                        .get_moveset(bump)
                        .advance(&mut enemy.state_machine, &mut enemy.creature.statuses);

                    enemy.has_acted_this_turn = false;
                }

                state
            },
            bump,
        );

        state
    }

    fn on_end_enemy_turn(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        for enemy in &mut self.enemies {
            enemy.has_taken_unblocked_damage_this_turn = false;

            let mut status_diff: EnumMap<Status, i16> = EnumMap::default();

            for (status, count) in &mut enemy.creature.statuses {
                match status {
                    Status::Vulnerable => decrease_non_neg(count),
                    Status::Weak => decrease_non_neg(count),
                    Status::CorrosiveWave => *count = 0,
                    Status::Frail => decrease_non_neg(count),
                    Status::Territorial => {
                        status_diff[Status::Strength] += *count;
                    }
                    Status::Anticipate => {
                        status_diff[Status::Dexterity] -= *count;
                        *count = 0;
                    }

                    _ => {}
                }
            }
            for (v, count) in self
                .player
                .creature
                .statuses
                .values_mut()
                .zip(status_diff.into_values())
            {
                *v += count;
            }
        }

        Distribution::single_value(self, bump)
    }

    fn on_start_player_turn(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        self.turn_counter += 1;

        // Give Player Energy
        // FIXME: Calculate the amount of energy to give
        self.player.energy = 3;

        // Remove player block
        // TODO: Keep block power
        self.player.creature.block = 0;

        for (status, count) in &mut self.player.creature.statuses {
            match status {
                Status::Poison => {
                    // I dont think the player can be poisoned
                    self.player.creature.hp = self.player.creature.hp.saturating_sub_signed(*count);
                    decrease_non_neg(count);
                }
                Status::BonusEnergyOnTurnStart => {
                    self.player.energy += u8::try_from(*count).unwrap();
                    *count = 0;
                }
                Status::NoxiousFumes => {
                    if *count == 0 {
                        continue;
                    }
                    // TODO: This does not respect triggers in 'apply_status'
                    for enemy in &mut self.enemies {
                        if enemy.creature.statuses[Status::Artifact] > 0 {
                            enemy.creature.statuses[Status::Artifact] -= 1;
                        } else {
                            enemy.creature.statuses[Status::Poison] += *count;
                        }
                    }
                }
                Status::BlockNextTurn => {
                    self.player.creature.block +=
                        u16::try_from(*count).expect("BlockNextTurn must be positive");
                    *count = 0;
                }
                Status::Plating => {
                    decrease_non_neg(count);
                }

                _ => {}
            }
        }

        let state = Distribution::single_value(self, bump);

        let state = state.flat_map(
            |state, bump| {
                // TODO: Relics
                if state.turn_counter == 1
                    && state.relic_state.contains(RelicPrototype::RingOfTheSnake)
                {
                    let mut state = Distribution::single_value(state, bump);
                    for _ in 0..2 {
                        state = state.flat_map(CombatState::draw_single_card, bump);
                    }
                    state
                } else {
                    Distribution::single_value(state, bump)
                }
            },
            bump,
        );

        state.flat_map(Self::draw_cards_for_turn, bump)
    }

    fn draw_specific_card(mut self, card: Card, bump: &'bump Bump) -> Distribution<'bump, Self> {
        assert!(self.player.draw_pile_top_card.is_none());

        self.player.draw_pile.remove_card(card);
        self.player.hand.add_card(card);

        self.on_draw_card(bump)
    }

    fn on_draw_card(self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        // Stuff like kingly kick (I think that gets cheaper when you draw it)

        assert!(self.player.creature.statuses[Status::CorrosiveWave] >= 0);
        let corrosive = self.player.creature.statuses[Status::CorrosiveWave].abs();

        let num_enemies = self.enemies.len();
        let mut state = Distribution::single_value(self, bump);
        // TODO: Index shift problems

        if corrosive > 0 {
            // Apply Corrosive
            for enemy in 0..num_enemies {
                state = state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Enemy(enemy),
                            Status::Poison,
                            corrosive,
                            bump,
                        )
                    },
                    bump,
                );
            }
        }

        state
    }

    fn on_draw_non_draw_phase_card(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        // TODO: Stuff like speedster

        self.on_draw_card(bump)
    }

    fn shuffle_discard_into_draw(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        self.player.draw_pile.append(&mut self.player.discard_pile);

        Distribution::single_value(self, bump)
    }

    // The card must already be removed from whereever it came from, so we take it by value here to express that
    #[allow(clippy::needless_pass_by_value)]
    fn play_card(
        self,
        card: Card,
        target: Option<usize>,

        is_raw_play: bool,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        let state = Distribution::single_value(self, bump);

        let state = match card.prototype {
            CardPrototype::AscendersBane => unreachable!("Ascender's bane is unplayable"),
            CardPrototype::Dazed => unreachable!("Dazed is unplayable"),
            CardPrototype::Infection => unreachable!("Infection is unplayable"),
            CardPrototype::Wound => unreachable!("Wound is unplayable"),
            CardPrototype::Greed => unreachable!("Greed is unplayable"),
            CardPrototype::Strike => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 9 } else { 6 };

                state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Defend => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 8 } else { 5 };

                state.flat_map(
                    |slf, bump| {
                        let fasten = slf.player.creature.statuses[Status::Fasten];
                        slf.add_block_to_creature(
                            CharacterIndex::Player,
                            base_amount + u16::try_from(fasten).expect("Fasten must be positive"),
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Dash => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 13 } else { 10 };

                let state = state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |slf, bump| {
                        slf.add_block_to_creature(CharacterIndex::Player, base_amount, bump)
                    },
                    bump,
                )
            }
            CardPrototype::Neutralize => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 4 } else { 3 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        state.apply_status_to_enemy(
                            target,
                            Status::Weak,
                            if card.upgraded { 2 } else { 1 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::SuckerPunch => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 10 } else { 8 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        state.apply_status_to_enemy(
                            target,
                            Status::Weak,
                            if card.upgraded { 2 } else { 1 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Squash => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 12 } else { 10 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        state.apply_status_to_enemy(
                            target,
                            Status::Vulnerable,
                            if card.upgraded { 3 } else { 2 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Survivor => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 11 } else { 8 };

                state.flat_map(
                    |mut slf, bump| {
                        let state = if slf.player.hand.num_cards() > 0 {
                            if slf.player.hand.num_cards() > 1 {
                                slf.player.waiting_for_decision =
                                    Some(RequiredPlayerDecision::ChooseCardInHand {
                                        filter: |_| true,
                                        action: |state, bump, card| {
                                            state.flat_map(
                                                |mut state, bump| {
                                                    state.player.waiting_for_decision = None;
                                                    state.discard_card(card, bump)
                                                },
                                                bump,
                                            )
                                        },
                                    });
                                Distribution::single_value(slf, bump)
                            } else {
                                let card = *slf
                                    .player
                                    .hand
                                    .iter()
                                    .next()
                                    .expect("Hand contains a single card");
                                slf.discard_card(card, bump)
                            }
                        } else {
                            Distribution::single_value(slf, bump)
                        };
                        state.flat_map(
                            |state, bump| {
                                state.add_block_to_creature(
                                    CharacterIndex::Player,
                                    base_amount,
                                    bump,
                                )
                            },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::PoisonedStab => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 8 } else { 6 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        state.apply_status_to_enemy(
                            target,
                            Status::Poison,
                            if card.upgraded { 4 } else { 3 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Backflip => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 8 } else { 5 };

                let mut state = state.flat_map(
                    |slf, bump| {
                        slf.add_block_to_creature(CharacterIndex::Player, base_amount, bump)
                    },
                    bump,
                );

                let cards = 2;

                for _ in 0..cards {
                    state = state.flat_map(CombatState::draw_single_card, bump);
                }
                state.dedup();
                state
            }
            CardPrototype::DeadlyPoison => {
                let target = target.unwrap();

                state.flat_map(
                    |state, bump| {
                        state.apply_status_to_enemy(
                            target,
                            Status::Poison,
                            if card.upgraded { 7 } else { 5 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::CorrosiveWave => {
                assert!(target.is_none());
                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::CorrosiveWave,
                            if card.upgraded { 4 } else { 3 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Burst => {
                assert!(target.is_none());
                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Burst,
                            if card.upgraded { 2 } else { 1 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Footwork => {
                assert!(target.is_none());
                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Dexterity,
                            if card.upgraded { 3 } else { 2 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Accuracy => {
                assert!(target.is_none());
                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Accuracy,
                            if card.upgraded { 6 } else { 4 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::LegSweep => {
                let target = target.unwrap();

                let state = state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Enemy(target),
                            Status::Weak,
                            if card.upgraded { 3 } else { 2 },
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        state.add_block_to_creature(
                            CharacterIndex::Player,
                            if card.upgraded { 14 } else { 11 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::PreciseCut => {
                let target = target.unwrap();

                let base_amount: usize = if card.upgraded { 16 } else { 13 };

                state.flat_map(
                    |state, bump| {
                        let num_hand_cards = state.player.hand.num_cards();

                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            // FIXME: Strength and the negative are prob additive here, so this is overestimating the dmg slightly
                            (base_amount.saturating_sub(2 * num_hand_cards))
                                .try_into()
                                .unwrap(),
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Anticipate => {
                assert!(target.is_none());

                let state = state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Dexterity,
                            if card.upgraded { 5 } else { 3 },
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Anticipate,
                            if card.upgraded { 4 } else { 3 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::NoxiousFumes => {
                assert!(target.is_none());

                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::NoxiousFumes,
                            if card.upgraded { 3 } else { 2 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Fasten => {
                assert!(target.is_none());

                state.flat_map(
                    |state, bump| {
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Fasten,
                            if card.upgraded { 7 } else { 5 },
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::DodgeAndRoll => {
                assert!(target.is_none());

                let state = state.flat_map(
                    |state, bump| {
                        state.add_block_to_creature(
                            CharacterIndex::Player,
                            if card.upgraded { 6 } else { 4 },
                            bump,
                        )
                    },
                    bump,
                );

                state.flat_map(
                    |state, bump| {
                        let amount = state.calculate_block(
                            CharacterIndex::Player,
                            if card.upgraded { 6 } else { 4 },
                        );
                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::BlockNextTurn,
                            amount
                                .try_into()
                                .expect("More than i16::MAX block next turn"),
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Shiv => {
                let target = target.unwrap();

                state.flat_map(
                    |state, bump| {
                        let base_amount = if card.upgraded { 6 } else { 4 }
                            + u16::try_from(state.player.creature.statuses[Status::Accuracy])
                                .expect("Accuracy should always be positive");
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::CloakAndDagger => {
                assert!(target.is_none());

                let state = state.flat_map(
                    |state, bump| state.add_block_to_creature(CharacterIndex::Player, 6, bump),
                    bump,
                );

                state.map(
                    |mut state| {
                        for _ in 0..(if card.upgraded { 2 } else { 1 }) {
                            state.player.hand.add_card(Card {
                                prototype: CardPrototype::Shiv,
                                upgraded: false,
                                enchantment: None,
                            });
                        }

                        state
                    },
                    bump,
                )
            }
            CardPrototype::BladeDance => {
                assert!(target.is_none());

                state.map(
                    |mut state| {
                        for _ in 0..(if card.upgraded { 4 } else { 3 }) {
                            state.player.hand.add_card(Card {
                                prototype: CardPrototype::Shiv,
                                upgraded: false,
                                enchantment: None,
                            });
                        }

                        state
                    },
                    bump,
                )
            }
            CardPrototype::LeadingStrike => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 10 } else { 7 };

                let state = state.flat_map(
                    |state, bump| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_amount,
                            CharacterIndex::Enemy(target),
                            bump,
                        )
                    },
                    bump,
                );

                state.map(
                    |mut state| {
                        state.player.hand.add_card(Card {
                            prototype: CardPrototype::Shiv,
                            upgraded: false,
                            enchantment: None,
                        });

                        state
                    },
                    bump,
                )
            }
            CardPrototype::Tracking => {
                assert!(target.is_none());

                state.flat_map(
                    |state, bump| {
                        let change = if state.player.creature.statuses[Status::Tracking] == 0 {
                            2
                        } else {
                            1
                        };

                        state.apply_status_change(
                            CharacterIndex::Player,
                            Status::Tracking,
                            change,
                            bump,
                        )
                    },
                    bump,
                )
            }
            CardPrototype::Haze => {
                assert!(target.is_none());

                state.flat_map(
                    |state, bump| {
                        let poison_amount = if card.upgraded { 6 } else { 4 };

                        state.for_all_enemies(
                            |state, enemy_index, bump| {
                                state.apply_status_to_enemy(
                                    enemy_index,
                                    Status::Poison,
                                    poison_amount,
                                    bump,
                                )
                            },
                            bump,
                        )
                    },
                    bump,
                )
            }
        };

        let mut state = state.flat_map(Self::on_any_card_played, bump);

        if is_raw_play && card.prototype.get_kind() == CardKind::Skill {
            state = state.flat_map(
                |mut state, bump| {
                    if state.player.creature.statuses[Status::Burst] > 0 {
                        // TODO: What if the target is no longer valid????
                        state.player.creature.statuses[Status::Burst] -= 1;
                        state.play_card(card, target, false, bump)
                    } else {
                        Distribution::single_value(state, bump)
                    }
                },
                bump,
            );
        }

        if is_raw_play {
            if card.prototype.get_kind() == CardKind::Power {
                state
            } else if card.has_exhaust() {
                state.map(
                    |mut state| {
                        state.player.exhaust_pile.add_card(card);
                        state
                    },
                    bump,
                )
            } else {
                state.map(
                    |mut state| {
                        state.player.discard_pile.add_card(card);
                        state
                    },
                    bump,
                )
            }
        } else {
            state
        }
    }

    fn for_all_enemies(
        self,
        fun: impl Fn(Self, usize, &'bump Bump) -> Distribution<'bump, Self>,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        let num_enemies = self.enemies.len();
        let mut state = Distribution::single_value(self, bump);

        // FIXME: Index shifts
        for enemy_index in 0..num_enemies {
            state = state.flat_map(|state, bump| (fun)(state, enemy_index, bump), bump);
        }

        state
    }

    fn discard_card(mut self, card: Card, bump: &'bump Bump) -> Distribution<'bump, Self> {
        self.player.hand.remove_card(card);

        if card.has_sly() {
            // FIXME: What about targeting???
            self.play_card(card, None, true, bump)
        } else {
            self.player.discard_pile.add_card(card);
            Distribution::single_value(self, bump)
        }
    }

    fn add_block_to_creature(
        mut self,
        creature: CharacterIndex,
        base_amount: u16,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        let amount = self.calculate_block(creature, base_amount);

        match creature {
            CharacterIndex::Player => self.player.creature.block += amount,
            CharacterIndex::Enemy(index) => self.enemies[index].creature.block += amount,
        }

        // TODO: Triggers

        Distribution::single_value(self, bump)
    }

    fn calculate_block(&self, creature: CharacterIndex, base_amount: u16) -> u16 {
        let status = match creature {
            CharacterIndex::Player => &self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &self.enemies[index].creature.statuses,
        };

        let amount = base_amount.saturating_add_signed(status[Status::Dexterity]);

        let amount = if status[Status::Frail] > 0 {
            (amount as f32 * 0.75) as u16
        } else {
            amount
        };
        amount
    }

    fn apply_attack_damage(
        mut self,
        source: CharacterIndex,
        base_amount: u16,
        target: CharacterIndex,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        let source_status = match source {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        let imbalanced = source_status[Status::Imbalanced] > 0;

        let amount = base_amount.saturating_add_signed(source_status[Status::Strength]);

        // Use up vigor
        let amount = amount.saturating_add_signed(source_status[Status::Vigor]);
        source_status[Status::Vigor] = 0;

        let amount = if source_status[Status::Weak] > 0 {
            f32::from(amount) * 0.75
        } else {
            f32::from(amount)
        };

        let mut amount = if source_status[Status::Shrink] > 0 {
            amount * 0.7
        } else {
            amount
        };

        let source_has_tracking =
            (source_status[Status::Tracking] > 0).then_some(source_status[Status::Tracking]);

        let target_status = match target {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        if target_status[Status::Weak] > 0
            && let Some(tracking_mul) = source_has_tracking
        {
            amount *= f32::from(tracking_mul);
        }

        let personal_hive = target_status[Status::PersonalHive];

        let amount = if target_status[Status::Vulnerable] > 0 {
            amount * 1.5
        } else {
            amount
        };

        let amount = amount as u16;

        // TODO: Triggers
        match target {
            CharacterIndex::Player => {
                let mut unblocked = amount.saturating_sub(self.player.creature.block);
                self.player.creature.block = self.player.creature.block.saturating_sub(amount);
                if unblocked > 0 && target_status[Status::Slippery] > 0 {
                    unblocked = 1;
                    target_status[Status::Slippery] -= 1;
                }

                if unblocked == 0 && imbalanced {
                    match source {
                        CharacterIndex::Player => todo!("Stun the player"),
                        CharacterIndex::Enemy(source_enemy_index) => {
                            self.enemies[source_enemy_index].state_machine.stunned = 2;
                        }
                    }
                }
                self.player.creature.hp = self.player.creature.hp.saturating_sub(unblocked);
            }
            CharacterIndex::Enemy(index) => {
                let enemy_block = &mut self.enemies[index].creature.block;
                let mut unblocked = amount.saturating_sub(*enemy_block);
                *enemy_block = enemy_block.saturating_sub(amount);
                if unblocked > 0 {
                    if self.enemies[index].creature.statuses[Status::Slippery] > 0 {
                        unblocked = 1;
                        self.enemies[index].creature.statuses[Status::Slippery] -= 1;
                    }

                    if self.enemies[index].creature.statuses[Status::VitalSpark] > 0
                        && source == CharacterIndex::Player
                        && !self.enemies[index].has_taken_unblocked_damage_this_turn
                    {
                        self.player.energy += 1;
                    }

                    self.enemies[index].has_taken_unblocked_damage_this_turn = true;
                }

                if unblocked == 0 && imbalanced {
                    match source {
                        CharacterIndex::Player => todo!("Stun the player"),
                        CharacterIndex::Enemy(source_enemy_index) => {
                            self.enemies[source_enemy_index].state_machine.stunned = 2;
                        }
                    }
                }
                self.enemies[index].creature.hp =
                    self.enemies[index].creature.hp.saturating_sub(unblocked);
            }
        }

        let mut state = match target {
            CharacterIndex::Player => Distribution::single_value(self, bump),
            CharacterIndex::Enemy(enemy_index) => self.on_enemy_lost_hp(enemy_index, bump),
        };

        if personal_hive > 0 {
            state = state.map(
                |mut state| {
                    for _ in 0..personal_hive {
                        state.player.draw_pile.add_card(Card {
                            prototype: CardPrototype::Dazed,
                            upgraded: false,
                            enchantment: None,
                        });
                    }
                    state
                },
                bump,
            );
        }

        state
    }

    fn on_enemy_lost_hp(
        mut self,
        enemy_index: usize,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        if self.enemies[enemy_index].creature.statuses[Status::Slumber] > 0 {
            self.enemies[enemy_index].creature.statuses[Status::Slumber] -= 1;
            if self.enemies[enemy_index].creature.statuses[Status::Slumber] == 0 {
                self.enemies[enemy_index].creature.statuses[Status::Plating] = 0;
                self.enemies[enemy_index].state_machine.stunned = 1;
            }
        }
        if self.enemies[enemy_index].creature.statuses[Status::Asleep] > 0 {
            self.enemies[enemy_index].creature.statuses[Status::Slumber] = 0;
            self.enemies[enemy_index].creature.statuses[Status::Plating] = 0;
            self.enemies[enemy_index].state_machine.stunned = 1;
        }

        self.enemies.retain(|enemy| enemy.creature.hp > 0);

        Distribution::single_value(self, bump)
    }

    fn apply_status_to_enemy(
        self,
        enemy_index: usize,
        status: Status,
        amount: i16,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        self.apply_status_change(CharacterIndex::Enemy(enemy_index), status, amount, bump)
    }

    fn apply_status_change(
        mut self,
        target: CharacterIndex,
        status: Status,
        diff: i16,
        bump: &'bump Bump,
    ) -> Distribution<'bump, Self> {
        assert_ne!(diff, 0);

        let status_list = match target {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => {
                if let Some(enemy) = self.enemies.get_mut(index) {
                    &mut enemy.creature.statuses
                } else {
                    return Distribution::single_value(self, bump);
                }
            }
        };

        if status.is_debuff() && status_list[Status::Artifact] > 0 {
            status_list[Status::Artifact] -= 1;
            return Distribution::single_value(self, bump);
        }

        status_list[status] += diff;

        Distribution::single_value(self, bump)
    }

    fn on_any_card_played(mut self, bump: &'bump Bump) -> Distribution<'bump, Self> {
        Distribution::single_value(self, bump)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player<'bump> {
    pub hand: UnorderedCardSet<'bump>,
    draw_pile: UnorderedCardSet<'bump>,
    draw_pile_top_card: Option<Card>,
    discard_pile: UnorderedCardSet<'bump>,
    exhaust_pile: UnorderedCardSet<'bump>,

    // TODO: Unfortunately the real algorithm is muuch more complex
    play_pile: UnorderedCardSet<'bump>,
    waiting_for_decision: Option<RequiredPlayerDecision>,

    orbs: Vec<'bump, Orb>,
    num_orb_slots: u8,

    energy: u8,
    stars: u8,

    pub creature: Creature,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RequiredPlayerDecision {
    ChooseCardInHand {
        filter: fn(Card) -> bool,
        action: for<'bump> fn(
            Distribution<'bump, CombatState<'bump>>,
            &'bump Bump,
            Card,
        ) -> Distribution<'bump, CombatState<'bump>>,
    },
}

impl Player<'_> {
    fn launder(self, new_bump: &Bump) -> Player<'_> {
        Player {
            hand: self.hand.launder(new_bump),
            draw_pile: self.draw_pile.launder(new_bump),
            draw_pile_top_card: self.draw_pile_top_card,
            discard_pile: self.discard_pile.launder(new_bump),
            exhaust_pile: self.exhaust_pile.launder(new_bump),
            play_pile: self.play_pile.launder(new_bump),
            waiting_for_decision: self.waiting_for_decision,
            orbs: self.orbs.into_iter().collect_in(new_bump),
            num_orb_slots: self.num_orb_slots,
            energy: self.energy,
            stars: self.stars,
            creature: self.creature,
        }
    }
}

impl<'bump> Player<'bump> {
    pub fn default(bump: &'bump Bump) -> Self {
        Self {
            hand: vec![in bump;
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
            ]
            .into_iter()
            .collect_in(bump),
            draw_pile: vec![in bump;
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Neutralize.get_normal_card(),
                CardPrototype::Survivor.get_normal_card(),
            ]
            .into_iter()
            .collect_in(bump),
            draw_pile_top_card: None,
            discard_pile: vec![in bump;].into_iter().collect_in(bump),
            exhaust_pile: vec![in bump;].into_iter().collect_in(bump),
            play_pile: vec![in bump;].into_iter().collect_in(bump),
            waiting_for_decision: None,
            orbs: vec![in bump;],
            num_orb_slots: 1,
            energy: 3,
            stars: 0,
            creature: Creature {
                hp: 70,
                max_hp: 70,
                block: 0,
                statuses: EnumMap::default(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Orb {
    Lightning,
    Frost,
    Dark { accumulator: u16 },
    Plasma,
    Glass { damage_reduction: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Enemy {
    pub prototype: EnemyPrototype,
    pub creature: Creature,

    pub has_acted_this_turn: bool,

    pub has_taken_unblocked_damage_this_turn: bool,

    pub state_machine: EnemyStateMachine,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Creature {
    pub hp: u16,
    pub max_hp: u16,
    pub block: u16,

    pub statuses: enum_map::EnumMap<Status, i16>,
}

#[derive(Debug, Clone, Copy, Enum, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum Status {
    #[serde(rename = "STRENGTH_POWER")]
    Strength,
    #[serde(rename = "DEXTERITY_POWER")]
    Dexterity,
    #[serde(rename = "VULNERABLE_POWER")]
    Vulnerable,
    #[serde(rename = "WEAK_POWER")]
    Weak,
    #[serde(rename = "POISON_POWER")]
    Poison,
    #[serde(rename = "SHRINK_POWER")]
    Shrink,
    #[serde(rename = "CORROSIVE_WAVE_POWER")]
    CorrosiveWave,
    #[serde(rename = "ARTIFACT_POWER")]
    Artifact,
    #[serde(rename = "FRAIL_POWER")]
    Frail,
    Focus,
    Vigor,
    BonusEnergyOnTurnStart,

    #[serde(rename = "TERRITORIAL_POWER")]
    Territorial,

    #[serde(rename = "ANTICIPATE_POWER")]
    Anticipate,

    #[serde(rename = "NOXIOUS_FUMES_POWER")]
    NoxiousFumes,
    #[serde(rename = "FASTEN_POWER")]
    Fasten,
    #[serde(rename = "BLOCK_NEXT_TURN_POWER")]
    BlockNextTurn,

    #[serde(rename = "SLIPPERY_POWER")]
    Slippery,
    #[serde(rename = "TRACKING_POWER")]
    Tracking,
    #[serde(rename = "IMBALANCED_POWER")]
    Imbalanced,
    #[serde(rename = "ACCURACY_POWER")]
    Accuracy,
    #[serde(rename = "VITAL_SPARK_POWER")]
    VitalSpark,
    #[serde(rename = "PLATING_POWER")]
    Plating,
    #[serde(rename = "PERSONAL_HIVE_POWER")]
    PersonalHive,

    #[serde(rename = "BURST_POWER")]
    Burst,

    #[serde(rename = "SLUMBER_POWER")]
    Slumber,
    #[serde(rename = "ASLEEP_POWER")]
    Asleep,
}

impl Status {
    fn is_debuff(self) -> bool {
        match self {
            Status::Strength => false,
            Status::Dexterity => false,
            Status::Vulnerable => true,
            Status::Weak => true,
            Status::Poison => true,
            Status::Shrink => true,
            Status::CorrosiveWave => false,
            Status::Artifact => false,
            Status::Frail => true,
            Status::Focus => false,
            Status::Vigor => false,
            Status::BonusEnergyOnTurnStart => false,
            Status::Territorial => false,
            Status::Anticipate => false,
            Status::NoxiousFumes => false,
            Status::Fasten => false,
            Status::BlockNextTurn => false,
            Status::Slippery => false,
            Status::Tracking => false,
            Status::Imbalanced => true,
            Status::Accuracy => false,
            Status::VitalSpark => true,
            Status::Plating => false,
            Status::PersonalHive => false,
            Status::Burst => false,
            Status::Slumber => true,
            Status::Asleep => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnemyStateMachine {
    pub current_state: usize,
    pub stunned: u8,
}

impl Default for EnemyStateMachine {
    fn default() -> Self {
        Self {
            current_state: 0,
            stunned: 0,
        }
    }
}

pub enum EnemyMoveSet<'bump> {
    ConstantRotation {
        // TODO: static would be much better
        // rotation: &'static [EnemyMove],
        rotation: Vec<'bump, EnemyMove>,
    },
    Prefix {
        prefixed_move: EnemyMove,
        after: Box<'bump, Self>,
    },
}

impl EnemyMoveSet<'_> {
    pub fn eval(
        &self,
        state_machine: &EnemyStateMachine,
        status: &EnumMap<Status, i16>,
    ) -> EnemyMove {
        if state_machine.stunned > 0 || status[Status::Slumber] > 0 || status[Status::Asleep] > 0 {
            return EnemyMove { actions: &[] };
        }

        match self {
            EnemyMoveSet::ConstantRotation { rotation } => {
                rotation[state_machine.current_state % rotation.len()]
            }
            EnemyMoveSet::Prefix {
                prefixed_move,
                after,
            } => {
                if state_machine.current_state == 0 {
                    *prefixed_move
                } else {
                    after.eval(
                        &EnemyStateMachine {
                            current_state: state_machine.current_state - 1,
                            stunned: state_machine.stunned,
                        },
                        status,
                    )
                }
            }
        }
    }

    fn advance(&self, state_machine: &mut EnemyStateMachine, status: &mut EnumMap<Status, i16>) {
        if state_machine.stunned > 0 {
            state_machine.stunned -= 1;
            return;
        }

        if status[Status::Slumber] > 0 {
            status[Status::Slumber] -= 1;
            if status[Status::Slumber] == 0 {
                status[Status::Plating] = 0;
            }
            return;
        }

        if status[Status::Asleep] > 0 {
            status[Status::Asleep] -= 1;
            if status[Status::Asleep] == 0 {
                status[Status::Plating] = 0;
            }
            return;
        }

        match self {
            Self::ConstantRotation { .. } => {
                state_machine.current_state += 1;
            }
            Self::Prefix { .. } => {
                state_machine.current_state += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnemyMove {
    pub actions: &'static [EnemyAction],
}

#[derive(Debug, Clone, Copy)]
pub enum EnemyAction {
    Attack { base_damage: u16, repeat: u16 },
    Block { amount: u16 },
    ApplyStatusSelf { status: Status, diff: i16 },
    ApplyStatusPlayer { status: Status, diff: i16 },
    ShuffleCards { card: Card, count: u8, pile: Pile },
}

#[derive(Debug, Clone, Copy)]
enum Pile {
    Draw,
    Hand,
    Discard,
}

impl EnemyStateMachine {
    fn get_intent(&self) -> Intent {
        todo!()
    }
}

enum Intent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum EnemyPrototype {
    Nibbit,
    SmallTwigSlime,
    MediumTwigSlime,
    SmallLeafSlime,
    MediumLeafSlime,
    FuzzyWurmCrawler,
    ShrinkerBeetle,
    Byrdonis,
    BygoneEffigy,
    CubexConstruct,
    AxeRubyRaider,
    AssassinRubyRaider,
    BruteRubyRaider,
    CrossbowRubyRaider,
    TrackerRubyRaider,
    Vantom,
    BowlbugRock,
    BowlbugEgg,
    BowlbugNectar,
    BowlbugSilk,
    SlumberingBeetle,
    InfestedPrism,
    Entomancer,
    Chomper,
}

impl EnemyPrototype {
    #[allow(clippy::match_same_arms)]
    pub fn get_moveset(self, bump: &Bump) -> EnemyMoveSet {
        match self {
            Self::Nibbit => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 12,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 6,
                                repeat: 1,
                            },
                            EnemyAction::Block { amount: 5 },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 2,
                        }],
                    },
                ],
            },
            Self::FuzzyWurmCrawler => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 7,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 1,
                        }],
                    },
                ],
            },
            Self::SmallTwigSlime => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 4,
                        repeat: 1,
                    }],
                }],
            },
            Self::MediumTwigSlime => todo!(),
            Self::SmallLeafSlime => todo!(),
            Self::MediumLeafSlime => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 0,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 8,
                            repeat: 1,
                        }],
                    },
                ],
            },

            Self::ShrinkerBeetle => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusPlayer {
                        status: Status::Shrink,
                        diff: 1,
                    }],
                },
                after: Box::new_in(
                    EnemyMoveSet::ConstantRotation {
                        rotation: vec![in bump;
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 7,
                                    repeat: 1,
                                }],
                            },
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 13,
                                    repeat: 1,
                                }],
                            },
                        ],
                    },
                    bump,
                ),
            },
            Self::Byrdonis => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 16,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 3,
                        }],
                    },
                ],
            },
            Self::BygoneEffigy => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove { actions: &[] },
                after: Box::new_in(
                    EnemyMoveSet::Prefix {
                        prefixed_move: EnemyMove {
                            actions: &[EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 10,
                            }],
                        },
                        after: Box::new_in(
                            EnemyMoveSet::ConstantRotation {
                                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 15, repeat: 1 }] }],
                            },
                            bump,
                        ),
                    },
                    bump,
                ),
            },
            Self::CubexConstruct => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusSelf {
                        status: Status::Strength,
                        diff: 2,
                    }],
                },
                after: Box::new_in(
                    EnemyMoveSet::ConstantRotation {
                        rotation: vec![in bump;
                        EnemyMove { actions: &[
                            EnemyAction::Attack { base_damage: 7, repeat: 1 },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 2,
                            }] },
                        EnemyMove { actions: &[
                            EnemyAction::Attack { base_damage: 7, repeat: 1 },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 2,
                            }] },
                        EnemyMove { actions: &[EnemyAction::Attack { base_damage: 5, repeat: 2 }, ] }],
                    },
                    bump,
                ),
            },
            Self::AxeRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 5, repeat: 1 }, EnemyAction::Block { amount: 5 }] }, EnemyMove { actions: &[EnemyAction::Attack { base_damage: 5, repeat: 1 }, EnemyAction::Block { amount: 5 }] }, EnemyMove { actions: &[EnemyAction::Attack { base_damage: 12, repeat: 1 }] }],
            },
            Self::AssassinRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 11, repeat: 1 }]}],
            },
            Self::BruteRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 7, repeat: 1 }]}, EnemyMove { actions: &[EnemyAction::ApplyStatusSelf { status: Status::Strength, diff: 3 }]}],
            },
            Self::CrossbowRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Block { amount: 3 }] }, EnemyMove { actions: &[EnemyAction::Attack { base_damage: 14, repeat: 1 }] }],
            },
            Self::TrackerRubyRaider => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusPlayer {
                        status: Status::Frail,
                        diff: 2,
                    }],
                },
                after: Box::new_in(
                    EnemyMoveSet::ConstantRotation {
                        rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 1, repeat: 8 }] }],
                    },
                    bump,
                ),
            },
            Self::Vantom => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                EnemyMove { actions: &[EnemyAction::Attack { base_damage: 7, repeat: 1 }] },
                EnemyMove { actions: &[EnemyAction::Attack { base_damage: 6, repeat: 2 }] },
                EnemyMove { actions: &[EnemyAction::Attack { base_damage: 27, repeat: 1 }, EnemyAction::ShuffleCards { card: Card { prototype: CardPrototype::Wound, upgraded: false, enchantment: None }, count: 3, pile: Pile::Discard}] },
                EnemyMove { actions: &[EnemyAction::ApplyStatusSelf { status: Status::Strength, diff: 2 }] },
                ],
            },
            Self::BowlbugRock => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 15, repeat: 1 }] }],
            },
            Self::BowlbugEgg => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::Attack { base_damage: 7, repeat: 1 }, EnemyAction::Block { amount: 7 }] }],
            },
            Self::BowlbugSilk => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump; EnemyMove { actions: &[EnemyAction::ApplyStatusPlayer { status: Status::Weak, diff: 1 }] }, EnemyMove { actions: &[EnemyAction::Attack { base_damage: 4, repeat: 2 }] }],
            },
            Self::BowlbugNectar => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 3,
                        repeat: 1,
                    }],
                },
                after: Box::new_in(
                    EnemyMoveSet::Prefix {
                        prefixed_move: EnemyMove {
                            actions: &[EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 15,
                            }],
                        },
                        after: Box::new_in(
                            EnemyMoveSet::ConstantRotation {
                                rotation: vec![in bump; EnemyMove {
                                    actions: &[EnemyAction::Attack {
                                        base_damage: 3,
                                        repeat: 1,
                                    }],
                                }],
                            },
                            bump,
                        ),
                    },
                    bump,
                ),
            },
            Self::InfestedPrism => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 22, repeat: 1 }] },
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 16, repeat: 1 }, EnemyAction::Block { amount: 16 }] },
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 9, repeat: 3 }] },
                    EnemyMove { actions:&[EnemyAction::Block { amount: 20 }, EnemyAction::ApplyStatusSelf { status: Status::Strength, diff: 4 }] },
                ],
            },
            Self::Entomancer => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 3, repeat: 7 }] },
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 18, repeat: 1 }] },
                    EnemyMove { actions:&[EnemyAction::ApplyStatusSelf { status: Status::PersonalHive, diff: 1 }, EnemyAction::ApplyStatusSelf { status: Status::Strength, diff: 1 }] },
                ],
            },
            Self::Chomper => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 8, repeat: 2 }] },
                    EnemyMove { actions:&[EnemyAction::ShuffleCards { card: Card { prototype: CardPrototype::Dazed, upgraded: false, enchantment: None }, count: 3, pile: Pile::Discard }] },
                ],
            },
            Self::SlumberingBeetle => EnemyMoveSet::ConstantRotation {
                rotation: vec![in bump;
                    EnemyMove { actions:&[EnemyAction::Attack { base_damage: 16, repeat: 1 }, EnemyAction::ApplyStatusSelf { status: Status::Strength, diff: 2 }] },
                ],
            },
        }
    }
}

fn decrease_non_neg(val: &mut i16) {
    *val = max(0, *val - 1);
}

#[cfg(test)]
pub(crate) mod test {
    use std::{collections::HashSet, iter};

    use enum_map::EnumMap;
    use rapidhash::fast::RandomState;

    use super::*;

    use bumpalo::{Bump, vec};

    pub fn simple_test_combat_state(bump: &Bump) -> CombatState<'_> {
        CombatState {
            turn_counter: 0,
            player: Player::default(bump),
            enemies: vec![in bump;
                Enemy {
                    prototype: EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    state_machine: EnemyStateMachine { current_state: 0, ..Default::default() },

                has_taken_unblocked_damage_this_turn: false,
                    has_acted_this_turn: false,
                },
                Enemy {
                    prototype: EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    state_machine: EnemyStateMachine { current_state: 2, ..Default::default() },

                has_taken_unblocked_damage_this_turn: false,
                    has_acted_this_turn: false,
                },
            ],
            relic_state: iter::empty().collect(),
        }
    }

    pub fn very_confused(bump: &Bump) -> CombatState {
        use crate::game_state::CardPrototype::*;
        use crate::game_state::EnemyPrototype::*;
        CombatState {
            turn_counter: 2,
            player: Player {
                hand: vec![in bump;
                    Card {
                        prototype: Neutralize,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Survivor,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                ]
                .into_iter()
                .collect_in(bump),
                draw_pile: vec![in bump;
                    Card {
                        prototype: Defend,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                ]
                .into_iter()
                .collect_in(bump),
                draw_pile_top_card: None,
                discard_pile: vec![in bump;].into_iter().collect_in(bump),
                exhaust_pile: vec![in bump;].into_iter().collect_in(bump),
                play_pile: vec![in bump;].into_iter().collect_in(bump),
                waiting_for_decision: None,
                orbs: vec![in bump;],
                num_orb_slots: 1,
                energy: 3,
                stars: 0,
                creature: Creature {
                    hp: 62,
                    max_hp: 70,
                    block: 6,
                    statuses: EnumMap::default(),
                },
            },
            enemies: vec![in bump;
                Enemy {
                    prototype: FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::from_array([7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    },
                    has_taken_unblocked_damage_this_turn: false,
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2, ..Default::default() },
                },
                Enemy {
                    prototype: FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 31,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                has_taken_unblocked_damage_this_turn: false,
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 1 , ..Default::default()},
                },
            ],
            relic_state: iter::empty().collect(),
        }
    }

    pub fn unneeded_blocking(bump: &Bump) -> CombatState {
        use crate::game_state::CardPrototype::*;
        use crate::game_state::EnemyPrototype::*;
        CombatState {
            turn_counter: 1,
            player: Player {
                hand: vec![in bump;
                    Card {
                        prototype: Defend,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Survivor,
                        upgraded: false,
                                enchantment: None,
                    },
                ]
                .into_iter()
                .collect_in(bump),
                draw_pile: vec![in bump;].into_iter().collect_in(bump),
                draw_pile_top_card: None,
                discard_pile: vec![in bump;
                    Card {
                        prototype: Neutralize,
                        upgraded: false,
                                enchantment: None,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                                enchantment: None,
                    },
                ]
                .into_iter()
                .collect_in(bump),
                exhaust_pile: vec![in bump;].into_iter().collect_in(bump),
                play_pile: vec![in bump;].into_iter().collect_in(bump),
                waiting_for_decision: None,
                orbs: vec![in bump;],
                num_orb_slots: 1,
                energy: 1,
                stars: 0,
                creature: Creature {
                    hp: 66,
                    max_hp: 70,
                    block: 0,
                    statuses: EnumMap::default(),
                },
            },
            enemies: vec![in bump;Enemy {
                prototype: FuzzyWurmCrawler,
                creature: Creature {
                    hp: 47,
                    max_hp: 57,
                    block: 0,
                    statuses: EnumMap::default(),
                },
                has_taken_unblocked_damage_this_turn: false,
                has_acted_this_turn: false,
                state_machine: EnemyStateMachine { current_state: 1, ..Default::default() },
            }],
            relic_state: iter::empty().collect(),
        }
    }

    pub fn transposition_test(bump: &Bump) -> CombatState {
        CombatState {
            turn_counter: 0,
            player: Player {
                hand: vec![in bump;
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                ]
                .into_iter()
                .collect_in(bump),
                draw_pile: vec![in bump;].into_iter().collect_in(bump),
                draw_pile_top_card: None,
                discard_pile: vec![in bump;].into_iter().collect_in(bump),
                exhaust_pile: vec![in bump;].into_iter().collect_in(bump),
                play_pile: vec![in bump;].into_iter().collect_in(bump),
                waiting_for_decision: None,
                orbs: vec![in bump;],
                num_orb_slots: 1,
                energy: 3,
                stars: 0,
                creature: Creature {
                    hp: 70,
                    max_hp: 70,
                    block: 0,
                    statuses: EnumMap::default(),
                },
            },
            enemies: vec![in bump;Enemy {
                prototype: EnemyPrototype::FuzzyWurmCrawler,
                creature: Creature {
                    hp: 55,
                    max_hp: 55,
                    block: 0,
                    statuses: EnumMap::default(),
                },
                state_machine: EnemyStateMachine { current_state: 2, ..Default::default() },

                has_acted_this_turn: false,
                has_taken_unblocked_damage_this_turn: false,
            }],
            relic_state: iter::empty().collect(),
        }
    }

    #[test]
    fn equality_for_card_sets() {
        let bump = &Bump::new();

        assert_eq!(
            vec![in bump; CardPrototype::Strike.get_normal_card()]
                .into_iter()
                .collect_in::<UnorderedCardSet>(bump),
            vec![in bump; CardPrototype::Strike.get_normal_card()]
                .into_iter()
                .collect_in(bump),
        );

        assert_ne!(
            vec![in bump; CardPrototype::Strike.get_normal_card()]
                .into_iter()
                .collect_in::<UnorderedCardSet>(bump),
            vec![in bump; CardPrototype::Defend.get_normal_card()]
                .into_iter()
                .collect_in(bump),
        );

        assert_eq!(
            vec![in bump;
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card()
            ]
            .into_iter()
            .collect_in::<UnorderedCardSet>(bump),
            vec![in bump;
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Strike.get_normal_card()
            ]
            .into_iter()
            .collect_in(bump),
        );

        let hash: HashSet<UnorderedCardSet, RandomState> = HashSet::from_iter(iter::once(
            vec![in bump;
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
            ]
            .into_iter()
            .collect_in(bump),
        ));

        assert!(
            hash.contains(
                &vec![in bump;
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card()
                ]
                .into_iter()
                .collect_in(bump)
            )
        );
    }
}
