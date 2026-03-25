use std::cmp::max;

use enum_map::Enum;
use itertools::Itertools;

use crate::{combat_action::CombatAction, distribution::Distribution};

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

impl CombatState {
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
                            index: card_index as u8,
                            target: None,
                        }],
                        LegalTarget::OtherPlayer => todo!(),
                        LegalTarget::Enemy => self
                            .enemies
                            .iter()
                            .enumerate()
                            .map(|(enemy_index, enemy)| CombatAction::PlayCard {
                                index: card_index as u8,
                                target: Some(enemy_index as u8),
                            })
                            .collect(),
                    })
            })
            .chain(std::iter::once(CombatAction::EndTurn))
    }

    pub(crate) fn apply(&self, action: CombatAction) -> Distribution<Self> {
        match action {
            CombatAction::PlayCard { index, target } => {
                let mut result = self.clone();

                let card = result.player.hand.remove(index as usize);

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

    fn end_turn(mut self) -> Distribution<Self> {
        // TODO: Check the order

        self.player.discard_pile.append(&mut self.player.hand);

        self.turn_counter += 1;

        // TODO: Orbs

        // TODO: Enemy actions
        // FIXME: Placeholder
        self.player.creature.hp = self
            .player
            .creature
            .hp
            .saturating_sub(5u16.saturating_sub(self.player.creature.block));
        self.player.creature.block = 0;
        let state = Distribution::single_value(self);

        // TODO: Next enemy intents

        let state = state.flat_map(Self::on_start_turn);

        let state = state.flat_map(Self::draw_cards_for_turn);

        state
    }

    fn draw_cards_for_turn(self) -> Distribution<Self> {
        // TODO:
        let num_cards = 5;

        let mut res = Distribution::single_value(self);

        for _ in 0..num_cards {
            res = res.flat_map(Self::draw_single_card);
        }

        res
    }

    fn draw_single_card(mut self) -> Distribution<Self> {
        let state = if let Some(top_card) = self.player.draw_pile_top_card.take() {
            self.player.hand.push(top_card);
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
            let cards = self.player.draw_pile.iter().counts();

            Distribution::from_duplicates(cards.into_iter().map(|(card, count)| {
                let mut new = self.clone();
                let drawn_card_index = new.player.draw_pile.iter().position(|c| c == card).unwrap();
                let drawn_card = new.player.draw_pile.swap_remove(drawn_card_index);
                new.player.hand.push(drawn_card);
                (new, count)
            }))
        };

        state.flat_map(Self::on_draw_card)
    }

    #[allow(clippy::match_same_arms)]
    fn on_start_turn(mut self) -> Distribution<Self> {
        // TODO:

        // Apply effect of statuses
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
                Status::BonusEnergyOnTurnStart => {
                    self.player.energy += u8::try_from(*count).unwrap();
                }
            }
        }

        // Give Player Energy
        // FIXME: Calculate the amount of energy to give
        self.player.energy += 3;

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
                Status::Artifact => {}
                Status::Frail => decrease_non_neg(count),
                Status::Focus => {}
                Status::Vigor => {}
                Status::BonusEnergyOnTurnStart => {}
            }
        }

        Distribution::single_value(self)
    }

    fn on_draw_card(mut self) -> Distribution<Self> {
        let drawn_card = self
            .player
            .hand
            .last()
            .expect("We just drew a card, why is our hand empty???");

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

                state.flat_map(|state| state.apply_attack_damage_to_enemy(base_amount, target))
            }
            CardPrototype::Defend => {
                let base_amount = if card.upgraded { 8 } else { 5 };

                state.flat_map(|slf| slf.add_armor_to_player(base_amount))
            }
            CardPrototype::Neutralize => {
                let target = target.unwrap();
                let base_amount = if card.upgraded { 4 } else { 3 };

                // FIXME: If the enemy die, the index will shift....
                let state =
                    state.flat_map(|state| state.apply_attack_damage_to_enemy(base_amount, target));

                state.flat_map(|state| {
                    state.apply_status_to_enemy(
                        target,
                        Status::Weak,
                        if card.upgraded { 2 } else { 1 },
                    )
                })
            }
            CardPrototype::Survivor => {
                // FIXME: todo!("This gives choices!")
                state
            }
        };

        let state = state.flat_map(Self::on_any_card_played);
        state.map(|mut state| {
            state.player.discard_pile.push(card.clone());
            state
        })
    }

    fn add_armor_to_player(mut self, base_amount: u16) -> Distribution<Self> {
        let amount =
            base_amount.saturating_add_signed(self.player.creature.statuses[Status::Dexterity]);

        let amount = if self.player.creature.statuses[Status::Frail] > 0 {
            (amount as f32 * 0.75) as u16
        } else {
            amount
        };

        assert!(amount >= 0);
        self.player.creature.block += amount;
        // TODO: Triggers

        Distribution::single_value(self)
    }

    fn apply_attack_damage_to_enemy(
        mut self,
        base_amount: u16,
        enemy_index: usize,
    ) -> Distribution<Self> {
        let amount =
            base_amount.saturating_add_signed(self.player.creature.statuses[Status::Strength]);

        // Use up the players vigor
        let amount = amount.saturating_add_signed(self.player.creature.statuses[Status::Vigor]);
        self.player.creature.statuses[Status::Vigor] = 0;

        let amount = if self.player.creature.statuses[Status::Weak] > 0 {
            (amount as f32 * 0.75) as u16
        } else {
            amount
        };

        let enemy = &mut self.enemies[enemy_index];

        let amount = if enemy.creature.statuses[Status::Vulnerable] > 0 {
            (amount as f32 * 1.5) as u16
        } else {
            amount
        };

        assert!(amount >= 0);

        let unblocked = amount.saturating_sub(enemy.creature.block);
        enemy.creature.block = enemy.creature.block.saturating_sub(amount);
        enemy.creature.hp = enemy.creature.hp.saturating_sub(unblocked);

        // dbg!(enemy);
        // TODO: Triggers

        let state = if unblocked > 0 {
            self.on_enemy_lost_hp(enemy_index)
        } else {
            Distribution::single_value(self)
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

    fn on_any_card_played(mut self) -> Distribution<Self> {
        Distribution::single_value(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player {
    pub hand: Vec<Card>,
    draw_pile: Vec<Card>,
    draw_pile_top_card: Option<Card>,
    discard_pile: Vec<Card>,
    exhaust_pile: Vec<Card>,

    orbs: Vec<Orb>,
    num_orb_slots: u8,

    energy: u8,
    stars: u8,

    pub creature: Creature,
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
    prototype: EnemyPrototype,
    pub creature: Creature,

    state_machine: EnemyStateMachine,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Creature {
    pub hp: u16,
    max_hp: u16,
    block: u16,

    statuses: enum_map::EnumMap<Status, i16>,
}

#[derive(Debug, Clone, Copy, Enum)]
enum Status {
    Strength,
    Dexterity,
    Vulnerable,
    Weak,
    Artifact,
    Frail,
    Focus,
    Vigor,
    BonusEnergyOnTurnStart,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EnemyStateMachine {}

impl EnemyStateMachine {
    fn get_intent(&self) -> Intent {
        todo!()
    }
}

enum Intent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EnemyPrototype {
    SmallTwigSlime,
    MediumTwigSlime,
    FuzzyWurmCrawler,
    ShrinkerBeetle,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Card {
    pub prototype: CardPrototype,
    upgraded: bool,
}

#[derive(Debug, Clone, Copy)]
struct Cost {
    energy: CostVal,
    stars: CostVal,
}

#[derive(Debug, Clone, Copy)]
enum CostVal {
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
    const fn get_cost(&self) -> Cost {
        match (self.prototype, self.upgraded) {
            (CardPrototype::Strike, _) => ENERGY[1],
            (CardPrototype::Defend, _) => ENERGY[1],
            (CardPrototype::Neutralize, _) => ENERGY[0],
            (CardPrototype::Survivor, _) => ENERGY[1],
        }
    }

    #[allow(clippy::match_same_arms)]
    fn get_legal_targets(&self) -> impl Iterator<Item = LegalTarget> {
        match self.prototype {
            CardPrototype::Strike => [LegalTarget::Enemy],
            CardPrototype::Defend => [LegalTarget::OwnPlayer],
            CardPrototype::Neutralize => [LegalTarget::Enemy],
            CardPrototype::Survivor => [LegalTarget::OwnPlayer],
        }
        .into_iter()
    }
}

enum LegalTarget {
    OwnPlayer,
    OtherPlayer,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPrototype {
    Strike,
    Defend,
    Neutralize,
    Survivor,
}

impl CardPrototype {
    const fn get_normal_card(self) -> Card {
        Card {
            prototype: self,
            upgraded: false,
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
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CardKind {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

fn decrease_non_neg(val: &mut i16) {
    *val = max(0, *val - 1)
}

#[cfg(test)]
pub(crate) mod test {
    use enum_map::EnumMap;

    use super::*;

    pub fn simple_test_combat_state() -> CombatState {
        CombatState {
            turn_counter: 0,
            player: Player {
                hand: vec![
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                ],
                draw_pile: vec![
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Neutralize.get_normal_card(),
                    CardPrototype::Survivor.get_normal_card(),
                ],
                draw_pile_top_card: None,
                discard_pile: vec![],
                exhaust_pile: vec![],
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
                state_machine: EnemyStateMachine {},
            }],
        }
    }
}
