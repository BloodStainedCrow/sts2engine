use std::borrow::Borrow;

use enum_map::EnumMap;
use itertools::{Itertools, iproduct};
use strum::EnumIter;

use crate::combat_state::relics::FullRelicState;
use crate::distribution::DistributionFamily;
use crate::run_state::ActPrototype;
use crate::run_state::encounter_generation::EncounterKind;
use crate::{
    combat_state::{
        CharacterIndex, CombatState, Creature, Enemy, EnemyStateMachine, Player, RelicPrototype,
        RunInfo, Status, cards::Card, enemy::EnemyPrototype,
    },
    distribution::Distribution,
};

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
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
    ExoskeletonWeak,
    ExoskeletonStrong,
    Mytes,
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
    ConstructGang,
    Queen,
    TestSubject,
    Doormaker,
}

impl EncounterPrototype {
    pub fn is_finished_implementing(self) -> bool {
        match self {
            // Returning the status on death
            EncounterPrototype::TheLostAndForgotten => false,

            // Chains of Binding Status
            // Teammate move crash if minion dies to poison
            EncounterPrototype::Queen => false,

            _ => true,
        }
    }

    pub fn get_act(self) -> ActPrototype {
        use ActPrototype::*;
        match self {
            EncounterPrototype::FuzzyWurmCrawler => Overgrowth,
            EncounterPrototype::SingleNibbit => Overgrowth,
            EncounterPrototype::DoubleNibbit => Overgrowth,
            EncounterPrototype::SlimesWeak => Overgrowth,
            EncounterPrototype::ShrinkerBeetle => Overgrowth,
            EncounterPrototype::Byrdonis => Overgrowth,
            EncounterPrototype::PhrogParasite => Overgrowth,
            EncounterPrototype::BygoneEffigy => Overgrowth,
            EncounterPrototype::SingleCubexConstruct => Overgrowth,
            EncounterPrototype::BeetleAndFuzzy => Overgrowth,
            EncounterPrototype::RubyRaiders => Overgrowth,
            EncounterPrototype::JaxfruitAndFlyconid => Overgrowth,
            EncounterPrototype::Vantom => Overgrowth,
            EncounterPrototype::TheKin => Overgrowth,
            EncounterPrototype::BowlbugsWeak => Hive,
            EncounterPrototype::BowlbugsStrong => Hive,
            EncounterPrototype::ExoskeletonWeak => Hive,
            EncounterPrototype::ExoskeletonStrong => Hive,
            EncounterPrototype::SoloTunneler => Hive,
            EncounterPrototype::LouseProgenitor => Hive,
            EncounterPrototype::SpinyToad => Hive,
            EncounterPrototype::InfestedPrism => Hive,
            EncounterPrototype::Entomancer => Hive,
            EncounterPrototype::Chompers => Hive,
            EncounterPrototype::Mytes => Hive,
            EncounterPrototype::SlumberParty => Hive,
            EncounterPrototype::TheInsatiable => Hive,
            EncounterPrototype::TurretOperator => Glory,
            EncounterPrototype::DevotedSculptor => Glory,
            EncounterPrototype::OwlMagistrate => Glory,
            EncounterPrototype::SlimedBerserker => Glory,
            EncounterPrototype::MechaKnight => Glory,
            EncounterPrototype::Knights => Glory,
            EncounterPrototype::SoulNexus => Glory,
            EncounterPrototype::TheLostAndForgotten => Glory,
            EncounterPrototype::ConstructGang => Glory,
            EncounterPrototype::Queen => Glory,
            EncounterPrototype::TestSubject => Glory,
            EncounterPrototype::Doormaker => Glory,
        }
    }

