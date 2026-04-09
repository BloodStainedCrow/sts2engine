use std::cmp::{max, min};

use enum_map::{Enum, EnumMap};
use itertools::Itertools;
use std::hash::Hash;
use strum::EnumIter;

use crate::distribution;
use crate::game_state::cards::{
    Card, CardKind, CardPrototype, Cost, CostVal, LegalTarget, UnorderedCardSet,
};
use crate::game_state::relics::{FullRelicState, RelicPrototype};
use crate::{combat_action::CombatAction, distribution::Distribution};

pub(crate) mod cards;
pub(crate) mod relics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatState {
    pub turn_counter: u8,

    pub player: Player,

    pub enemies: Vec<Enemy>,

    pub relic_state: FullRelicState,
}

impl Hash for CombatState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // TODO: The turn counter does not matter (for now)
        // TODO: Test if that increases the transposition hit rate
        self.turn_counter.hash(state);
        self.player.hash(state);
        self.enemies.hash(state);
        self.relic_state.hash(state);
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

#[derive(Debug, Clone, Copy, EnumIter)]
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
    TheKin,
    BowlbugsWeak,
    BowlbugsStrong,
    SoloTunneler,
    InfestedPrism,
    // Mr. Beeeees!!!
    Entomancer,
    Chompers,
    SlumberParty,
    TheInsatiable,
    TurretOperator,
    SlimedBerserker,
    MechaKnight,
    SoulNexus,
    TheLostAndForgotten,
    JaxfruitAndFlyconid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Act1,
    Act2,
    Act3,
}

impl EncounterPrototype {
    pub fn get_act(self) -> Act {
        use Act::*;
        match self {
            EncounterPrototype::FuzzyWurmCrawler => Act1,
            EncounterPrototype::SingleNibbit => Act1,
            EncounterPrototype::DoubleNibbit => Act1,
            EncounterPrototype::SlimesWeak => Act1,
            EncounterPrototype::ShrinkerBeetle => Act1,
            EncounterPrototype::Byrdonis => Act1,
            EncounterPrototype::BygoneEffigy => Act1,
            EncounterPrototype::SingleCubexConstruct => Act1,
            EncounterPrototype::BeetleAndFuzzy => Act1,
            EncounterPrototype::RubyRaiders => Act1,
            EncounterPrototype::JaxfruitAndFlyconid => Act1,
            EncounterPrototype::Vantom => Act1,
            EncounterPrototype::TheKin => Act1,
            EncounterPrototype::BowlbugsWeak => Act2,
            EncounterPrototype::BowlbugsStrong => Act2,
            EncounterPrototype::SoloTunneler => Act2,
            EncounterPrototype::InfestedPrism => Act2,
            EncounterPrototype::Entomancer => Act2,
            EncounterPrototype::Chompers => Act2,
            EncounterPrototype::SlumberParty => Act2,
            EncounterPrototype::TheInsatiable => Act2,
            EncounterPrototype::TurretOperator => Act3,
            EncounterPrototype::SlimedBerserker => Act3,
            EncounterPrototype::MechaKnight => Act3,
            EncounterPrototype::SoulNexus => Act3,
            EncounterPrototype::TheLostAndForgotten => Act3,
        }
    }

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
            EncounterPrototype::JaxfruitAndFlyconid => false,
            EncounterPrototype::Vantom => false,
            EncounterPrototype::TheKin => false,
            EncounterPrototype::BowlbugsWeak => false,
            EncounterPrototype::BowlbugsStrong => false,
            EncounterPrototype::SoloTunneler => false,
            EncounterPrototype::InfestedPrism => true,
            EncounterPrototype::Entomancer => true,
            EncounterPrototype::Chompers => false,
            EncounterPrototype::SlumberParty => false,
            EncounterPrototype::TheInsatiable => false,
            EncounterPrototype::TurretOperator => false,
            EncounterPrototype::SlimedBerserker => false,
            EncounterPrototype::MechaKnight => true,
            EncounterPrototype::SoulNexus => true,
            EncounterPrototype::TheLostAndForgotten => false,
        }
    }
}

// TODO:
pub struct RunInfo {
    pub hp: u16,
    pub max_hp: u16,
    pub deck: Vec<Card>,

    pub relic_state: FullRelicState,
}

impl CombatState {
    pub(crate) fn get_starting_states<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        encounter: EncounterPrototype,
        run_info: &RunInfo,

