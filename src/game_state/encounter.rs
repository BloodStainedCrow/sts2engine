use enum_map::EnumMap;
use itertools::Itertools;
use strum::EnumIter;

use crate::{
    distribution,
    game_state::{
        CharacterIndex, CombatState, Creature, Enemy, EnemyPrototype, EnemyStateMachine, Player,
        RelicPrototype, RunInfo, Status,
    },
};

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum EncounterPrototype {
    FuzzyWurmCrawler,
    SingleNibbit,
    DoubleNibbit,
    SlimesWeak,
    ShrinkerBeetle,
    Byrdonis,
    // Mr. Wriggles
    PhrogParasite,
    BygoneEffigy,
    SingleCubexConstruct,
    BeetleAndFuzzy,
    RubyRaiders,
    Vantom,
    TheKin,
    BowlbugsWeak,
    BowlbugsStrong,
    SoloTunneler,
    // TODO: Exoskeletons have rules like "after X always use Y", which I do not support yet
    // ExoskeletonEasy,
    SpinyToad,
    LouseProgenitor,
    InfestedPrism,
    // Mr. Beeeees!!!
    Entomancer,
    Chompers,
    SlumberParty,
    TheInsatiable,
    TurretOperator,
    SlimedBerserker,
    DevotedSculptor,
    OwlMagistrate,
    MechaKnight,
    SoulNexus,
    Knights,
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
    pub fn is_finished_implementing(self) -> bool {
        use Act::*;
        match self {
            EncounterPrototype::SoloTunneler => false,
            EncounterPrototype::TheLostAndForgotten => false,
            EncounterPrototype::SoulNexus => false,

            _ => true,
        }
    }

    pub fn get_act(self) -> Act {
        use Act::*;
        match self {
            EncounterPrototype::FuzzyWurmCrawler => Act1,
            EncounterPrototype::SingleNibbit => Act1,
            EncounterPrototype::DoubleNibbit => Act1,
            EncounterPrototype::SlimesWeak => Act1,
            EncounterPrototype::ShrinkerBeetle => Act1,
            EncounterPrototype::Byrdonis => Act1,
            EncounterPrototype::PhrogParasite => Act1,
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
            EncounterPrototype::LouseProgenitor => Act2,
            EncounterPrototype::SpinyToad => Act2,
            EncounterPrototype::InfestedPrism => Act2,
            EncounterPrototype::Entomancer => Act2,
            EncounterPrototype::Chompers => Act2,
            EncounterPrototype::SlumberParty => Act2,
            EncounterPrototype::TheInsatiable => Act2,
            EncounterPrototype::TurretOperator => Act3,
            EncounterPrototype::DevotedSculptor => Act3,
            EncounterPrototype::OwlMagistrate => Act3,
            EncounterPrototype::SlimedBerserker => Act3,
            EncounterPrototype::MechaKnight => Act3,
            EncounterPrototype::Knights => Act3,
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
            EncounterPrototype::PhrogParasite => true,
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
            EncounterPrototype::LouseProgenitor => false,
            EncounterPrototype::SpinyToad => false,
            EncounterPrototype::InfestedPrism => true,
            EncounterPrototype::Entomancer => true,
            EncounterPrototype::Chompers => false,
            EncounterPrototype::SlumberParty => false,
            EncounterPrototype::TheInsatiable => false,
            EncounterPrototype::TurretOperator => false,
            EncounterPrototype::DevotedSculptor => false,
            EncounterPrototype::OwlMagistrate => false,
            EncounterPrototype::SlimedBerserker => false,
            EncounterPrototype::MechaKnight => true,
            EncounterPrototype::Knights => true,
            EncounterPrototype::SoulNexus => true,
            EncounterPrototype::TheLostAndForgotten => false,
        }
    }
}