    #[allow(clippy::match_same_arms)]
    pub fn get_kind(self) -> EncounterKind {
        use EncounterKind::*;
        match self {
            EncounterPrototype::FuzzyWurmCrawler => Weak,
            EncounterPrototype::SingleNibbit => Weak,
            EncounterPrototype::DoubleNibbit => Normal,
            EncounterPrototype::SlimesWeak => Weak,
            EncounterPrototype::ShrinkerBeetle => Weak,
            EncounterPrototype::Byrdonis => Elite,
            EncounterPrototype::PhrogParasite => Elite,
            EncounterPrototype::BygoneEffigy => Elite,
            EncounterPrototype::SingleCubexConstruct => Normal,
            EncounterPrototype::BeetleAndFuzzy => Normal,
            EncounterPrototype::RubyRaiders => Normal,
            EncounterPrototype::JaxfruitAndFlyconid => Normal,
            EncounterPrototype::Vantom => Boss,
            EncounterPrototype::TheKin => Boss,
            EncounterPrototype::BowlbugsWeak => Weak,
            EncounterPrototype::BowlbugsStrong => Normal,
            EncounterPrototype::SoloTunneler => Weak,
            EncounterPrototype::LouseProgenitor => Normal,
            EncounterPrototype::SpinyToad => Normal,
            EncounterPrototype::InfestedPrism => Elite,
            EncounterPrototype::Entomancer => Elite,
            EncounterPrototype::Chompers => Normal,
            EncounterPrototype::SlumberParty => Normal,
            EncounterPrototype::TheInsatiable => Boss,
            EncounterPrototype::TurretOperator => Weak,
            EncounterPrototype::DevotedSculptor => Weak,
            EncounterPrototype::OwlMagistrate => Normal,
            EncounterPrototype::SlimedBerserker => Normal,
            EncounterPrototype::MechaKnight => Elite,
            EncounterPrototype::Knights => Elite,
            EncounterPrototype::SoulNexus => Elite,
            EncounterPrototype::TheLostAndForgotten => Normal,
            EncounterPrototype::ConstructGang => Normal,
            EncounterPrototype::Queen => Boss,
            EncounterPrototype::ExoskeletonWeak => Weak,
            EncounterPrototype::ExoskeletonStrong => Normal,
            EncounterPrototype::Mytes => Normal,
            EncounterPrototype::TestSubject => Boss,
            EncounterPrototype::Doormaker => Boss,
        }
    }
}