        mut enemy_max_hp_filter: impl FnMut(&[u16]) -> bool,
    ) -> Distribution {
        let state = Distribution::single_value(Self {
            turn_counter: 0,
            player: Player {
                hand: vec![].into_iter().collect(),
                draw_pile: run_info.deck.clone().into_iter().collect(),
                draw_pile_top_card: None,
                discard_pile: vec![].into_iter().collect(),
                exhaust_pile: vec![].into_iter().collect(),
                play_pile: vec![].into_iter().collect(),
                waiting_for_decision: None,
                orbs: vec![],
                num_orb_slots: 1,
                energy: 0,
                stars: 0,
                creature: Creature {
                    hp: run_info.hp,
                    max_hp: run_info.max_hp,
                    block: 0,
                    statuses: EnumMap::default(),
                },
                skip_next_duration_tick: EnumMap::default(),
            },
            enemies: vec![],

            relic_state: run_info.relic_state,
        });

        assert!(state.all_unique());

        let mut state_with_enemy = match encounter {
            EncounterPrototype::FuzzyWurmCrawler => {
                let hp = 55..=57;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
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
                    }))
                });

                state
            }
            EncounterPrototype::SingleNibbit => {
                let hp = 42..=46;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
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
                    }))
                });

                state
            }
            EncounterPrototype::DoubleNibbit => {
                let hps = (42..=46).cartesian_product(42..=46);

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hps.clone().map(|(first_hp, second_hp)| {
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
                    }))
                });

                state
            }
            EncounterPrototype::SlimesWeak => {
                let large_variant = 0..=1;

                let typ_and_hp_range = large_variant.map(|ty| {
                    let large = match ty {
                        0 => (EnemyPrototype::LeafSlimeM, 32..=35),
                        1 => (EnemyPrototype::TwigSlimeM, 26..=28),

                        _ => unreachable!(),
                    };

                    [
                        (EnemyPrototype::TwigSlimeS, 7..=11),
                        large,
                        (EnemyPrototype::LeafSlimeS, 11..=15),
                    ]
                });

                let typ_and_hp = typ_and_hp_range.flat_map(|[large, small_0, small_1]| {
                    large
                        .1
                        .cartesian_product(small_0.1)
                        .cartesian_product(small_1.1)
                        .map(move |((a, b), c)| [(large.0, a), (small_0.0, b), (small_1.0, c)])
                });

                let typ_and_hp_action_range = typ_and_hp.map(|enemies| {
                    enemies.map(|enemy| {
                        let action_range = match enemy.0 {
                            EnemyPrototype::LeafSlimeS => 0..=1,
                            EnemyPrototype::LeafSlimeM => 0..=0,
                            EnemyPrototype::TwigSlimeS => 0..=0,
                            EnemyPrototype::TwigSlimeM => 0..=0,

                            _ => unreachable!(),
                        };
                        (enemy.0, enemy.1, action_range)
                    })
                });

                let typ_and_hp_action = typ_and_hp_action_range.flat_map(|[a, b, c]| {
                    (a.2).cartesian_product(b.2).cartesian_product(c.2).map(
                        move |((a_action, b_action), c_action)| {
                            [
                                (a.0, a.1, a_action),
                                (b.0, b.1, b_action),
                                (c.0, c.1, c_action),
                            ]
                        },
                    )
                });

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(
                        typ_and_hp_action
                            .clone()
                            .cartesian_product([false, true])
                            .map(|(mut enemies, swap)| {
                        let mut state = state.clone();

                                if swap {
                                    enemies.reverse();
                                }

                        for (enemy, hp, starting_action) in enemies {
                            state.enemies.push(Enemy {
                                prototype: enemy,
                                creature: Creature {
                                    hp,
                                    max_hp: hp,
                                    block: 0,
                                    statuses: EnumMap::default(),
                                },
                                has_acted_this_turn: false,
                                state_machine: EnemyStateMachine {
                                    current_state: starting_action,
                                    ..Default::default()
                                },
                                has_taken_unblocked_damage_this_turn: false,
                            });
                        }

                        state
                            }),
                    )
                });

                state
            }
            EncounterPrototype::ShrinkerBeetle => {
                let hp = 38..=40;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
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
                    }))
                });

                state
            }
            EncounterPrototype::Byrdonis => {
                let hp = 91..=94;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
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
                    }))
                });

                state
            }
            EncounterPrototype::BygoneEffigy => {
                let hp = 127..=127;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
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
                    }))
                });

                state
            }
            EncounterPrototype::SingleCubexConstruct => {
                let hp = 65..=65;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
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
                    }))
                });

                state
            }
            EncounterPrototype::BeetleAndFuzzy => {
                let hp = (38..=40).cartesian_product(55..=57);

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|(beetle_hp, fuzzy_hp)| {
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
                    }))
                });

                state
            }
            EncounterPrototype::JaxfruitAndFlyconid => {
                let hp = (31..=33)
                    .cartesian_product(47..=49)
                    // FIXME: Technically this is not a 50/50 chance
                    .cartesian_product(1..=2);

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(
                        |((jax_hp, flyconid_hp), current_state)| {
                            let mut state = state.clone();

                            state.enemies.push(Enemy {
                                prototype: EnemyPrototype::SnappingJaxfruit,
                                creature: Creature {
                                    hp: jax_hp,
                                    max_hp: jax_hp,
                                    block: 0,
                                    statuses: EnumMap::default(),
                                },
                                has_acted_this_turn: false,
                                state_machine: EnemyStateMachine::default(),
                                has_taken_unblocked_damage_this_turn: false,
                            });

                            state.enemies.push(Enemy {
                                prototype: EnemyPrototype::Flyconid,
                                creature: Creature {
                                    hp: flyconid_hp,
                                    max_hp: flyconid_hp,
                                    block: 0,
                                    statuses: EnumMap::default(),
                                },
                                has_acted_this_turn: false,
                                state_machine: EnemyStateMachine {
                                    current_state,
                                    stunned: 0,
                                },
                                has_taken_unblocked_damage_this_turn: false,
                            });

                            state
                        },
                    ))
                });

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

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(typ_and_hp.clone().map(|enemies| {
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
                    }))
                });

                state
            }
            EncounterPrototype::Vantom => state.map(|mut state| {
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
            }),
            EncounterPrototype::TheKin => state.flat_map_simple(|mut state| {
                let hp = (58..=59).cartesian_product(58..=59);

                Distribution::equal_chance(hp.map(|(first, second)| {
                    let mut state = state.clone();

                    state.enemies.push(Enemy {
                        prototype: EnemyPrototype::KinFollower,
                        creature: Creature {
                            hp: first,
                            max_hp: first,
                            block: 0,
                            statuses: EnumMap::default(),
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_damage_this_turn: false,
                    });

                    state.enemies.push(Enemy {
                        prototype: EnemyPrototype::KinFollower,
                        creature: Creature {
                            hp: second,
                            max_hp: second,
                            block: 0,
                            statuses: EnumMap::default(),
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine {
                            current_state: 2,
                            stunned: 0,
                        },
                        has_taken_unblocked_damage_this_turn: false,
                    });

                    state.enemies.push(Enemy {
                        prototype: EnemyPrototype::KinPriest,
                        creature: Creature {
                            hp: 190,
                            max_hp: 190,
                            block: 0,
                            statuses: EnumMap::default(),
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_damage_this_turn: false,
                    });

                    state
                }))
            }),
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

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(typ_and_hp.clone().map(|enemies| {
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
                    }))
                });

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

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(typ_and_hp.clone().map(|enemies| {
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
                    }))
                });

                state
            }
            EncounterPrototype::SoloTunneler => {
                todo!("Adaptive state machine intent logic (check if we lost all block")
            }
            EncounterPrototype::InfestedPrism => state.map(|mut state| {
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
            }),
            EncounterPrototype::Entomancer => state.map(|mut state| {
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
            }),
            EncounterPrototype::Chompers => {
                let hp = (60..=64).cartesian_product(60..=64);

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|(first, second)| {
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
                    }))
                });

                state
            }
            EncounterPrototype::SlumberParty => {
                let hp = (45..=58)
                    .cartesian_product(40..=43)
                    .cartesian_product(86..=86)
                    .map(|((a, b), c)| [a, b, c]);

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|enemies| {
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
                    }))
                });

                state
            }
            EncounterPrototype::TheInsatiable => state.map(|mut state| {
                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::TheInsatiable,
                    creature: Creature {
                        hp: 321,
                        max_hp: 321,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::TurretOperator => state.map(|mut state| {
                let mut rampart = EnumMap::default();
                rampart[Status::Rampart] = 25;

                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::LivingShield,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: rampart,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });
                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::TurretOperator,
                    creature: Creature {
                        hp: 41,
                        max_hp: 41,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::SlimedBerserker => state.map(|mut state| {
                let mut status = EnumMap::default();

                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::SlimedBerserker,
                    creature: Creature {
                        hp: 266,
                        max_hp: 266,
                        block: 0,
                        statuses: status,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::MechaKnight => state.map(|mut state| {
                let mut status = EnumMap::default();

                status[Status::Artifact] = 3;

                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::MechaKnight,
                    creature: Creature {
                        hp: 300,
                        max_hp: 300,
                        block: 0,
                        statuses: status,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::SoulNexus => state.map(|mut state| {
                let mut status = EnumMap::default();

                state.enemies.push(Enemy {
                    prototype: todo!(),
                    creature: Creature {
                        hp: 234,
                        max_hp: 234,
                        block: 0,
                        statuses: status,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::TheLostAndForgotten => state.map(|mut state| {
                let mut the_lost = EnumMap::default();
                todo!();
                // the_lost[Status::PossessStrength] = 1;

                let mut the_forgotten = EnumMap::default();
                todo!();
                // the_lost[Status::PossessSpeed] = 1;

                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::TheLost,
                    creature: Creature {
                        hp: 93,
                        max_hp: 93,
                        block: 0,
                        statuses: the_lost,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });
                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::TheForgotten,
                    creature: Creature {
                        hp: 106,
                        max_hp: 106,
                        block: 0,
                        statuses: the_forgotten,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_damage_this_turn: false,
                });

                state
            }),
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
        assert!(!state.is_empty(), "Did you adjust the hp filter???");

        assert!(state.all_unique());
        dbg!(state.len());

        let mut state: Distribution = state.flat_map_simple(Self::on_start_player_turn);
        dbg!(state.len());
        assert!(!state.is_empty());

        // Innate cards
        state.retain_no_chance_fix(|state| {
            state.player.draw_pile.iter().all(|card| !card.has_innate())
        });
        let mut state = state.fix_odds();
        assert!(!state.is_empty());

        let mut state = if encounter.is_elite()
            && run_info.relic_state.contains(RelicPrototype::BoomingConch)
        {
            for _ in 0..2 {
                state = state.flat_map_simple(CombatState::draw_single_card);
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
            state = state.flat_map_simple(|state| {
                state.apply_status_change(CharacterIndex::Player, Status::Dexterity, 1)
            });
        }

        if run_info.relic_state.contains(RelicPrototype::Gorget) {
            state = state.flat_map_simple(|state| {
                state.apply_status_change(CharacterIndex::Player, Status::Plating, 4)
            });
        }

        if run_info.relic_state.contains(RelicPrototype::Vajra) {
            state = state.flat_map_simple(|state| {
                state.apply_status_change(CharacterIndex::Player, Status::Strength, 1)
            });
        }

        if run_info.relic_state.contains(RelicPrototype::BronzeScales) {
            state = state.flat_map_simple(|state| {
                state.apply_status_change(CharacterIndex::Player, Status::Thorns, 3)
            });
        }

        if run_info
            .relic_state
            .contains(RelicPrototype::BagOfPreparation)
        {
            for _ in 0..2 {
                state = state.flat_map_simple(Self::draw_single_card);
            }
        }

        if run_info.relic_state.contains(RelicPrototype::BagOfMarbles) {
            state = state.flat_map_simple(|state| {
                let enemies = state.enemies.len();
                let mut state = Distribution::single_value(state);
                for enemy in 0..enemies {
                    state = state.flat_map_simple(|state| {
                        state.apply_status_change(
                            CharacterIndex::Enemy(enemy),
                            Status::Vulnerable,
                            1,
                        )
                    });
                }
                state
            });
        }

        if run_info.relic_state.contains(RelicPrototype::Bellows) {
            state = state.map(|mut state| {
                state.player.hand.upgrade_all();
                state
            });
        }

        // assert!(state.entries.iter().map(|(v, _)| v).all_unique());
        assert!(!state.is_empty());

        dbg!(state.len());
        state.dedup();
        dbg!(state.len());

        // assert!(state.entries.iter().map(|(v, _)| v).all_unique());

        state.dedup();
        assert!(!state.is_empty());

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
        } else if self
            .enemies
            .iter()
            .all(|enemy| enemy.creature.statuses[Status::Minion] > 0)
        {
            // Only minions left
            Some(PostCombatState {
                turn_counter: self.turn_counter,

                hp: self.player.creature.hp,
                max_hp: self.player.creature.max_hp,
                potions_used: [false; 10],
                bonus_card_rewards: 0,

                relic_state: self.relic_state,
            })
        } else {
            None
        }
    }

    pub(crate) fn legal_actions(&self) -> impl Iterator<Item = CombatAction> + use<'_> {
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
                        .collect::<Vec<_>>()
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
                        LegalTarget::OwnPlayer => vec![CombatAction::PlayCard {
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
                            .collect(),
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
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub(crate) fn apply<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        &self,
        action: CombatAction,
    ) -> Distribution {
        match action {
            CombatAction::PlayCard { card, target } => {
                let mut result = self.clone();

                result.player.hand.remove_card(card);

                // FIXME: state effects on cost
                let cost = card.get_cost();

                let result: Distribution = result.pay_cost(cost);
                // let result = Distribution::single_value(result);

                result.flat_map_simple(|state| state.play_card(card, target.map(Into::into), true))
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
                        (action)(distribution::full::Distribution::single_value(state), card).into()
                    }
                }
            }
            CombatAction::EndTurn => {
                let result = self.clone();

                result.handle_turn_transitions()
            }
        }
    }

    fn pay_cost<Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>>(
        mut self,
        cost: Cost,
    ) -> Distribution {
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

        Distribution::single_value(self)
    }

    fn draw_cards_for_turn<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
    ) -> Distribution {
        if self.turn_counter == 1 {
            let mut num_cards = 5;

            let mut innates = vec![];
            for (card, count) in self.player.draw_pile.iter_counts() {
                if card.has_innate() {
                    for _ in 0..count {
                        innates.push(*card);
                    }
                }
            }

            let mut res: Distribution = Distribution::single_value(self);
            for innate in innates {
                res = res.flat_map_simple(|state| state.draw_specific_card(innate));
                num_cards -= 1;
            }

            for _ in 0..num_cards {
                assert!(!res.is_empty());
                res = res.flat_map_simple(Self::draw_single_card::<Distribution>);
            }

            // This will produce lots of duplicated entries. Do reduce future work we dedup immediately
            res.dedup();

            return res;
        }

        let res: Distribution = self.draw_five_cards();

        assert!(res.len() > 0);
        res
    }

    fn draw_single_card<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        let state = if let Some(top_card) = self.player.draw_pile_top_card.take() {
            self.player.hand.add_card(top_card);
            Distribution::single_value(self)
        } else if self.player.draw_pile.is_empty() {
            if self.player.discard_pile.is_empty() {
                // Nothing to shuffle nor draw
                return Distribution::single_value(self);
            }

            // Shuffle the discard pile into the draw pile
            // TODO: Triggers

            self.player.draw_pile.append(&mut self.player.discard_pile);

            self.draw_single_card()
        } else {
            let cards = self.player.draw_pile.iter_counts().sorted_by_key(|v| v.0);

            Distribution::from_duplicates(cards.into_iter().map(|(card, count)| {
                let mut new = self.clone();
                new.player.draw_pile.remove_card(*card);
                new.player.hand.add_card(*card);
                (new, usize::from(count))
            }))
        };

        assert!(!state.is_empty());

        state.flat_map_simple(Self::on_draw_card)
    }

    fn shuffle_discard_pile<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        // TODO: Triggers

        self.player.draw_pile.append(&mut self.player.discard_pile);

        Distribution::single_value(self)
    }

    fn draw_five_cards<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
    ) -> Distribution {
        if self.player.draw_pile_top_card.is_some()
            || self.player.draw_pile.num_cards() + self.player.discard_pile.num_cards() < 5
        {
            // Just do the simple thing for now, to ensure we draw the top card
            let num_cards = 5;

            let mut res = Distribution::single_value(self);

            for _ in 0..num_cards {
                assert!(!res.is_empty());
                res = res.flat_map_simple(Self::draw_single_card);
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

            return Distribution::Inner::<Distribution>::equal_chance(indices.map(|v| {
                let mut state = Distribution::single_value(self.clone());

                for (index, card) in v.into_iter().enumerate() {
                    if index == num_draw_pile_cards {
                        // Shuffle
                        state = state.flat_map_simple(Self::shuffle_discard_pile);
                    }
                    state = state.flat_map_simple(|state| state.draw_specific_card(card));
                }

                state
            }))
            .flatten();
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

        Distribution::Inner::<Distribution>::equal_chance(indices.map(|[a, b, c, d, e]| {
            let mut state = Distribution::single_value(self.clone());

            state = state.flat_map_simple(|state| state.draw_specific_card(a));
            state = state.flat_map_simple(|state| state.draw_specific_card(b));
            state = state.flat_map_simple(|state| state.draw_specific_card(c));
            state = state.flat_map_simple(|state| state.draw_specific_card(d));
            state = state.flat_map_simple(|state| state.draw_specific_card(e));

            state
        }))
        .flatten()
    }

    fn handle_turn_transitions<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        let mut state = Distribution::single_value(self);

        state = state.flat_map_simple(Self::on_end_player_turn);

        state = state.flat_map_simple(Self::on_start_enemy_turn);

        state = state.flat_map_simple(Self::handle_enemy_actions);

        state = state.flat_map_simple(Self::on_end_enemy_turn);

        state = state.flat_map_simple(Self::on_start_player_turn);

        state
    }

    fn on_end_player_turn<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        if self.player.creature.block == 0 && self.relic_state.contains(RelicPrototype::Orichalcum)
        {
            self.player.creature.block += 6;
        }

        self.player.creature.block += u16::try_from(self.player.creature.statuses[Status::Plating])
            .expect("Plating cannot be negative");

        // TODO: Handle Etheral and Cards with "if in hand at end of turn"
        for (card, count) in self.player.hand.iter_counts() {
            match card.prototype {
                CardPrototype::Burn => {
                    for _ in 0..count {
                        let damage = 2;
                        let block_damage = min(self.player.creature.block, damage);
                        let unblocked = damage.saturating_sub(self.player.creature.block);
                        self.player.creature.block -= block_damage;
                        self.player.creature.hp -= unblocked;
                    }
                }
                _ => {}
            }
        }

        self.player.discard_pile.append(&mut self.player.hand);

        let mut status_diff: EnumMap<Status, i16> = EnumMap::default();
        for (status, count) in &mut self.player.creature.statuses {
            // if self.player.skip_next_duration_tick[status] {
            //     self.player.skip_next_duration_tick[status] = false;
            //     continue;
            // }

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

        Distribution::single_value(self)
    }

    fn on_start_enemy_turn<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
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

        Distribution::single_value(self)
    }

    fn handle_enemy_actions<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
    ) -> Distribution {
        // Enemy actions
        let mut state = Distribution::single_value(self);

        loop {
            let mut did_act = false;
            state = state.flat_map_simple(|mut state| {
                let alone = state.enemies.len() == 1;

                let enemy = state
                    .enemies
                    .iter_mut()
                    .find_position(|enemy| !enemy.has_acted_this_turn);

                if let Some((index, enemy)) = enemy {
                    enemy.has_acted_this_turn = true;

                    let action = enemy.prototype.get_moveset().eval(
                        &enemy.state_machine,
                        &enemy.creature.statuses,
                        alone,
                    );

                    did_act = true;

                    let mut state = Distribution::single_value(state);
                    for action in action.actions {
                        state = match action {
                            EnemyAction::Attack {
                                base_damage,
                                repeat,
                            } => {
                                for _ in 0..*repeat {
                                    state = state.flat_map_simple(|state| {
                                        state.apply_attack_damage(
                                            CharacterIndex::Enemy(index),
                                            *base_damage,
                                            CharacterIndex::Player,
                                        )
                                    });
                                }
                                state
                            }
                            EnemyAction::Block { amount } => state.flat_map_simple(|state| {
                                state.add_block_to_creature(CharacterIndex::Enemy(index), *amount)
                            }),
                            EnemyAction::ApplyStatusSelf { status, diff } => {
                                state.flat_map_simple(|state| {
                                    state.apply_status_change(
                                        CharacterIndex::Enemy(index),
                                        *status,
                                        *diff,
                                    )
                                })
                            }
                            EnemyAction::ApplyStatusPlayer { status, diff } => state
                                .flat_map_simple(|state| {
                                    state.apply_status_change(
                                        CharacterIndex::Player,
                                        *status,
                                        *diff,
                                    )
                                }),
                            EnemyAction::ShuffleCards { card, count, pile } => {
                                state.map(|mut state| {
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
                                })
                            }
                        };
                    }

                    state
                } else {
                    Distribution::single_value(state)
                }
            });

            if !did_act {
                break;
            }
        }

        // Next enemy intents
        state.flat_map_simple(|state| {
            state.for_all_enemies(|mut state, enemy_index| {
                let enemy = &mut state.enemies[enemy_index];
                let new_intent = enemy
                    .prototype
                    .get_moveset()
                    .advance(enemy.state_machine.clone(), &mut enemy.creature.statuses);

                new_intent
                    .map(|new_intent| {
                        let mut state = state.clone();

                        state.enemies[enemy_index].state_machine = new_intent;
                        state.enemies[enemy_index].has_acted_this_turn = false;

                        state
                    })
                    .into()
            })
        })
    }

    fn on_end_enemy_turn<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
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

        Distribution::single_value(self)
    }

    fn on_start_player_turn<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        self.turn_counter += 1;

        // Rampart Trigger
        for i in 0..self.enemies.len() {
            let rampart_amount = self.enemies[i].creature.statuses[Status::Rampart];
            if rampart_amount > 0 {
                // Find the target
                let target = self
                    .enemies
                    .iter_mut()
                    .enumerate()
                    .find_map(|(target, enemy)| (i != target).then_some(enemy));

                if let Some(target) = target {
                    target.creature.block +=
                        u16::try_from(rampart_amount).expect("Rampart must be positive");
                }
            }
        }

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

        let state = Distribution::single_value(self);

        let state = state.flat_map_simple(Self::draw_cards_for_turn);

        let state = state.flat_map_simple(|state| {
            // TODO: Relics
            if state.turn_counter == 1 && state.relic_state.contains(RelicPrototype::RingOfTheSnake)
            {
                let mut state = Distribution::single_value(state);
                for _ in 0..2 {
                    state = state.flat_map_simple(CombatState::draw_single_card);
                    state.dedup();
                }
                state
            } else {
                Distribution::single_value(state)
            }
        });

        state
    }

    fn draw_specific_card<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        card: Card,
    ) -> Distribution {
        assert!(self.player.draw_pile_top_card.is_none());

        self.player.draw_pile.remove_card(card);
        self.player.hand.add_card(card);

        self.on_draw_card()
    }

    fn on_draw_card<Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>>(
        self,
    ) -> Distribution {
        // Stuff like kingly kick (I think that gets cheaper when you draw it)

        assert!(self.player.creature.statuses[Status::CorrosiveWave] >= 0);
        let corrosive = self.player.creature.statuses[Status::CorrosiveWave].abs();

        let num_enemies = self.enemies.len();
        let mut state = Distribution::single_value(self);
        // TODO: Index shift problems

        if corrosive > 0 {
            // Apply Corrosive
            for enemy in 0..num_enemies {
                state = state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Enemy(enemy),
                        Status::Poison,
                        corrosive,
                    )
                });
            }
        }

        state
    }

    fn on_draw_non_draw_phase_card<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        // TODO: Stuff like speedster

        self.on_draw_card()
    }

    fn shuffle_discard_into_draw<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        self.player.draw_pile.append(&mut self.player.discard_pile);

        Distribution::single_value(self)
    }

    // The card must already be removed from whereever it came from, so we take it by value here to express that
    #[allow(clippy::needless_pass_by_value)]
    fn play_card<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
        card: Card,
        target: Option<usize>,
        is_raw_play: bool,
    ) -> Distribution {
        let mut state = Distribution::single_value(self);

        // FIXME: Afterimage is rough, since it really needs the playstack, which we do not have properly yet
        // state = state.flat_map_simple(|state| {
        //     let amount = state.player.creature.statuses[Status::Afterimage]
        //         .try_into()
        //         .unwrap();
        //     state.add_block_to_creature(CharacterIndex::Player, amount)
        // });

        let state = match card.prototype {
            CardPrototype::AscendersBane => unreachable!("Ascender's bane is unplayable"),
            CardPrototype::Dazed => unreachable!("Dazed is unplayable"),
            CardPrototype::Infection => unreachable!("Infection is unplayable"),
            CardPrototype::Wound => unreachable!("Wound is unplayable"),
            CardPrototype::Greed => unreachable!("Greed is unplayable"),
            CardPrototype::Burn => unreachable!("Burn is unplayable"),
            CardPrototype::Slimed => state.flat_map_simple(Self::draw_single_card),
            CardPrototype::Strike => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 9 } else { 6 };

                state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                })
            }
            CardPrototype::Backstab => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 15 } else { 11 };

                state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                })
            }
            CardPrototype::Defend => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 8 } else { 5 };

                state.flat_map_simple(|slf| {
                    let fasten = slf.player.creature.statuses[Status::Fasten];
                    slf.add_block_from_card(
                        base_amount + u16::try_from(fasten).expect("Fasten must be positive"),
                    )
                })
            }
            CardPrototype::Dash => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 13 } else { 10 };

                let state = state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                });

                state.flat_map_simple(|slf| slf.add_block_from_card(base_amount))
            }
            CardPrototype::Neutralize => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 4 } else { 3 };

                let state = state.flat_map_simple(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Weak,
                        if card.upgraded { 2 } else { 1 },
                    )
                });

                state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                })
            }
            CardPrototype::SuckerPunch => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 10 } else { 8 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                });

                state.flat_map_simple(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Weak,
                        if card.upgraded { 2 } else { 1 },
                    )
                })
            }
            CardPrototype::Squash => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 12 } else { 10 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                });

                state.flat_map_simple(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Vulnerable,
                        if card.upgraded { 3 } else { 2 },
                    )
                })
            }
            CardPrototype::Survivor => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 11 } else { 8 };

                state.flat_map_simple(|mut slf| {
                    let state = if slf.player.hand.num_cards() > 0 {
                        if slf.player.hand.num_cards() > 1 {
                            slf.player.waiting_for_decision =
                                Some(RequiredPlayerDecision::ChooseCardInHand {
                                    filter: |_| true,
                                    action: |state, card| {
                                        state.flat_map_simple(|mut state| {
                                            state.player.waiting_for_decision = None;
                                            state.discard_card(card)
                                        })
                                    },
                                });
                            Distribution::single_value(slf)
                        } else {
                            let card = *slf
                                .player
                                .hand
                                .iter()
                                .next()
                                .expect("Hand contains a single card");
                            slf.discard_card(card)
                        }
                    } else {
                        Distribution::single_value(slf)
                    };
                    state.flat_map_simple(|state| state.add_block_from_card(base_amount))
                })
            }
            CardPrototype::Acrobatics => {
                assert!(target.is_none());
                let cards = if card.upgraded { 4 } else { 3 };

                state.flat_map_simple(|slf| {
                    let mut state = Distribution::single_value(slf);

                    for _ in 0..cards {
                        state = state.flat_map_simple(CombatState::draw_single_card);
                    }

                    state = state.flat_map_simple(|mut slf| {
                        if slf.player.hand.num_cards() > 0 {
                            if slf.player.hand.num_cards() > 1 {
                                slf.player.waiting_for_decision =
                                    Some(RequiredPlayerDecision::ChooseCardInHand {
                                        filter: |_| true,
                                        action: |state, card| {
                                            state.flat_map_simple(|mut state| {
                                                state.player.waiting_for_decision = None;
                                                state.discard_card(card)
                                            })
                                        },
                                    });
                                Distribution::single_value(slf)
                            } else {
                                let card = *slf
                                    .player
                                    .hand
                                    .iter()
                                    .next()
                                    .expect("Hand contains a single card");
                                slf.discard_card(card)
                            }
                        } else {
                            Distribution::single_value(slf)
                        }
                    });

                    state
                })
            }
            CardPrototype::PoisonedStab => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 8 } else { 6 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map_simple(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Poison,
                        if card.upgraded { 4 } else { 3 },
                    )
                });

                state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                })
            }
            CardPrototype::Backflip => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 8 } else { 5 };

                let mut state = state.flat_map_simple(|slf| slf.add_block_from_card(base_amount));

                let cards = 2;

                for _ in 0..cards {
                    state = state.flat_map_simple(CombatState::draw_single_card);
                }
                state.dedup();
                state
            }
            CardPrototype::DeadlyPoison => {
                let target = target.unwrap();

                state.flat_map_simple(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Poison,
                        if card.upgraded { 7 } else { 5 },
                    )
                })
            }
            CardPrototype::CorrosiveWave => {
                assert!(target.is_none());
                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::CorrosiveWave,
                        if card.upgraded { 4 } else { 3 },
                    )
                })
            }
            CardPrototype::Burst => {
                assert!(target.is_none());
                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Burst,
                        if card.upgraded { 2 } else { 1 },
                    )
                })
            }
            CardPrototype::Footwork => {
                assert!(target.is_none());
                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Dexterity,
                        if card.upgraded { 3 } else { 2 },
                    )
                })
            }
            CardPrototype::Afterimage => {
                assert!(target.is_none());
                state.flat_map_simple(|state| {
                    state.apply_status_change(CharacterIndex::Player, Status::Afterimage, 1)
                })
            }
            CardPrototype::Accuracy => {
                assert!(target.is_none());
                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Accuracy,
                        if card.upgraded { 6 } else { 4 },
                    )
                })
            }
            CardPrototype::LegSweep => {
                let target = target.unwrap();

                let state = state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Enemy(target),
                        Status::Weak,
                        if card.upgraded { 3 } else { 2 },
                    )
                });

                state.flat_map_simple(|state| {
                    state.add_block_from_card(if card.upgraded { 14 } else { 11 })
                })
            }
            CardPrototype::PreciseCut => {
                let target = target.unwrap();

                let base_amount: usize = if card.upgraded { 16 } else { 13 };

                state.flat_map_simple(|state| {
                    let num_hand_cards = state.player.hand.num_cards();

                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        // FIXME: Strength and the negative are prob additive here, so this is overestimating the dmg slightly
                        (base_amount.saturating_sub(2 * num_hand_cards))
                            .try_into()
                            .unwrap(),
                        CharacterIndex::Enemy(target),
                    )
                })
            }
            CardPrototype::Anticipate => {
                assert!(target.is_none());

                let state = state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Dexterity,
                        if card.upgraded { 5 } else { 3 },
                    )
                });

                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Anticipate,
                        if card.upgraded { 5 } else { 3 },
                    )
                })
            }
            CardPrototype::NoxiousFumes => {
                assert!(target.is_none());

                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::NoxiousFumes,
                        if card.upgraded { 3 } else { 2 },
                    )
                })
            }
            CardPrototype::Fasten => {
                assert!(target.is_none());

                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Fasten,
                        if card.upgraded { 7 } else { 5 },
                    )
                })
            }
            CardPrototype::DodgeAndRoll => {
                assert!(target.is_none());

                let state = state.flat_map_simple(|state| {
                    state.add_block_from_card(if card.upgraded { 6 } else { 4 })
                });

                state.flat_map_simple(|state| {
                    let amount = state
                        .calculate_block(CharacterIndex::Player, if card.upgraded { 6 } else { 4 });
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::BlockNextTurn,
                        amount
                            .try_into()
                            .expect("More than i16::MAX block next turn"),
                    )
                })
            }
            CardPrototype::Shiv => {
                let target = target.unwrap();

                state.flat_map_simple(|state| {
                    let base_amount = if card.upgraded { 6 } else { 4 }
                        + u16::try_from(state.player.creature.statuses[Status::Accuracy])
                            .expect("Accuracy should always be positive");
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                })
            }
            CardPrototype::CloakAndDagger => {
                assert!(target.is_none());

                let state = state.flat_map_simple(|state| state.add_block_from_card(6));

                state.map(|mut state| {
                    for _ in 0..(if card.upgraded { 2 } else { 1 }) {
                        state.player.hand.add_card(Card {
                            prototype: CardPrototype::Shiv,
                            upgraded: false,
                            enchantment: None,
                        });
                    }

                    state
                })
            }
            CardPrototype::BladeDance => {
                assert!(target.is_none());

                state.map(|mut state| {
                    for _ in 0..(if card.upgraded { 4 } else { 3 }) {
                        state.player.hand.add_card(Card {
                            prototype: CardPrototype::Shiv,
                            upgraded: false,
                            enchantment: None,
                        });
                    }

                    state
                })
            }
            CardPrototype::LeadingStrike => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 10 } else { 7 };

                let state = state.flat_map_simple(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                });

                state.map(|mut state| {
                    state.player.hand.add_card(Card {
                        prototype: CardPrototype::Shiv,
                        upgraded: false,
                        enchantment: None,
                    });

                    state
                })
            }
            CardPrototype::Tracking => {
                assert!(target.is_none());

                state.flat_map_simple(|state| {
                    let change = if state.player.creature.statuses[Status::Tracking] == 0 {
                        2
                    } else {
                        1
                    };

                    state.apply_status_change(CharacterIndex::Player, Status::Tracking, change)
                })
            }
            CardPrototype::Haze => {
                assert!(target.is_none());

                state.flat_map_simple(|state| {
                    let poison_amount = if card.upgraded { 6 } else { 4 };

                    state.for_all_enemies(|state, enemy_index| {
                        state.apply_status_to_enemy(enemy_index, Status::Poison, poison_amount)
                    })
                })
            }
            CardPrototype::FranticEscape => {
                assert!(target.is_none());

                todo!("Sandpit is weird, and not implemented yet")
            }
            CardPrototype::Apotheosis => state.map(|mut state| {
                state.player.hand.upgrade_all();
                state.player.discard_pile.upgrade_all();
                state.player.draw_pile.upgrade_all();
                state.player.exhaust_pile.upgrade_all();
                state.player.play_pile.upgrade_all();

                state
            }),
            CardPrototype::Tactician => state.map(|mut state| {
                state.player.energy += if card.upgraded { 2 } else { 1 };

                state
            }),
            CardPrototype::Abrasive => {
                let state = state.flat_map_simple(|state| {
                    state.apply_status_change(CharacterIndex::Player, Status::Dexterity, 1)
                });

                state.flat_map_simple(|state| {
                    state.apply_status_change(
                        CharacterIndex::Player,
                        Status::Thorns,
                        if card.upgraded { 6 } else { 4 },
                    )
                })
            }
            CardPrototype::DaggerSpray => {
                let state = state.flat_map_simple(|state| {
                    state.for_all_enemies(|state, index| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            if card.upgraded { 6 } else { 4 },
                            CharacterIndex::Enemy(index),
                        )
                    })
                });

                state.flat_map_simple(|state| {
                    state.for_all_enemies(|state, index| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            if card.upgraded { 6 } else { 4 },
                            CharacterIndex::Enemy(index),
                        )
                    })
                })
            }
            CardPrototype::Ricochet => {
                assert!(target.is_none());

                let repeats = if card.upgraded { 5 } else { 4 };
                let base_dmg = 3;

                state.flat_map_simple(|state| {
                    let mut state = Distribution::single_value(state);

                    for _ in 0..repeats {
                        state = state.flat_map_simple(|state| {
                            if state.enemies.is_empty() {
                                Distribution::single_value(state)
                            } else {
                                Distribution::Inner::<Distribution>::equal_chance(
                                    (0..state.enemies.len()).map(|enemy| {
                                        let state = state.clone();
                                        state.apply_attack_damage(
                                            CharacterIndex::Player,
                                            base_dmg,
                                            CharacterIndex::Enemy(enemy),
                                        )
                                    }),
                                )
                                .flatten()
                            }
                        });
                    }

                    state
                })
            }
            CardPrototype::Peck => {
                let target = target.unwrap();

                let repeats = if card.upgraded { 4 } else { 3 };
                let base_dmg = 2;

                state.flat_map_simple(|state| {
                    state.repeat_single_enemy_cancel_if_dead(target, repeats, |state, enemy| {
                        state.apply_attack_damage(
                            CharacterIndex::Player,
                            base_dmg,
                            CharacterIndex::Enemy(enemy),
                        )
                    })
                })
            }
            CardPrototype::StormOfSteel => {
                assert!(target.is_none());

                state.flat_map_simple(|state| {
                    let num_cards = state.player.hand.num_cards();

                    let mut state = Distribution::single_value(state);

                    for _ in 0..num_cards {
                        state = state.flat_map_simple(|state| {
                            let card = *state.player.hand.iter().next().unwrap();
                            state.discard_card(card)
                        });
                    }

                    for _ in 0..num_cards {
                        state = state.flat_map_simple(CombatState::draw_single_card);
                    }

                    state
                })
            }
        };

        let mut state = state.flat_map_simple(Self::on_any_card_played);

        if is_raw_play && card.prototype.get_kind() == CardKind::Skill {
            state = state.flat_map_simple(|mut state| {
                if state.player.creature.statuses[Status::Burst] > 0 {
                    // TODO: What if the target is no longer valid????
                    state.player.creature.statuses[Status::Burst] -= 1;
                    state.play_card(card, target, false)
                } else {
                    Distribution::single_value(state)
                }
            });
        }

        if is_raw_play {
            if card.prototype.get_kind() == CardKind::Power {
                state
            } else if card.has_exhaust() {
                state.map(|mut state| {
                    state.player.exhaust_pile.add_card(card);
                    state
                })
            } else {
                state.map(|mut state| {
                    state.player.discard_pile.add_card(card);
                    state
                })
            }
        } else {
            state
        }
    }

    fn for_all_enemies<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
        fun: impl Fn(Self, usize) -> Distribution,
    ) -> Distribution {
        let num_enemies = self.enemies.len();
        let mut state = Distribution::single_value(self);

        // FIXME: Index shifts
        // FIXME: Rev is technically wrong, but it reduces the index shift issue (maybe?)
        for enemy_index in (0..num_enemies).rev() {
            state = state.flat_map_simple(|state| (fun)(state, enemy_index));
        }

        state
    }

    fn repeat_single_enemy_cancel_if_dead<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
        enemy_index: usize,
        repeats: usize,
        fun: impl Fn(Self, usize) -> Distribution,
    ) -> Distribution {
        let num_enemies = self.enemies.len();
        let mut state = Distribution::single_value(self);

        for _ in 0..repeats {
            state = state.flat_map_simple(|state| {
                if state.enemies.len() == num_enemies {
                    // Only do it if nothing has died yet
                    // TODO: This means this will stop if *anything* dies, which is wrong but close enough for now
                    (fun)(state, enemy_index)
                } else {
                    Distribution::single_value(state)
                }
            });
        }

        state
    }

    fn discard_card<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        card: Card,
    ) -> Distribution {
        self.player.hand.remove_card(card);

        if card.has_sly() {
            // FIXME: What about targeting???
            self.play_card(card, None, true)
        } else {
            self.player.discard_pile.add_card(card);
            Distribution::single_value(self)
        }
    }

    fn add_block_from_card<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        base_amount: u16,
    ) -> Distribution {
        let amount = self.calculate_block(CharacterIndex::Player, base_amount);

        if self.relic_state.get_state(RelicPrototype::Vanbrace) == Some(0) {
            self.player.creature.block += amount * 2;
            self.relic_state.set_state(RelicPrototype::Vanbrace, 1);
        } else {
            self.player.creature.block += amount;
        }

        // TODO: Triggers

        Distribution::single_value(self)
    }

    fn add_block_to_creature<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        creature: CharacterIndex,
        base_amount: u16,
    ) -> Distribution {
        let amount = self.calculate_block(creature, base_amount);

        match creature {
            CharacterIndex::Player => {
                self.player.creature.block += amount;
            }
            CharacterIndex::Enemy(index) => self.enemies[index].creature.block += amount,
        }

        // TODO: Triggers

        Distribution::single_value(self)
    }

    fn calculate_block(&self, creature: CharacterIndex, base_amount: u16) -> u16 {
        let status = match creature {
            CharacterIndex::Player => &self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &self.enemies[index].creature.statuses,
        };

        let amount = base_amount.saturating_add_signed(status[Status::Dexterity]);

        let amount = f32::from(amount);

        let amount = if status[Status::Frail] > 0 {
            amount * 0.75
        } else {
            amount
        };

        amount as u16
    }

    fn apply_attack_damage<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        source: CharacterIndex,
        base_amount: u16,
        target: CharacterIndex,
    ) -> Distribution {
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

        let target_thorns =
            u16::try_from(target_status[Status::Thorns]).expect("Thorns should be positive");

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
            CharacterIndex::Player => Distribution::single_value(self),
            CharacterIndex::Enemy(enemy_index) => self.on_enemy_lost_hp(enemy_index),
        };

        state = state.map(|mut state| {
            match source {
                CharacterIndex::Player => {
                    state.player.creature.hp -= target_thorns;
                }
                CharacterIndex::Enemy(index) => {
                    state.enemies[index].creature.hp -= target_thorns;
                }
            }
            state
        });

        if personal_hive > 0 {
            state = state.map(|mut state| {
                for _ in 0..personal_hive {
                    state.player.draw_pile.add_card(Card {
                        prototype: CardPrototype::Dazed,
                        upgraded: false,
                        enchantment: None,
                    });
                }
                state
            });
        }

        state
    }

    fn on_enemy_lost_hp<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        enemy_index: usize,
    ) -> Distribution {
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

        let dead = self.enemies.extract_if(.., |enemy| enemy.creature.hp == 0);

        for enemy in dead {
            if enemy.prototype == EnemyPrototype::ShrinkerBeetle {
                decrease_non_neg(&mut self.player.creature.statuses[Status::Shrink]);
            }
        }

        Distribution::single_value(self)
    }

    fn apply_status_to_enemy<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        self,
        enemy_index: usize,
        status: Status,
        diff: i16,
    ) -> Distribution {
        self.apply_status_change(CharacterIndex::Enemy(enemy_index), status, diff)
    }

    fn apply_status_change<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
        target: CharacterIndex,
        status: Status,
        diff: i16,
    ) -> Distribution {
        assert_ne!(diff, 0);

        let status_list = match target {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => {
                if let Some(enemy) = self.enemies.get_mut(index) {
                    &mut enemy.creature.statuses
                } else {
                    return Distribution::single_value(self);
                }
            }
        };

        if status.is_debuff() && status_list[Status::Artifact] > 0 {
            status_list[Status::Artifact] -= 1;
            return Distribution::single_value(self);
        }

        if target == CharacterIndex::Player && status.is_debuff() && status_list[status] == 0 {
            self.player.skip_next_duration_tick[status] = true;
        }

        status_list[status] += diff;

        Distribution::single_value(self)
    }

    fn on_any_card_played<
        Distribution: distribution::Distribution<Self, Inner<Self> = Distribution>,
    >(
        mut self,
    ) -> Distribution {
        Distribution::single_value(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player {
    pub hand: UnorderedCardSet,
    pub draw_pile: UnorderedCardSet,
    draw_pile_top_card: Option<Card>,
    discard_pile: UnorderedCardSet,
    exhaust_pile: UnorderedCardSet,

    // TODO: Unfortunately the real algorithm is muuch more complex
    play_pile: UnorderedCardSet,
    waiting_for_decision: Option<RequiredPlayerDecision>,

    orbs: Vec<Orb>,
    num_orb_slots: u8,

    energy: u8,
    stars: u8,

    pub creature: Creature,
    // This is taken from the game directly. I hate it
    skip_next_duration_tick: enum_map::EnumMap<Status, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RequiredPlayerDecision {
    ChooseCardInHand {
        filter: fn(Card) -> bool,
        action: fn(
            distribution::full::Distribution<CombatState>,
            Card,
        ) -> distribution::full::Distribution<CombatState>,
    },
}

impl Player {
    pub fn default() -> Self {
        Self {
            hand: vec![
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
            ]
            .into_iter()
            .collect(),
            draw_pile: vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Neutralize.get_normal_card(),
                CardPrototype::Survivor.get_normal_card(),
            ]
            .into_iter()
            .collect(),
            draw_pile_top_card: None,
            discard_pile: vec![].into_iter().collect(),
            exhaust_pile: vec![].into_iter().collect(),
            play_pile: vec![].into_iter().collect(),
            waiting_for_decision: None,
            orbs: vec![],
            num_orb_slots: 1,
            energy: 3,
            stars: 0,
            creature: Creature {
                hp: 70,
                max_hp: 70,
                block: 0,
                statuses: EnumMap::default(),
            },
            skip_next_duration_tick: EnumMap::default(),
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
    #[serde(rename = "SANDPIT_POWER")]
    Sandpit,
    #[serde(rename = "RAMPART_POWER")]
    Rampart,

    #[serde(rename = "THORNS_POWER")]
    Thorns,
    #[serde(rename = "AFTERIMAGE_POWER")]
    Afterimage,

    #[serde(rename = "MINION_POWER")]
    Minion,
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
            Status::Sandpit => false,
            Status::Thorns => false,
            Status::Rampart => false,
            Status::Afterimage => false,
            Status::Minion => false,
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

pub enum EnemyMoveSet {
    ConstantRotation {
        // TODO: static would be much better
        // rotation: &'static [EnemyMove],
        rotation: Vec<EnemyMove>,
    },
    RandomNoRepeat {
        weighted_options: Vec<(EnemyMove, u8)>,
    },
    Random {
        weighted_options: Vec<(EnemyMove, u8)>,
    },
    Prefix {
        prefixed_move: EnemyMove,
        after: Box<Self>,
    },
    IsAlone {
        alone: Box<Self>,
        not_alone: Box<Self>,
    },
}

impl EnemyMoveSet {
    pub fn eval(
        &self,
        state_machine: &EnemyStateMachine,
        status: &EnumMap<Status, i16>,
        is_alone: bool,
    ) -> EnemyMove {
        if state_machine.stunned > 0 || status[Status::Slumber] > 0 || status[Status::Asleep] > 0 {
            return EnemyMove { actions: &[] };
        }

        match self {
            EnemyMoveSet::ConstantRotation { rotation } => {
                rotation[state_machine.current_state % rotation.len()]
            }
            EnemyMoveSet::Random { weighted_options } => {
                weighted_options[state_machine.current_state].0
            }
            EnemyMoveSet::RandomNoRepeat { weighted_options } => {
                weighted_options[state_machine.current_state].0
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
                        is_alone,
                    )
                }
            }
            EnemyMoveSet::IsAlone { alone, not_alone } => {
                if is_alone {
                    alone.eval(state_machine, status, is_alone)
                } else {
                    not_alone.eval(state_machine, status, is_alone)
                }
            }
        }
    }

    fn advance(
        &self,
        mut state_machine: EnemyStateMachine,
        status: &mut EnumMap<Status, i16>,
    ) -> distribution::full::Distribution<EnemyStateMachine> {
        if state_machine.stunned > 0 {
            state_machine.stunned -= 1;
            return Distribution::single_value(state_machine);
        }

        if status[Status::Slumber] > 0 {
            status[Status::Slumber] -= 1;
            if status[Status::Slumber] == 0 {
                status[Status::Plating] = 0;
            }
            return Distribution::single_value(state_machine);
        }

        if status[Status::Asleep] > 0 {
            status[Status::Asleep] -= 1;
            if status[Status::Asleep] == 0 {
                status[Status::Plating] = 0;
            }
            return Distribution::single_value(state_machine);
        }

        match self {
            Self::ConstantRotation { .. } => {
                state_machine.current_state += 1;
                Distribution::single_value(state_machine)
            }
            Self::RandomNoRepeat { weighted_options } => {
                Distribution::from_duplicates(weighted_options.iter().enumerate().filter_map(
                    |(i, (_move, weight))| {
                        (i != state_machine.current_state).then_some((
                            EnemyStateMachine {
                                current_state: i,
                                stunned: 0,
                            },
                            usize::from(*weight),
                        ))
                    },
                ))
            }
            Self::Random { weighted_options } => {
                Distribution::from_duplicates(weighted_options.iter().enumerate().map(
                    |(i, (_move, weight))| {
                        (
                            EnemyStateMachine {
                                current_state: i,
                                stunned: 0,
                            },
                            usize::from(*weight),
                        )
                    },
                ))
            }
            Self::Prefix { .. } => {
                state_machine.current_state += 1;
                Distribution::single_value(state_machine)
            }
            Self::IsAlone { .. } => {
                state_machine.current_state += 1;
                Distribution::single_value(state_machine)
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
    KinFollower,
    KinPriest,
    BowlbugRock,
    BowlbugEgg,
    BowlbugNectar,
    BowlbugSilk,
    SlumberingBeetle,
    InfestedPrism,
    Entomancer,
    Chomper,
    TheInsatiable,
    LivingShield,
    TurretOperator,
    SlimedBerserker,
    MechaKnight,
    TheLost,
    TheForgotten,
    LeafSlimeM,
    TwigSlimeM,
    LeafSlimeS,
    TwigSlimeS,
    SnappingJaxfruit,
    Flyconid,
}

impl EnemyPrototype {
    #[allow(clippy::match_same_arms)]
    pub fn get_moveset(self) -> EnemyMoveSet {
        match self {
            Self::LeafSlimeS => EnemyMoveSet::RandomNoRepeat {
                weighted_options: vec![
                    (
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 3,
                                repeat: 1,
                            }],
                        },
                        1,
                    ),
                    (
                        EnemyMove {
                            actions: &[EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Slimed,
                                    upgraded: false,
                                    enchantment: None,
                                },
                                count: 1,
                                pile: Pile::Discard,
                            }],
                        },
                        1,
                    ),
                ],
            },
            Self::LeafSlimeM => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Slimed,
                                upgraded: false,
                                enchantment: None,
                            },
                            count: 2,
                            pile: Pile::Discard,
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
            Self::TwigSlimeS => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 4,
                        repeat: 1,
                    }],
                }],
            },
            // TODO: This cannot actually repeat the slimed move, twice in a row. This slightly changes the odds of future intents which could matter but is prob fine
            Self::TwigSlimeM => EnemyMoveSet::Random {
                weighted_options: vec![
                    (
                        EnemyMove {
                            actions: &[EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Slimed,
                                    upgraded: false,
                                    enchantment: None,
                                },
                                count: 1,
                                pile: Pile::Discard,
                            }],
                        },
                        1,
                    ),
                    (
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 11,
                                repeat: 1,
                            }],
                        },
                        2,
                    ),
                ],
            },
            Self::SnappingJaxfruit => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[
                        EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 1,
                        },
                        EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 2,
                        },
                    ],
                }],
            },
            Self::Flyconid => EnemyMoveSet::RandomNoRepeat {
                weighted_options: vec![
                    (
                        EnemyMove {
                            actions: &[EnemyAction::ApplyStatusPlayer {
                                status: Status::Vulnerable,
                                diff: 2,
                            }],
                        },
                        3,
                    ),
                    (
                        EnemyMove {
                            actions: &[
                                EnemyAction::Attack {
                                    base_damage: 8,
                                    repeat: 1,
                                },
                                EnemyAction::ApplyStatusPlayer {
                                    status: Status::Frail,
                                    diff: 2,
                                },
                            ],
                        },
                        2,
                    ),
                    (
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 11,
                                repeat: 1,
                            }],
                        },
                        1,
                    ),
                ],
            },

            Self::Nibbit => EnemyMoveSet::ConstantRotation {
                rotation: vec![
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
                rotation: vec![
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
            Self::ShrinkerBeetle => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusPlayer {
                        status: Status::Shrink,
                        diff: 1,
                    }],
                },
                after: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![
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
                }),
            },
            Self::Byrdonis => EnemyMoveSet::ConstantRotation {
                rotation: vec![
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
                after: Box::new(EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 10,
                        }],
                    },
                    after: Box::new(EnemyMoveSet::ConstantRotation {
                        rotation: vec![EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 15,
                                repeat: 1,
                            }],
                        }],
                    }),
                }),
            },
            Self::CubexConstruct => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusSelf {
                        status: Status::Strength,
                        diff: 2,
                    }],
                },
                after: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![
                        EnemyMove {
                            actions: &[
                                EnemyAction::Attack {
                                    base_damage: 7,
                                    repeat: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Strength,
                                    diff: 2,
                                },
                            ],
                        },
                        EnemyMove {
                            actions: &[
                                EnemyAction::Attack {
                                    base_damage: 7,
                                    repeat: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Strength,
                                    diff: 2,
                                },
                            ],
                        },
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 5,
                                repeat: 2,
                            }],
                        },
                    ],
                }),
            },
            Self::AxeRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 5,
                                repeat: 1,
                            },
                            EnemyAction::Block { amount: 5 },
                        ],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 5,
                                repeat: 1,
                            },
                            EnemyAction::Block { amount: 5 },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 12,
                            repeat: 1,
                        }],
                    },
                ],
            },
            Self::AssassinRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 11,
                        repeat: 1,
                    }],
                }],
            },
            Self::BruteRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 7,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 3,
                        }],
                    },
                ],
            },
            Self::CrossbowRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Block { amount: 3 }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 14,
                            repeat: 1,
                        }],
                    },
                ],
            },
            Self::TrackerRubyRaider => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusPlayer {
                        status: Status::Frail,
                        diff: 2,
                    }],
                },
                after: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 1,
                            repeat: 8,
                        }],
                    }],
                }),
            },
            Self::Vantom => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 7,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 6,
                            repeat: 2,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 27,
                                repeat: 1,
                            },
                            EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Wound,
                                    upgraded: false,
                                    enchantment: None,
                                },
                                count: 3,
                                pile: Pile::Discard,
                            },
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
            Self::KinFollower => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 5,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 2,
                            repeat: 2,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 2,
                        }],
                    },
                ],
            },
            Self::KinPriest => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 8,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Frail,
                                diff: 1,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 2,
                                repeat: 2,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Weak,
                                diff: 1,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 3,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 2,
                        }],
                    },
                ],
            },
            Self::BowlbugRock => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 15,
                        repeat: 1,
                    }],
                }],
            },
            Self::BowlbugEgg => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[
                        EnemyAction::Attack {
                            base_damage: 7,
                            repeat: 1,
                        },
                        EnemyAction::Block { amount: 7 },
                    ],
                }],
            },
            Self::BowlbugSilk => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusPlayer {
                            status: Status::Weak,
                            diff: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 2,
                        }],
                    },
                ],
            },
            Self::BowlbugNectar => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 3,
                        repeat: 1,
                    }],
                },
                after: Box::new(EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 15,
                        }],
                    },
                    after: Box::new(EnemyMoveSet::ConstantRotation {
                        rotation: vec![EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 3,
                                repeat: 1,
                            }],
                        }],
                    }),
                }),
            },
            Self::InfestedPrism => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 22,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 16,
                                repeat: 1,
                            },
                            EnemyAction::Block { amount: 16 },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 9,
                            repeat: 3,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Block { amount: 20 },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 4,
                            },
                        ],
                    },
                ],
            },
            Self::Entomancer => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 7,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 18,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::ApplyStatusSelf {
                                status: Status::PersonalHive,
                                diff: 1,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 1,
                            },
                        ],
                    },
                ],
            },
            Self::Chomper => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 8,
                            repeat: 2,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Dazed,
                                upgraded: false,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Discard,
                        }],
                    },
                ],
            },
            Self::SlumberingBeetle => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[
                        EnemyAction::Attack {
                            base_damage: 16,
                            repeat: 1,
                        },
                        EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 2,
                        },
                    ],
                }],
            },
            Self::TheInsatiable => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[
                        EnemyAction::ApplyStatusSelf {
                            status: Status::Sandpit,
                            diff: 4,
                        },
                        EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::FranticEscape,
                                upgraded: false,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Draw,
                        },
                        EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::FranticEscape,
                                upgraded: false,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Discard,
                        },
                    ],
                },
                after: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 8,
                                repeat: 2,
                            }],
                        },
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 28,
                                repeat: 1,
                            }],
                        },
                        EnemyMove {
                            actions: &[EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 2,
                            }],
                        },
                    ],
                }),
            },
            Self::LivingShield => EnemyMoveSet::IsAlone {
                not_alone: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 6,
                            repeat: 1,
                        }],
                    }],
                }),
                alone: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 16,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 3,
                            },
                        ],
                    }],
                }),
            },
            Self::TurretOperator => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 5,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 5,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 1,
                        }],
                    },
                ],
            },
            Self::SlimedBerserker => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Slimed,
                                upgraded: false,
                                enchantment: None,
                            },
                            count: 10,
                            pile: Pile::Discard,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 4,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Weak,
                                diff: 3,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 3,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 30,
                            repeat: 1,
                        }],
                    },
                ],
            },
            Self::MechaKnight => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 25,
                        repeat: 1,
                    }],
                },
                after: Box::new(EnemyMoveSet::ConstantRotation {
                    rotation: vec![
                        EnemyMove {
                            actions: &[EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Burn,
                                    upgraded: false,
                                    enchantment: None,
                                },
                                count: 4,
                                pile: Pile::Hand,
                            }],
                        },
                        EnemyMove {
                            actions: &[
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Strength,
                                    diff: 5,
                                },
                                EnemyAction::Block { amount: 15 },
                            ],
                        },
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 35,
                                repeat: 1,
                            }],
                        },
                    ],
                }),
            },
            Self::TheLost => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Strength,
                                diff: -2,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 2,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 2,
                        }],
                    },
                ],
            },
            Self::TheForgotten => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        actions: &[
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Dexterity,
                                diff: -2,
                            },
                            EnemyAction::Block { amount: 8 },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Dexterity,
                                diff: 2,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 15,
                            repeat: 1,
                        }],
                    },
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
    pub fn simple_test_combat_state() -> CombatState {
        CombatState {
            turn_counter: 0,
            player: Player::default(),
            enemies: vec![
                Enemy {
                    prototype: EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    state_machine: EnemyStateMachine {
                        current_state: 0,
                        ..Default::default()
                    },

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
                    state_machine: EnemyStateMachine {
                        current_state: 2,
                        ..Default::default()
                    },

                    has_taken_unblocked_damage_this_turn: false,
                    has_acted_this_turn: false,
                },
            ],
            relic_state: iter::empty().collect(),
        }
    }

    pub fn very_confused() -> CombatState {
        use crate::game_state::CardPrototype::*;
        use crate::game_state::EnemyPrototype::*;
        CombatState {
            turn_counter: 2,
            player: Player {
                hand: vec![
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
                .collect(),
                draw_pile: vec![
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
                .collect(),
                draw_pile_top_card: None,
                discard_pile: vec![].into_iter().collect(),
                exhaust_pile: vec![].into_iter().collect(),
                play_pile: vec![].into_iter().collect(),
                waiting_for_decision: None,
                orbs: vec![],
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
            enemies: vec![
                Enemy {
                    prototype: FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::from_fn(
                            |status| if status == Status::Strength { 7 } else { 0 },
                        ),
                    },
                    has_taken_unblocked_damage_this_turn: false,
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine {
                        current_state: 2,
                        ..Default::default()
                    },
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
                    state_machine: EnemyStateMachine {
                        current_state: 1,
                        ..Default::default()
                    },
                },
            ],
            relic_state: iter::empty().collect(),
        }
    }

    pub fn unneeded_blocking() -> CombatState {
        use crate::game_state::CardPrototype::*;
        use crate::game_state::EnemyPrototype::*;
        CombatState {
            turn_counter: 1,
            player: Player {
                hand: vec![
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
                .collect(),
                draw_pile: vec![].into_iter().collect(),
                draw_pile_top_card: None,
                discard_pile: vec![
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
                .collect(),
                exhaust_pile: vec![].into_iter().collect(),
                play_pile: vec![].into_iter().collect(),
                waiting_for_decision: None,
                orbs: vec![],
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
            enemies: vec![Enemy {
                prototype: FuzzyWurmCrawler,
                creature: Creature {
                    hp: 47,
                    max_hp: 57,
                    block: 0,
                    statuses: EnumMap::default(),
                },
                has_taken_unblocked_damage_this_turn: false,
                has_acted_this_turn: false,
                state_machine: EnemyStateMachine {
                    current_state: 1,
                    ..Default::default()
                },
            }],
            relic_state: iter::empty().collect(),
        }
    }

    pub fn transposition_test() -> CombatState {
        CombatState {
            turn_counter: 0,
            player: Player {
                hand: vec![
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                ]
                .into_iter()
                .collect(),
                draw_pile: vec![].into_iter().collect(),
                draw_pile_top_card: None,
                discard_pile: vec![].into_iter().collect(),
                exhaust_pile: vec![].into_iter().collect(),
                play_pile: vec![].into_iter().collect(),
                waiting_for_decision: None,
                orbs: vec![],
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
            enemies: vec![Enemy {
                prototype: EnemyPrototype::FuzzyWurmCrawler,
                creature: Creature {
                    hp: 55,
                    max_hp: 55,
                    block: 0,
                    statuses: EnumMap::default(),
                },
                state_machine: EnemyStateMachine {
                    current_state: 2,
                    ..Default::default()
                },

                has_acted_this_turn: false,
                has_taken_unblocked_damage_this_turn: false,
            }],
            relic_state: iter::empty().collect(),
        }
    }

    #[test]
    fn equality_for_card_sets() {
        assert_eq!(
            vec![CardPrototype::Strike.get_normal_card()]
                .into_iter()
                .collect::<UnorderedCardSet>(),
            vec![CardPrototype::Strike.get_normal_card()]
                .into_iter()
                .collect(),
        );

        assert_ne!(
            vec![CardPrototype::Strike.get_normal_card()]
                .into_iter()
                .collect::<UnorderedCardSet>(),
            vec![CardPrototype::Defend.get_normal_card()]
                .into_iter()
                .collect(),
        );

        assert_eq!(
            vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card()
            ]
            .into_iter()
            .collect::<UnorderedCardSet>(),
            vec![
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Strike.get_normal_card()
            ]
            .into_iter()
            .collect(),
        );

        let hash: HashSet<UnorderedCardSet, RandomState> = HashSet::from_iter(iter::once(
            vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
            ]
            .into_iter()
            .collect(),
        ));

        assert!(
            hash.contains(
                &vec![
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card()
                ]
                .into_iter()
                .collect()
            )
        );
    }
}
