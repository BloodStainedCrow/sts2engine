use crate::combat_state::{
    EnemyAction, EnemyMove, EnemyMoveSet, Pile, Status,
    cards::{Card, CardPrototype},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum EnemyPrototype {
    Nibbit,
    FuzzyWurmCrawler,
    ShrinkerBeetle,
    Byrdonis,
    PhrogParasite,
    Wriggler,
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
    LouseProgenitor,
    SpinyToad,
    InfestedPrism,
    Entomancer,
    Chomper,
    TheInsatiable,
    LivingShield,
    TurretOperator,
    DevotedSculptor,
    OwlMagistrate,
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
    FlailKnight,
    SpectralKnight,
    MagiKnight,
    PunchConstruct,
    TorchHeadAmalgam,
    Queen,
    SoulNexus,
    Tunneler,
    Exoskeleton,
    Myte,
    TestSubject,
    Doormaker,
}

impl EnemyPrototype {
    #[allow(clippy::match_same_arms)]
    pub const fn get_moveset(self) -> EnemyMoveSet {
        match self {
            Self::LeafSlimeS => EnemyMoveSet::RandomNoRepeatEqualWeights {
                options: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 3,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Slimed,
                                upgraded: false,
                                energy_cost_offset: 0,
                                enchantment: None,
                            },
                            count: 1,
                            pile: Pile::Discard,
                        }],
                    },
                ],
            },
            Self::LeafSlimeM => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Slimed,
                                upgraded: false,
                                energy_cost_offset: 0,
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
                rotation: &[EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 4,
                        repeat: 1,
                    }],
                }],
            },
            // TODO: This cannot actually repeat the slimed move, twice in a row. This slightly changes the odds of future intents which could matter but is prob fine
            Self::TwigSlimeM => EnemyMoveSet::Random {
                weighted_options: &[
                    (
                        EnemyMove {
                            actions: &[EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Slimed,
                                    upgraded: false,
                                    energy_cost_offset: 0,
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
                rotation: &[EnemyMove {
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
                weighted_options: &[
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
                rotation: &[
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
                rotation: &[
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
                        diff: -1,
                    }],
                },
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[
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
            },
            Self::Byrdonis => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 17,
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
            Self::PhrogParasite => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Infection,
                                upgraded: false,
                                energy_cost_offset: 0,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Discard,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 4,
                        }],
                    },
                ],
            },
            Self::Wriggler => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 6,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Infection,
                                    upgraded: false,
                                    energy_cost_offset: 0,
                                    enchantment: None,
                                },
                                count: 1,
                                pile: Pile::Discard,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 2,
                            },
                        ],
                    },
                ],
            },
            Self::BygoneEffigy => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove { actions: &[] },
                after: &EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 10,
                        }],
                    },
                    after: &EnemyMoveSet::ConstantRotation {
                        rotation: &[EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 15,
                                repeat: 1,
                            }],
                        }],
                    },
                },
            },
            Self::CubexConstruct => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusSelf {
                        status: Status::Strength,
                        diff: 2,
                    }],
                },
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[
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
                },
            },
            Self::AxeRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: &[
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
                rotation: &[EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 11,
                        repeat: 1,
                    }],
                }],
            },
            Self::BruteRubyRaider => EnemyMoveSet::ConstantRotation {
                rotation: &[
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
                rotation: &[
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
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 1,
                            repeat: 8,
                        }],
                    }],
                },
            },
            Self::Vantom => EnemyMoveSet::ConstantRotation {
                rotation: &[
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
                                    energy_cost_offset: 0,
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
                rotation: &[
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
                rotation: &[
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
                                base_damage: 8,
                                repeat: 1,
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
                rotation: &[EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 15,
                        repeat: 1,
                    }],
                }],
            },
            Self::BowlbugEgg => EnemyMoveSet::ConstantRotation {
                rotation: &[EnemyMove {
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
                rotation: &[
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
                after: &EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: 15,
                        }],
                    },
                    after: &EnemyMoveSet::ConstantRotation {
                        rotation: &[EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 3,
                                repeat: 1,
                            }],
                        }],
                    },
                },
            },
            Self::LouseProgenitor => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 9,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Frail,
                                diff: 2,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Block { amount: 14 },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 5,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 14,
                            repeat: 1,
                        }],
                    },
                ],
            },
            Self::SpinyToad => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Thorns,
                            diff: 5,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 23,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Thorns,
                                diff: -5,
                            },
                        ],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 17,
                            repeat: 1,
                        }],
                    },
                ],
            },
            Self::InfestedPrism => EnemyMoveSet::ConstantRotation {
                rotation: &[
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
                rotation: &[
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
                rotation: &[
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
                                energy_cost_offset: 0,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Discard,
                        }],
                    },
                ],
            },
            Self::SlumberingBeetle => EnemyMoveSet::ConstantRotation {
                rotation: &[EnemyMove {
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
                                energy_cost_offset: 0,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Draw,
                        },
                        EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::FranticEscape,
                                upgraded: false,
                                energy_cost_offset: 0,
                                enchantment: None,
                            },
                            count: 3,
                            pile: Pile::Discard,
                        },
                    ],
                },
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[
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
                },
            },
            Self::LivingShield => EnemyMoveSet::IsAlone {
                not_alone: &EnemyMoveSet::ConstantRotation {
                    rotation: &[EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 6,
                            repeat: 1,
                        }],
                    }],
                },
                alone: &EnemyMoveSet::ConstantRotation {
                    rotation: &[EnemyMove {
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
                },
            },
            Self::TurretOperator => EnemyMoveSet::ConstantRotation {
                rotation: &[
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
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Slimed,
                                upgraded: false,
                                energy_cost_offset: 0,
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
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[
                        EnemyMove {
                            actions: &[EnemyAction::ShuffleCards {
                                card: Card {
                                    prototype: CardPrototype::Burn,
                                    upgraded: false,
                                    energy_cost_offset: 0,
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
                },
            },
            Self::TheLost => EnemyMoveSet::ConstantRotation {
                rotation: &[
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
                rotation: &[
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
            Self::DevotedSculptor => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[
                        EnemyAction::ApplyStatusSelf {
                            status: Status::Ritual,
                            diff: 9,
                        },
                        // FIXME: This is a hack to avoid issues with Ritual immediatly giving the strength.
                        EnemyAction::ApplyStatusSelf {
                            status: Status::Strength,
                            diff: -9,
                        },
                    ],
                },
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 12,
                            repeat: 1,
                        }],
                    }],
                },
            },
            Self::OwlMagistrate => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 16,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 4,
                            repeat: 6,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::ApplyStatusSelf {
                            status: Status::Soar,
                            diff: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 33,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Vulnerable,
                                diff: 4,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Soar,
                                diff: -1,
                            },
                        ],
                    },
                ],
            },
            // TODO: Technically, the buff move cannot be repeated. But this is an overapproximation,
            // which can only result in slightly worse play, not in desyncs.
            Self::FlailKnight => EnemyMoveSet::RandomEqualWeights {
                options: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 15,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 9,
                            repeat: 2,
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
            // NOTE(BSC): The wiki lied to me. Spectral Knight does *not* infact alternate between its attacks
            Self::SpectralKnight => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusPlayer {
                        status: Status::Hex,
                        diff: 2,
                    }],
                },
                // TODO: Technically the rules around repeating are complex, and some things are disallowed, but this is close enough
                after: &EnemyMoveSet::RandomEqualWeights {
                    options: &[
                        EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 15,
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
            },
            Self::MagiKnight => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[
                        EnemyAction::Attack {
                            base_damage: 6,
                            repeat: 1,
                        },
                        EnemyAction::Block { amount: 5 },
                    ],
                },
                after: &EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[EnemyAction::ApplyStatusPlayer {
                            status: Status::Dampen,
                            diff: 1,
                        }],
                    },
                    after: &EnemyMoveSet::ConstantRotation {
                        rotation: &[
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 10,
                                    repeat: 1,
                                }],
                            },
                            EnemyMove {
                                actions: &[EnemyAction::Block { amount: 5 }],
                            },
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 35,
                                    repeat: 1,
                                }],
                            },
                        ],
                    },
                },
            },
            Self::PunchConstruct => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::Block { amount: 10 }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 14,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 5,
                                repeat: 2,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Weak,
                                diff: 1,
                            },
                        ],
                    },
                ],
            },
            Self::TorchHeadAmalgam => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 18,
                        repeat: 1,
                    }],
                },
                after: &EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 18,
                            repeat: 1,
                        }],
                    },
                    after: &EnemyMoveSet::ConstantRotation {
                        rotation: &[
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 8,
                                    repeat: 3,
                                }],
                            },
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 14,
                                    repeat: 1,
                                }],
                            },
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 14,
                                    repeat: 1,
                                }],
                            },
                        ],
                    },
                },
            },
            Self::Queen => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::ApplyStatusPlayer {
                        status: Status::ChainsOfBinding,
                        diff: 3,
                    }],
                },
                after: &EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Frail,
                                diff: 99,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Weak,
                                diff: 99,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Vulnerable,
                                diff: 99,
                            },
                        ],
                    },
                    after: &EnemyMoveSet::IsAlone {
                        alone: &EnemyMoveSet::ConstantRotation {
                            rotation: &[
                                EnemyMove {
                                    actions: &[EnemyAction::ApplyStatusTeammate {
                                        status: Status::Strength,
                                        diff: 1,
                                    }],
                                },
                                EnemyMove {
                                    actions: &[EnemyAction::Block { amount: 20 }],
                                },
                            ],
                        },
                        not_alone: &EnemyMoveSet::ConstantRotation {
                            rotation: &[
                                EnemyMove {
                                    actions: &[EnemyAction::Attack {
                                        base_damage: 3,
                                        repeat: 5,
                                    }],
                                },
                                EnemyMove {
                                    actions: &[EnemyAction::Attack {
                                        base_damage: 15,
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
                        },
                    },
                },
            },
            Self::SoulNexus => EnemyMoveSet::RandomNoRepeatEqualWeights {
                options: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 29,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 6,
                            repeat: 4,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 18,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Vulnerable,
                                diff: 2,
                            },
                            EnemyAction::ApplyStatusPlayer {
                                status: Status::Weak,
                                diff: 2,
                            },
                        ],
                    },
                ],
            },

            Self::Tunneler => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 13,
                        repeat: 1,
                    }],
                },
                after: &EnemyMoveSet::Prefix {
                    prefixed_move: EnemyMove {
                        actions: &[
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Burrowed,
                                diff: 1,
                            },
                            EnemyAction::Block { amount: 32 },
                        ],
                    },
                    after: &EnemyMoveSet::ConstantRotation {
                        rotation: &[EnemyMove {
                            actions: &[EnemyAction::Attack {
                                base_damage: 23,
                                repeat: 1,
                            }],
                        }],
                    },
                },
            },
            // TODO: The real algorithm is more complex, but this is an overapproximation which will avoid desyncs and only slightly reduce playing srenth
            Self::Exoskeleton => EnemyMoveSet::RandomEqualWeights {
                options: &[
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 1,
                            repeat: 3,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 8,
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
            },
            Self::Myte => EnemyMoveSet::ConstantRotation {
                rotation: &[
                    EnemyMove {
                        actions: &[EnemyAction::ShuffleCards {
                            card: Card {
                                prototype: CardPrototype::Toxic,
                                upgraded: false,
                                energy_cost_offset: 0,
                                enchantment: None,
                            },
                            count: 2,
                            pile: Pile::Hand,
                        }],
                    },
                    EnemyMove {
                        actions: &[EnemyAction::Attack {
                            base_damage: 13,
                            repeat: 1,
                        }],
                    },
                    EnemyMove {
                        actions: &[
                            EnemyAction::Attack {
                                base_damage: 13,
                                repeat: 1,
                            },
                            EnemyAction::ApplyStatusSelf {
                                status: Status::Strength,
                                diff: 2,
                            },
                        ],
                    },
                ],
            },
            Self::TestSubject => EnemyMoveSet::MultipleSets {
                movesets: &[
                    EnemyMoveSet::ConstantRotation {
                        rotation: &[
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 20,
                                    repeat: 1,
                                }],
                            },
                            EnemyMove {
                                actions: &[
                                    EnemyAction::Attack {
                                        base_damage: 14,
                                        repeat: 1,
                                    },
                                    EnemyAction::ApplyStatusPlayer {
                                        status: Status::Vulnerable,
                                        diff: 1,
                                    },
                                ],
                            },
                        ],
                    },
                    EnemyMoveSet::ConstantRotation {
                        rotation: &[
                            // FIXME: This hits one more time for each time it runs. That is not currently possible yet
                            EnemyMove {
                                actions: &[
                                    EnemyAction::Attack {
                                        base_damage: 10,
                                        repeat: 3,
                                    },
                                    EnemyAction::AddAttackRepeats { count: 1 },
                                ],
                            },
                        ],
                    },
                    EnemyMoveSet::ConstantRotation {
                        rotation: &[
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 10,
                                    repeat: 3,
                                }],
                            },
                            EnemyMove {
                                actions: &[EnemyAction::Attack {
                                    base_damage: 45,
                                    repeat: 1,
                                }],
                            },
                            EnemyMove {
                                actions: &[
                                    EnemyAction::ShuffleCards {
                                        card: Card {
                                            prototype: CardPrototype::Burn,
                                            upgraded: false,
                                            energy_cost_offset: 0,
                                            enchantment: None,
                                        },
                                        count: 3,
                                        pile: Pile::Discard,
                                    },
                                    EnemyAction::ApplyStatusSelf {
                                        status: Status::Strength,
                                        diff: 2,
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
            EnemyPrototype::Doormaker => EnemyMoveSet::Prefix {
                prefixed_move: EnemyMove {
                    actions: &[
                        EnemyAction::Transform { new_hp: 489 },
                        EnemyAction::ApplyStatusSelf {
                            status: Status::Hunger,
                            diff: 1,
                        },
                    ],
                },
                after: &EnemyMoveSet::ConstantRotation {
                    rotation: &[
                        EnemyMove {
                            actions: &[
                                EnemyAction::Attack {
                                    base_damage: 30,
                                    repeat: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Scrutiny,
                                    diff: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Hunger,
                                    diff: -1,
                                },
                            ],
                        },
                        EnemyMove {
                            actions: &[
                                EnemyAction::Attack {
                                    base_damage: 24,
                                    repeat: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Grasp,
                                    diff: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Scrutiny,
                                    diff: -1,
                                },
                            ],
                        },
                        EnemyMove {
                            actions: &[
                                EnemyAction::Attack {
                                    base_damage: 10,
                                    repeat: 2,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Strength,
                                    diff: 3,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Hunger,
                                    diff: 1,
                                },
                                EnemyAction::ApplyStatusSelf {
                                    status: Status::Grasp,
                                    diff: -1,
                                },
                            ],
                        },
                    ],
                },
            },
        }
    }
}