impl CombatState {
    pub(crate) fn get_starting_states<
        Family: DistributionFamily,
        R: Borrow<FullRelicState>,
        D: Borrow<[Card]>,
    >(
        encounter: EncounterPrototype,
        run_info: &RunInfo<R, D>,

        mut enemy_max_hp_filter: impl FnMut(&[u16]) -> bool,
    ) -> Family::Distribution<Self> {
        let state = Family::Distribution::single_value(Self {
            turn_counter: 0,
            died_to_sandpit: false,
            current_turn_side: super::CombatSide::Player,

            player: Box::new(Player {
                hand: vec![].into_iter().collect(),
                draw_pile: run_info.deck.borrow().iter().copied().collect(),
                draw_pile_top_card: None,
                discard_pile: vec![].into_iter().collect(),
                exhaust_pile: vec![].into_iter().collect(),
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
            }),
            enemies: vec![].into(),

            relic_state: *run_info.relic_state.borrow(),
        });

        assert!(state.all_unique());

        let mut state_with_enemy = match encounter {
            EncounterPrototype::FuzzyWurmCrawler => {
                let hp = 55..=57;

                let state = state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hps.clone().map(|(first_hp, second_hp)| {
                        let mut state = state.clone();

                        state.enemies.add_enemy(Enemy {
                            prototype: EnemyPrototype::Nibbit,
                            creature: Creature {
                                hp: first_hp,
                                max_hp: first_hp,
                                block: 0,
                                statuses: EnumMap::default(),
                            },
                            has_acted_this_turn: false,
                            state_machine: EnemyStateMachine {
                                current_state: 1,
                                stunned: 0,
                                bonus_attack_repeats: 0,
                            },
                            has_taken_unblocked_attack_damage_this_turn: false,
                        });

                        state.enemies.add_enemy(Enemy {
                            prototype: EnemyPrototype::Nibbit,
                            creature: Creature {
                                hp: second_hp,
                                max_hp: second_hp,
                                block: 0,
                                statuses: EnumMap::default(),
                            },
                            has_acted_this_turn: false,
                            state_machine: EnemyStateMachine {
                                current_state: 2,
                                stunned: 0,
                                bonus_attack_repeats: 0,
                            },
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
                    Family::Distribution::equal_chance(
                        typ_and_hp_action
                            .clone()
                            .cartesian_product([false, true])
                            .map(|(mut enemies, swap)| {
                                let mut state = state.clone();

                                if swap {
                                    enemies.reverse();
                                }

                                for (enemy, hp, starting_action) in enemies {
                                    state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        state.enemies.add_enemy(Enemy {
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
                let hp = 81..=84;

                let state = state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::Territorial] = 1;

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::Infested] = 4;

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::Slow] = 1;

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::Artifact] = 1;

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|(beetle_hp, fuzzy_hp)| {
                        let mut state = state.clone();

                        state.enemies.add_enemy(Enemy {
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

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(
                        |((jax_hp, flyconid_hp), current_state)| {
                            let mut state = state.clone();

                            state.enemies.add_enemy(Enemy {
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

                            state.enemies.add_enemy(Enemy {
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
                                    bonus_attack_repeats: 0,
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

                let state = state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(typ_and_hp.clone().map(|enemies| {
                        let mut state = state.clone();

                        for (enemy, hp) in enemies {
                            state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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
                let hp = (58..=59)
                    .cartesian_product(58..=59)
                    .cartesian_product([false, true].into_iter());

                Family::Distribution::equal_chance(hp.map(|((first, second), swap)| {
                    let mut state = state.clone();

                    let mut follower = EnumMap::default();
                    follower[Status::Minion] = 1;

                    state.enemies.add_enemy(Enemy {
                        prototype: EnemyPrototype::KinFollower,
                        creature: Creature {
                            hp: first,
                            max_hp: first,
                            block: 0,
                            statuses: follower.clone(),
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_attack_damage_this_turn: false,
                    });

                    state.enemies.add_enemy(Enemy {
                        prototype: EnemyPrototype::KinFollower,
                        creature: Creature {
                            hp: second,
                            max_hp: second,
                            block: 0,
                            statuses: follower,
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine {
                            current_state: 2,
                            stunned: 0,
                            bonus_attack_repeats: 0,
                        },
                        has_taken_unblocked_attack_damage_this_turn: false,
                    });

                    if swap {
                        state.enemies.enemies.rotate_left(1);
                    }

                    state.enemies.add_enemy(Enemy {
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

                let state = state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(typ_and_hp.clone().map(|enemies| {
                        let mut state = state.clone();

                        let mut unbalanced = EnumMap::default();
                        unbalanced[Status::Imbalanced] = 1;

                        for (enemy, hp) in enemies {
                            state.enemies.add_enemy(Enemy {
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

                let state = state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(typ_and_hp.clone().map(|enemies| {
                        let mut state = state.clone();

                        let mut unbalanced = EnumMap::default();
                        unbalanced[Status::Imbalanced] = 1;

                        for (enemy, hp) in enemies {
                            state.enemies.add_enemy(Enemy {
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
                let state = state.map(|mut state| {
                    state.enemies.add_enemy(Enemy {
                        prototype: EnemyPrototype::Tunneler,
                        creature: Creature {
                            hp: 87,
                            max_hp: 87,
                            block: 0,
                            statuses: EnumMap::default(),
                        },
                        has_acted_this_turn: false,
                        state_machine: EnemyStateMachine::default(),
                        has_taken_unblocked_attack_damage_this_turn: false,
                    });

                    state
                });

                state
            }
            EncounterPrototype::LouseProgenitor => {
                let hp = 134..=136;

                let state = state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::CurlUp] = 14;

                        state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|(first, second)| {
                        let mut state = state.clone();

                        let mut status = EnumMap::default();

                        status[Status::Artifact] = 2;

                        state.enemies.add_enemy(Enemy {
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

                        state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|enemies| {
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

                            state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
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
                    Family::Distribution::equal_chance(hp.clone().map(|hp| {
                        let mut state = state.clone();

                        state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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

                state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::SoulNexus,
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
            EncounterPrototype::TheLostAndForgotten => state.map(|mut state| {
                let mut the_lost = EnumMap::default();
                todo!();
                // the_lost[Status::PossessStrength] = 1;

                let mut the_forgotten = EnumMap::default();
                todo!();
                // the_lost[Status::PossessSpeed] = 1;

                state.enemies.add_enemy(Enemy {
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
                state.enemies.add_enemy(Enemy {
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
            EncounterPrototype::ConstructGang => state.map(|mut state| {
                let mut arti = EnumMap::default();
                arti[Status::Artifact] += 1;

                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::PunchConstruct,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: arti,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });
                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::CubexConstruct,
                    creature: Creature {
                        hp: 65,
                        max_hp: 65,
                        block: 0,
                        statuses: arti,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });
                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::CubexConstruct,
                    creature: Creature {
                        hp: 65,
                        max_hp: 65,
                        block: 0,
                        statuses: arti,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::Queen => state.map(|mut state| {
                let mut minion = EnumMap::default();
                minion[Status::Minion] += 1;

                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::TorchHeadAmalgam,
                    creature: Creature {
                        hp: 199,
                        max_hp: 199,
                        block: 0,
                        statuses: minion,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });
                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::Queen,
                    creature: Creature {
                        hp: 400,
                        max_hp: 400,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::ExoskeletonWeak => {
                let hps = iproduct!(24..=28, 24..=28, 24..=28).map(Into::into);

                state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(hps.clone().map(|hps: [u16; 3]| {
                        let mut state = state.clone();

                        let mut exo = EnumMap::default();
                        exo[Status::HardToKill] = 9;

                        for (i, hp) in hps.into_iter().enumerate() {
                            state.enemies.add_enemy(Enemy {
                                prototype: EnemyPrototype::Exoskeleton,
                                creature: Creature {
                                    hp,
                                    max_hp: hp,
                                    block: 0,
                                    statuses: exo,
                                },
                                has_acted_this_turn: false,
                                state_machine: EnemyStateMachine {
                                    stunned: 0,
                                    current_state: i.try_into().unwrap(),
                                    bonus_attack_repeats: 0,
                                },
                                has_taken_unblocked_attack_damage_this_turn: false,
                            });
                        }

                        state
                    }))
                })
            }
            EncounterPrototype::ExoskeletonStrong => {
                let hps = iproduct!(24..=28, 24..=28, 24..=28, 24..=28)
                    .map(Into::into)
                    .cartesian_product([0, 1]);

                state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(hps.clone().map(
                        |(hps, fourth): ([u16; 4], usize)| {
                            let mut state = state.clone();

                            let mut exo = EnumMap::default();
                            exo[Status::HardToKill] = 9;

                            for (mut i, hp) in hps.into_iter().enumerate() {
                                if i == 3 {
                                    i = fourth;
                                }

                                state.enemies.add_enemy(Enemy {
                                    prototype: EnemyPrototype::Exoskeleton,
                                    creature: Creature {
                                        hp,
                                        max_hp: hp,
                                        block: 0,
                                        statuses: exo,
                                    },
                                    has_acted_this_turn: false,
                                    state_machine: EnemyStateMachine {
                                        stunned: 0,
                                        current_state: i.try_into().unwrap(),
                                        bonus_attack_repeats: 0,
                                    },
                                    has_taken_unblocked_attack_damage_this_turn: false,
                                });
                            }

                            state
                        },
                    ))
                })
            }
            EncounterPrototype::Mytes => {
                let hps = iproduct!(24..=28, 24..=28).map(Into::into);

                state.flat_map_simple(|state| {
                    Family::Distribution::equal_chance(hps.clone().map(|hps: [u16; 2]| {
                        let mut state = state.clone();

                        for (i, hp) in hps.into_iter().enumerate() {
                            state.enemies.add_enemy(Enemy {
                                prototype: EnemyPrototype::Myte,
                                creature: Creature {
                                    hp,
                                    max_hp: hp,
                                    block: 0,
                                    statuses: EnumMap::default(),
                                },
                                has_acted_this_turn: false,
                                state_machine: EnemyStateMachine {
                                    stunned: 0,
                                    // Map 0 to 0 and 1 to 2
                                    current_state: (i + i).try_into().unwrap(),
                                    bonus_attack_repeats: 0,
                                },
                                has_taken_unblocked_attack_damage_this_turn: false,
                            });
                        }

                        state
                    }))
                })
            }
            EncounterPrototype::TestSubject => state.map(|mut state| {
                let mut adaptable = EnumMap::default();
                adaptable[Status::Adaptable] += 2;
                adaptable[Status::Enrage] += 2;

                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::TestSubject,
                    creature: Creature {
                        hp: 100,
                        max_hp: 100,
                        block: 0,
                        statuses: adaptable,
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine::default(),
                    has_taken_unblocked_attack_damage_this_turn: false,
                });

                state
            }),
            EncounterPrototype::Doormaker => state.map(|mut state| {
                state.enemies.add_enemy(Enemy {
                    prototype: EnemyPrototype::Doormaker,
                    creature: Creature {
                        hp: u16::MAX,
                        max_hp: u16::MAX,
                        block: 0,
                        statuses: EnumMap::default(),
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

        if run_info
            .relic_state
            .borrow()
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
        let mut state = state.flat_map_simple(Self::on_start_player_turn::<Family>);
        assert!(!state.is_empty());

        // Innate cards
        state.retain_no_chance_fix(|state| {
            state.player.draw_pile.iter().all(|card| !card.has_innate())
        });
        let mut state = state.fix_odds();
        assert!(!state.is_empty());

        let mut state = if encounter.get_kind() == EncounterKind::Elite
            && run_info
                .relic_state
                .borrow()
                .contains(RelicPrototype::BoomingConch)
        {
            for _ in 0..2 {
                state = state.flat_map_simple(CombatState::draw_single_card::<Family>);
            }
            state
        } else {
            state
        };

        state.dedup();

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::OddlySmoothStone)
        {
            state = state.flat_map_simple(|state| {
                state.apply_status_change::<Family>(CharacterIndex::Player, Status::Dexterity, 1)
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::Gorget)
        {
            state = state.flat_map_simple(|state| {
                state.apply_status_change::<Family>(CharacterIndex::Player, Status::Plating, 4)
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::Vajra)
        {
            state = state.flat_map_simple(|state| {
                state.apply_status_change::<Family>(CharacterIndex::Player, Status::Strength, 1)
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::BronzeScales)
        {
            state = state.flat_map_simple(|state| {
                state.apply_status_change::<Family>(CharacterIndex::Player, Status::Thorns, 3)
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::BagOfPreparation)
        {
            for _ in 0..2 {
                state = state.flat_map_simple(Self::draw_single_card::<Family>);
            }
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::Anchor)
        {
            state = state.flat_map_simple(|state| {
                state.creature_add_block_to_itself::<Family>(CharacterIndex::Player, 10)
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::BagOfMarbles)
        {
            state = state.flat_map_simple(|state| {
                state.for_all_enemies::<Family>(|state, enemy| {
                    state.apply_status_change::<Family>(
                        CharacterIndex::Enemy(enemy),
                        Status::Vulnerable,
                        1,
                    )
                })
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::RedMask)
        {
            state = state.flat_map_simple(|state| {
                state.for_all_enemies::<Family>(|state, enemy| {
                    state.apply_status_change::<Family>(
                        CharacterIndex::Enemy(enemy),
                        Status::Weak,
                        1,
                    )
                })
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::TwistedFunnel)
        {
            state = state.flat_map_simple(|state| {
                state.for_all_enemies::<Family>(|state, enemy| {
                    state.apply_status_change::<Family>(
                        CharacterIndex::Enemy(enemy),
                        Status::Poison,
                        4,
                    )
                })
            });
        }

        if let Some(v) = run_info
            .relic_state
            .borrow()
            .get_state(RelicPrototype::Girya)
        {
            state = state.flat_map_simple(|state| {
                state.apply_status_change::<Family>(
                    CharacterIndex::Player,
                    Status::Strength,
                    i16::from(v),
                )
            });
        }

        if run_info
            .relic_state
            .borrow()
            .contains(RelicPrototype::Bellows)
        {
            state = state.map(|mut state| {
                state.player.hand.upgrade_all();
                state
            });
        }

        // assert!(state.entries.iter().map(|(v, _)| v).all_unique());
        assert!(!state.is_empty());

        state.dedup();

        // assert!(state.entries.iter().map(|(v, _)| v).all_unique());

        state.dedup();
        assert!(!state.is_empty());

        state
    }
}
