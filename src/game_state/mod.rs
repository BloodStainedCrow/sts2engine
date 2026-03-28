use std::cmp::max;

use enum_map::{Enum, EnumMap};
use itertools::Itertools;
use std::hash::Hash;

use crate::game_state::cards::{Card, CardPrototype, Cost, CostVal, LegalTarget, UnorderedCardSet};
use crate::{combat_action::CombatAction, distribution::Distribution};

pub(crate) mod cards;

struct RunState {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CombatState {
    pub turn_counter: u8,

    pub player: Player,

    pub enemies: Vec<Enemy>,
}

#[derive(Debug, Clone, Copy)]
pub struct PostCombatState {
    pub turn_counter: u8,

    pub hp: u16,
    pub max_hp: u16,
    // I use an array of bools here to make this cheaper to clone
    pub potions_used: [bool; 10],
    // TODO
    // lost_cards: Vec<()>,
    pub bonus_card_rewards: u8,
}

#[derive(Debug, Clone, Copy)]
enum CharacterIndex {
    Player,
    Enemy(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum EncounterPrototype {
    FuzzyWurmCrawler,
    SingleNibbit,
    SlimesWeak,
    ShrinkerBeetle,
}

// TODO:
pub struct RunInfo {
    pub hp: u16,
    pub deck: Vec<Card>,
}

impl CombatState {
    pub(crate) fn get_starting_states(
        encounter: EncounterPrototype,
        run_info: &RunInfo,
    ) -> Distribution<Self> {
        let state = Distribution::single_value(Self {
            turn_counter: 0,
            player: Player {
                hand: UnorderedCardSet::from(vec![]),
                draw_pile: run_info.deck.clone().into(),
                draw_pile_top_card: None,
                discard_pile: UnorderedCardSet::from(vec![]),
                exhaust_pile: UnorderedCardSet::from(vec![]),
                orbs: vec![],
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
            enemies: vec![],
        });

        assert!(state.entries.iter().map(|(v, _)| v).all_unique());

        let state_with_enemy = match encounter {
            EncounterPrototype::FuzzyWurmCrawler => {
                let hp = 55..=57;

                let state = state.flat_map(|state| {
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
                            state_machine: EnemyStateMachine { current_state: 0 },
                        });

                        state
                    }))
                });

                state
            }
            EncounterPrototype::SingleNibbit => {
                let hp = 42..=46;

                let state = state.flat_map(|state| {
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
                            state_machine: EnemyStateMachine { current_state: 0 },
                        });

                        state
                    }))
                });

                state
            }
            EncounterPrototype::SlimesWeak => todo!(),
            EncounterPrototype::ShrinkerBeetle => {
                let hp = 38..=40;

                let state = state.flat_map(|state| {
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
                            state_machine: EnemyStateMachine { current_state: 0 },
                        });

                        state
                    }))
                });

                state
            }
        };

        assert!(state_with_enemy.entries.iter().map(|(v, _)| v).all_unique());

        let state = state_with_enemy.flat_map(Self::on_start_turn);

        assert!(state.entries.iter().map(|(v, _)| v).all_unique());
        assert!(!state.entries.is_empty());

        let state = state.flat_map(Self::draw_cards_for_turn);

        assert!(state.entries.iter().map(|(v, _)| v).all_unique());
        assert!(!state.entries.is_empty());

        state
    }

    pub(crate) const fn get_post_game_state(&self) -> Option<PostCombatState> {
        if self.enemies.is_empty() {
            Some(PostCombatState {
                turn_counter: self.turn_counter,

                hp: self.player.creature.hp,
                max_hp: self.player.creature.max_hp,
                potions_used: [false; 10],
                bonus_card_rewards: 0,
            })
        } else if self.player.creature.hp == 0 {
            // } else if self.player.is_dead() {
            Some(PostCombatState {
                turn_counter: self.turn_counter,

                hp: 0,
                max_hp: self.player.creature.max_hp,
                potions_used: [false; 10],
                bonus_card_rewards: 0,
            })
        } else {
            None
        }
    }

    pub(crate) fn legal_actions(&self) -> impl Iterator<Item = CombatAction> + use<'_> {
        // TODO: Potions
        self.player
            .hand
            .iter()
            .enumerate()
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
                usize::from(!self.player.hand.iter().any(|card| {
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
                })),
            ))
    }

    pub(crate) fn apply(&self, action: CombatAction) -> Distribution<Self> {
        match action {
            CombatAction::PlayCard { card, target } => {
                let mut result = self.clone();

                result.player.hand.remove_card(card);

                // FIXME: state effects on cost
                let cost = card.get_cost();

                let result = result.pay_cost(cost);
                // let result = Distribution::single_value(result);

                result.flat_map(|state| state.play_card(card.clone(), target.map(Into::into)))
            }
            CombatAction::UsePotion { index } => todo!(),
            CombatAction::EndTurn => {
                let result = self.clone();

                result.end_turn()
            }
        }
    }

    fn pay_cost(mut self, cost: Cost) -> Distribution<Self> {
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

    #[allow(clippy::match_same_arms)]
    fn end_turn(mut self) -> Distribution<Self> {
        // TODO: Check the order

        self.player.discard_pile.append(&mut self.player.hand);

        self.turn_counter += 1;

        // TODO: Orbs

        // Apply effect of statuses on enemies
        for enemy in &mut self.enemies {
            for (status, count) in &mut enemy.creature.statuses {
                match status {
                    Status::Strength => {}
                    Status::Dexterity => {}
                    Status::Vulnerable => {}
                    Status::Weak => {}
                    Status::Artifact => {}
                    Status::Frail => {}
                    Status::Focus => {}
                    Status::Vigor => {}
                    Status::Poison => {
                        enemy.creature.hp = enemy.creature.hp.saturating_sub_signed(*count);
                    }
                    Status::BonusEnergyOnTurnStart => {
                        assert!(*count == 0);
                    }
                    Status::Shrink => {}
                }
            }
            // Remove enemy block
            enemy.creature.block = 0;
        }

        // Enemy actions
        let mut state = Distribution::single_value(self);

        loop {
            let mut did_act = false;
            state = state.flat_map(|mut state| {
                let enemy = state
                    .enemies
                    .iter_mut()
                    .find_position(|enemy| !enemy.has_acted_this_turn);

                if let Some((index, enemy)) = enemy {
                    enemy.has_acted_this_turn = true;

                    let action = enemy.prototype.get_moveset().eval(&enemy.state_machine);

                    did_act = true;

                    let mut state = Distribution::single_value(state);
                    for action in action.actions {
                        state = match action {
                            EnemyAction::Attack {
                                base_damage,
                                repeat,
                            } => state.flat_map(|state| {
                                state.apply_attack_damage(
                                    CharacterIndex::Enemy(index),
                                    *base_damage,
                                    CharacterIndex::Player,
                                )
                            }),
                            EnemyAction::Block { amount } => state.flat_map(|state| {
                                state.add_block_to_creature(CharacterIndex::Enemy(index), *amount)
                            }),
                            EnemyAction::ApplyStatusSelf { status, diff } => {
                                state.flat_map(|state| {
                                    state.apply_status_change(
                                        CharacterIndex::Enemy(index),
                                        *status,
                                        *diff,
                                    )
                                })
                            }
                            EnemyAction::ApplyStatusPlayer { status, diff } => {
                                state.flat_map(|state| {
                                    state.apply_status_change(
                                        CharacterIndex::Player,
                                        *status,
                                        *diff,
                                    )
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
        let state = state.map(|mut state| {
            for enemy in &mut state.enemies {
                enemy
                    .prototype
                    .get_moveset()
                    .advance(&mut enemy.state_machine);

                enemy.has_acted_this_turn = false;
            }

            state
        });

        let state = state.flat_map(Self::on_start_turn);

        assert!(!state.entries.is_empty());

        let state = state.flat_map(Self::draw_cards_for_turn);

        assert!(!state.entries.is_empty());

        state
    }

    fn draw_cards_for_turn(self) -> Distribution<Self> {
        // TODO:
        let mut num_cards = 5;

        // FIXME: Implement relics
        if self.turn_counter == 0 {
            num_cards += 2;
        }

        let mut res = Distribution::single_value(self);

        for _ in 0..num_cards {
            assert!(!res.entries.is_empty());
            res = res.flat_map(Self::draw_single_card);
        }

        // This will produce lots of duplicated entries. Do reduce future work we dedup immediately
        res.dedup();

        res
    }

    fn draw_single_card(mut self) -> Distribution<Self> {
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
            let cards = self
                .player
                .draw_pile
                .iter()
                .counts()
                .into_iter()
                .sorted_by_key(|v| v.0);

            Distribution::from_duplicates(cards.into_iter().map(|(card, count)| {
                let mut new = self.clone();
                new.player.draw_pile.remove_card(*card);
                new.player.hand.add_card(*card);
                (new, count)
            }))
        };

        assert!(!state.entries.is_empty());

        state.flat_map(Self::on_draw_card)
    }

    #[allow(clippy::match_same_arms)]
    fn on_start_turn(mut self) -> Distribution<Self> {
        // TODO:

        // Apply effect of statuses on the player
        for (status, count) in &mut self.player.creature.statuses {
            match status {
                Status::Strength => {}
                Status::Dexterity => {}
                Status::Vulnerable => {}
                Status::Weak => {}
                Status::Artifact => {}
                Status::Frail => {}
                Status::Focus => {}
                Status::Vigor => {}
                Status::Poison => {
                    self.player.creature.hp = self.player.creature.hp.saturating_sub_signed(*count);
                }
                Status::BonusEnergyOnTurnStart => {
                    self.player.energy += u8::try_from(*count).unwrap();
                }
                Status::Shrink => {}
            }
        }

        // Give Player Energy
        // FIXME: Calculate the amount of energy to give
        self.player.energy += 3;

        // Remove player block
        // TODO: Keep block power
        self.player.creature.block = 0;

        self.apply_end_of_turn_status_changes()
    }

    #[allow(clippy::match_same_arms)]
    fn apply_end_of_turn_status_changes(mut self) -> Distribution<Self> {
        // TODO:

        // Decrease Status Values
        for (status, count) in &mut self.player.creature.statuses {
            match status {
                Status::Strength => {}
                Status::Dexterity => {}
                Status::Vulnerable => decrease_non_neg(count),
                Status::Weak => decrease_non_neg(count),
                Status::Poison => decrease_non_neg(count),
                Status::Artifact => {}
                Status::Frail => decrease_non_neg(count),
                Status::Focus => {}
                Status::Vigor => {}
                Status::BonusEnergyOnTurnStart => {}
                Status::Shrink => {}
            }
        }

        for enemy in &mut self.enemies {
            for (status, count) in &mut enemy.creature.statuses {
                match status {
                    Status::Strength => {}
                    Status::Dexterity => {}
                    Status::Vulnerable => decrease_non_neg(count),
                    Status::Weak => decrease_non_neg(count),
                    Status::Poison => decrease_non_neg(count),
                    Status::Artifact => {}
                    Status::Frail => decrease_non_neg(count),
                    Status::Focus => {}
                    Status::Vigor => {}
                    Status::BonusEnergyOnTurnStart => {}
                    Status::Shrink => {}
                }
            }
        }

        Distribution::single_value(self)
    }

    fn on_draw_card(self) -> Distribution<Self> {
        // Stuff like kingly kick (I think that gets cheaper when you draw it)

        Distribution::single_value(self)
    }

    fn on_draw_non_draw_phase_card(mut self) -> Distribution<Self> {
        // TODO: Stuff like speedster

        self.on_draw_card()
    }

    fn shuffle_discard_into_draw(mut self) -> Distribution<Self> {
        self.player.draw_pile.append(&mut self.player.discard_pile);

        Distribution::single_value(self)
    }

    // The card must already be removed from whereever it came from, so we take it by value here to express that
    #[allow(clippy::needless_pass_by_value)]
    fn play_card(mut self, card: Card, target: Option<usize>) -> Distribution<Self> {
        let state = Distribution::single_value(self);

        let state = match card.prototype {
            CardPrototype::Strike => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 9 } else { 6 };

                state.flat_map(|state| {
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

                state.flat_map(|slf| slf.add_block_to_creature(CharacterIndex::Player, base_amount))
            }
            CardPrototype::Neutralize => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 4 } else { 3 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                });

                state.flat_map(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Weak,
                        if card.upgraded { 2 } else { 1 },
                    )
                })
            }
            CardPrototype::Survivor => {
                assert!(target.is_none());
                // FIXME: todo!("This gives choices!")
                // FIXME: Add discard
                let base_amount = if card.upgraded { 11 } else { 8 };

                state.flat_map(|slf| slf.add_block_to_creature(CharacterIndex::Player, base_amount))
            }
            CardPrototype::PoisonedStab => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 8 } else { 6 };

                // FIXME: If the enemy die, the index will shift....
                let state = state.flat_map(|state| {
                    state.apply_attack_damage(
                        CharacterIndex::Player,
                        base_amount,
                        CharacterIndex::Enemy(target),
                    )
                });

                state.flat_map(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Poison,
                        if card.upgraded { 4 } else { 3 },
                    )
                })
            }
            CardPrototype::Backflip => {
                assert!(target.is_none());
                let base_amount = if card.upgraded { 8 } else { 5 };

                let mut state = state
                    .flat_map(|slf| slf.add_block_to_creature(CharacterIndex::Player, base_amount));

                let cards = 2;

                for _ in 0..cards {
                    state = state.flat_map(CombatState::draw_single_card);
                }
                state.dedup();
                state
            }
            CardPrototype::DeadlyPoison => {
                let target = target.unwrap();

                state.flat_map(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Poison,
                        if card.upgraded { 7 } else { 5 },
                    )
                })
            }
        };

        let state = state.flat_map(Self::on_any_card_played);
        state.map(|mut state| {
            state.player.discard_pile.add_card(card);
            state
        })
    }

    fn add_block_to_creature(
        mut self,
        creature: CharacterIndex,
        base_amount: u16,
    ) -> Distribution<Self> {
        let status = match creature {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        let amount = base_amount.saturating_add_signed(status[Status::Dexterity]);

        let amount = if status[Status::Frail] > 0 {
            (amount as f32 * 0.75) as u16
        } else {
            amount
        };

        match creature {
            CharacterIndex::Player => self.player.creature.block += amount,
            CharacterIndex::Enemy(index) => self.enemies[index].creature.block += amount,
        }

        // TODO: Triggers

        Distribution::single_value(self)
    }

    fn apply_attack_damage(
        mut self,
        source: CharacterIndex,
        base_amount: u16,
        target: CharacterIndex,
    ) -> Distribution<Self> {
        let source_status = match source {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        let amount = base_amount.saturating_add_signed(source_status[Status::Strength]);

        // Use up vigor
        let amount = amount.saturating_add_signed(source_status[Status::Vigor]);
        source_status[Status::Vigor] = 0;

        let amount = if source_status[Status::Weak] > 0 {
            (amount as f32 * 0.75) as u16
        } else {
            amount
        };

        let amount = if source_status[Status::Shrink] > 0 {
            (amount as f32 * 0.7) as u16
        } else {
            amount
        };

        let target_status = match target {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        let amount = if target_status[Status::Vulnerable] > 0 {
            (amount as f32 * 1.5) as u16
        } else {
            amount
        };

        // TODO: Triggers
        match target {
            CharacterIndex::Player => {
                let unblocked = amount.saturating_sub(self.player.creature.block);
                self.player.creature.block = self.player.creature.block.saturating_sub(amount);
                self.player.creature.hp = self.player.creature.hp.saturating_sub(unblocked);
            }
            CharacterIndex::Enemy(index) => {
                let enemy = &mut self.enemies[index];
                let unblocked = amount.saturating_sub(enemy.creature.block);
                enemy.creature.block = enemy.creature.block.saturating_sub(amount);
                enemy.creature.hp = enemy.creature.hp.saturating_sub(unblocked);
            }
        }

        let state = match target {
            CharacterIndex::Player => Distribution::single_value(self),
            CharacterIndex::Enemy(enemy_index) => self.on_enemy_lost_hp(enemy_index),
        };

        state
    }

    fn on_enemy_lost_hp(mut self, enemy_index: usize) -> Distribution<Self> {
        self.enemies.retain(|enemy| enemy.creature.hp > 0);

        Distribution::single_value(self)
    }

    fn apply_status_to_enemy(
        mut self,
        enemy_index: usize,
        status: Status,
        amount: i16,
    ) -> Distribution<Self> {
        let enemy = self.enemies.get_mut(enemy_index);

        if let Some(enemy) = enemy {
            enemy.creature.statuses[status] += amount;
        }

        Distribution::single_value(self)
    }

    fn apply_status_change(
        mut self,
        target: CharacterIndex,
        status: Status,
        diff: i16,
    ) -> Distribution<Self> {
        let status_list = match target {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        status_list[status] += diff;

        Distribution::single_value(self)
    }

    fn on_any_card_played(mut self) -> Distribution<Self> {
        Distribution::single_value(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player {
    pub hand: UnorderedCardSet,
    draw_pile: UnorderedCardSet,
    draw_pile_top_card: Option<Card>,
    discard_pile: UnorderedCardSet,
    exhaust_pile: UnorderedCardSet,

    orbs: Vec<Orb>,
    num_orb_slots: u8,

    energy: u8,
    stars: u8,

    pub creature: Creature,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            hand: vec![
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
            ]
            .into(),
            draw_pile: vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Neutralize.get_normal_card(),
                CardPrototype::Survivor.get_normal_card(),
            ]
            .into(),
            draw_pile_top_card: None,
            discard_pile: vec![].into(),
            exhaust_pile: vec![].into(),
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
    Artifact,
    Frail,
    Focus,
    Vigor,
    BonusEnergyOnTurnStart,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnemyStateMachine {
    pub current_state: usize,
}

pub enum EnemyMoveSet {
    ConstantRotation {
        // TODO: static would be much better
        // rotation: &'static [EnemyMove],
        rotation: Vec<EnemyMove>,
    },
    Prefix {
        prefixed_move: EnemyMove,
        after: Box<Self>,
    },
}

impl EnemyMoveSet {
    pub fn eval(&self, state_machine: &EnemyStateMachine) -> EnemyMove {
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
                    after.eval(&EnemyStateMachine {
                        current_state: state_machine.current_state - 1,
                    })
                }
            }
        }
    }

    fn advance(&self, state_machine: &mut EnemyStateMachine) {
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
}

impl EnemyPrototype {
    pub fn get_moveset(self) -> EnemyMoveSet {
        match self {
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
            Self::SmallTwigSlime => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    actions: &[EnemyAction::Attack {
                        base_damage: 4,
                        repeat: 1,
                    }],
                }],
            },
            Self::MediumTwigSlime => todo!(),
            Self::SmallLeafSlime => todo!(),
            Self::MediumLeafSlime => EnemyMoveSet::ConstantRotation {
                rotation: vec![
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
        }
    }
}

fn decrease_non_neg(val: &mut i16) {
    *val = max(0, *val - 1);
}

#[cfg(test)]
pub(crate) mod test {
    use std::{
        collections::{HashMap, HashSet},
        iter,
    };

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
                        statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    },
                    state_machine: EnemyStateMachine { current_state: 0 },

                    has_acted_this_turn: false,
                },
                Enemy {
                    prototype: EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    },
                    state_machine: EnemyStateMachine { current_state: 2 },

                    has_acted_this_turn: false,
                },
            ],
        }
    }

    pub fn very_confused() -> CombatState {
        use crate::game_state::CardPrototype::*;
        use crate::game_state::EnemyPrototype::*;
        CombatState {
            turn_counter: 2,
            player: Player {
                hand: UnorderedCardSet::from(vec![
                    Card {
                        prototype: Neutralize,
                        upgraded: false,
                    },
                    Card {
                        prototype: Survivor,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                ]),
                draw_pile: UnorderedCardSet::from(vec![
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                ]),
                draw_pile_top_card: None,
                discard_pile: UnorderedCardSet::from(vec![]),
                exhaust_pile: UnorderedCardSet::from(vec![]),
                orbs: vec![],
                num_orb_slots: 1,
                energy: 3,
                stars: 0,
                creature: Creature {
                    hp: 62,
                    max_hp: 70,
                    block: 6,
                    statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                },
            },
            enemies: vec![
                Enemy {
                    prototype: FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::from_array([7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2 },
                },
                Enemy {
                    prototype: FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 31,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 1 },
                },
            ],
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
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Survivor,
                        upgraded: false,
                    },
                ]
                .into(),
                draw_pile: vec![].into(),
                draw_pile_top_card: None,
                discard_pile: vec![
                    Card {
                        prototype: Neutralize,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Defend,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                    Card {
                        prototype: Strike,
                        upgraded: false,
                    },
                ]
                .into(),
                exhaust_pile: vec![].into(),
                orbs: vec![],
                num_orb_slots: 1,
                energy: 1,
                stars: 0,
                creature: Creature {
                    hp: 66,
                    max_hp: 70,
                    block: 0,
                    statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                },
            },
            enemies: vec![Enemy {
                prototype: FuzzyWurmCrawler,
                creature: Creature {
                    hp: 42,
                    max_hp: 57,
                    block: 0,
                    statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                },
                has_acted_this_turn: false,
                state_machine: EnemyStateMachine { current_state: 1 },
            }],
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
                .into(),
                draw_pile: vec![].into(),
                draw_pile_top_card: None,
                discard_pile: vec![].into(),
                exhaust_pile: vec![].into(),
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
                    statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                },
                state_machine: EnemyStateMachine { current_state: 2 },

                has_acted_this_turn: false,
            }],
        }
    }

    #[test]
    fn equality_for_card_sets() {
        assert_eq!(
            UnorderedCardSet::from(vec![CardPrototype::Strike.get_normal_card()]),
            UnorderedCardSet::from(vec![CardPrototype::Strike.get_normal_card()]),
        );

        assert_ne!(
            UnorderedCardSet::from(vec![CardPrototype::Strike.get_normal_card()]),
            UnorderedCardSet::from(vec![CardPrototype::Defend.get_normal_card()]),
        );

        assert_eq!(
            UnorderedCardSet::from(vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card()
            ]),
            UnorderedCardSet::from(vec![
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Strike.get_normal_card()
            ]),
        );

        let hash: HashSet<UnorderedCardSet, RandomState> =
            HashSet::from_iter(iter::once(UnorderedCardSet::from(vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
            ])));

        assert!(hash.contains(&UnorderedCardSet::from(vec![
            CardPrototype::Defend.get_normal_card(),
            CardPrototype::Strike.get_normal_card()
        ])));
    }
}