impl CombatState {
    pub(crate) fn get_starting_states<
        Distribution: 'static + distribution::Distribution<Self, Inner<Self> = Distribution> + std::fmt::Debug,
    >(
        encounter: EncounterPrototype,
        run_info: &RunInfo,

        mut enemy_max_hp_filter: impl FnMut(&[u16]) -> bool,
    ) -> Distribution {
        let state = Distribution::single_value(Self {
            turn_counter: 0,
            current_turn_side: super::CombatSide::Player,

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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                                        has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
                        });

                        state
                    }))
                });

                state
            }
            EncounterPrototype::PhrogParasite => {
                let hp = 61..=64;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::Infested] = 4;

                        state.enemies.push(Enemy {
                            prototype: EnemyPrototype::PhrogParasite,
                            creature: Creature {
                                hp,
                                max_hp: hp,
                                block: 0,
                                statuses: status,
                            },
                            has_acted_this_turn: false,
                            state_machine: EnemyStateMachine::default(),
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                                has_taken_unblocked_attack_damage_this_turn: false,
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
                                has_taken_unblocked_attack_damage_this_turn: false,
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
                                has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                        has_taken_unblocked_attack_damage_this_turn: false,
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
                        has_taken_unblocked_attack_damage_this_turn: false,
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
                        has_taken_unblocked_attack_damage_this_turn: false,
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
                                has_taken_unblocked_attack_damage_this_turn: false,
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
                                has_taken_unblocked_attack_damage_this_turn: false,
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
            EncounterPrototype::LouseProgenitor => {
                let hp = 134..=136;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::CurlUp] = 14;

                        state.enemies.push(Enemy {
                            prototype: EnemyPrototype::LouseProgenitor,
                            creature: Creature {
                                hp,
                                max_hp: hp,
                                block: 0,
                                statuses: status,
                            },
                            has_acted_this_turn: false,
                            state_machine: EnemyStateMachine::default(),
                            has_taken_unblocked_attack_damage_this_turn: false,
                        });

                        state
                    }))
                });

                state
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                            has_taken_unblocked_attack_damage_this_turn: false,
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
                                has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::SpinyToad => {
                let hp = 116..=119;

                let state = state.flat_map_simple(|state| {
                    Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        state.enemies.push(Enemy {
                            prototype: EnemyPrototype::SpinyToad,
                            creature: Creature {
                                hp,
                                max_hp: hp,
                                block: 0,
                                statuses: EnumMap::default(),
                            },
                            has_acted_this_turn: false,
                            state_machine: EnemyStateMachine::default(),
                            has_taken_unblocked_attack_damage_this_turn: false,
                        });

                        state
                    }))
                });

                state
            }
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::DevotedSculptor => state.map(|mut state| {
                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::DevotedSculptor,
                    creature: Creature {
                        hp: 162,
                        max_hp: 162,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::OwlMagistrate => state.map(|mut state| {
                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::OwlMagistrate,
                    creature: Creature {
                        hp: 234,
                        max_hp: 234,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::Knights => state.map(|mut state| {
                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::FlailKnight,
                    creature: Creature {
                        hp: 101,
                        max_hp: 101,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::SpectralKnight,
                    creature: Creature {
                        hp: 93,
                        max_hp: 93,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state.enemies.push(Enemy {
                    prototype: EnemyPrototype::MagiKnight,
                    creature: Creature {
                        hp: 82,
                        max_hp: 82,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
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
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
        };

        assert!(!state_with_enemy.is_empty());

        state_with_enemy.retain_no_chance_fix(|state| {
            (enemy_max_hp_filter)(
                &state
                    .enemies
                    .iter()
                    .map(|enemy| enemy.creature.max_hp)
                    .collect_vec(),
            )
        });
        let mut state = state_with_enemy.fix_odds();
        assert!(!state.is_empty(), "Did you adjust the max_hp filter???");

        assert!(state.all_unique());
        dbg!(state.len());

        if run_info
            .relic_state
            .contains(RelicPrototype::TeaOfDiscourtesy)
        {
            for _ in 0..2 {
                state = state.map(|mut state| {
                    state
                        .player
                        .draw_pile
                        .add_card(crate::CardPrototype::Dazed.get_normal_card());
                    state
                });
            }
        }

        // TODO: This means we instantiate #NumPossibleStartingHands GameStates.
        // This will likely blow up our RAM. Find a way to solve that
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

        if run_info.relic_state.contains(RelicPrototype::Anchor) {
            state = state
                .flat_map_simple(|state| state.add_block_to_creature(CharacterIndex::Player, 10));
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
}
