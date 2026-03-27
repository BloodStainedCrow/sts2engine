use std::ops::{Deref, DerefMut};
use std::{array, cmp::max, os::linux::raw::stat};

use enum_map::{Enum, EnumMap};
use itertools::Itertools;
use rapidhash::fast::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;

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

#[derive(Debug, Clone, Copy)]
enum CharacterIndex {
    Player,
    Enemy(usize),
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

                let index = result.player.hand.iter().position(|v| *v == card).unwrap();
                let card = result.player.hand.remove(index);

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

        // Enemy actions
        let mut state = Distribution::single_value(self);

        loop {
            let mut did_attack = false;
            state = state.flat_map(|mut state| {
                let enemy = state
                    .enemies
                    .iter_mut()
                    .find_position(|enemy| !enemy.has_acted_this_turn);

                if let Some((index, enemy)) = enemy {
                    enemy.has_acted_this_turn = true;

                    let enemy_actions = enemy.prototype.get_moveset();

                    let (base_damage, status_change) = match enemy_actions {
                        EnemyMoveSet::ConstantRotation { rotation } => (
                            rotation[enemy.state_machine.current_state].attack.0,
                            rotation[enemy.state_machine.current_state].apply_status_self,
                        ),
                    };

                    did_attack = true;
                    let state = state.apply_attack_damage(
                        CharacterIndex::Enemy(index),
                        base_damage,
                        CharacterIndex::Player,
                    );

                    state.flat_map(|state| {
                        state.apply_statuses(CharacterIndex::Enemy(index), status_change)
                    })
                } else {
                    Distribution::single_value(state)
                }
            });

            if !did_attack {
                break;
            }
        }

        // Next enemy intents
        let state = state.map(|mut state| {
            for enemy in &mut state.enemies {
                match enemy.prototype.get_moveset() {
                    EnemyMoveSet::ConstantRotation { rotation } => {
                        enemy.state_machine.current_state += 1;
                        enemy.state_machine.current_state %= rotation.len();
                    }
                }

                enemy.has_acted_this_turn = false;
            }

            state
        });

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
            let cards = self
                .player
                .draw_pile
                .iter()
                .counts()
                .into_iter()
                .sorted_by_key(|v| v.0);

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

        for enemy in &mut self.enemies {
            for (status, count) in &mut enemy.creature.statuses {
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

                state.flat_map(|slf| slf.add_armor_to_player(base_amount))
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

                state.flat_map(|slf| slf.add_armor_to_player(base_amount))
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

        self.player.creature.block += amount;
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

        // Use up the players vigor
        let amount = amount.saturating_add_signed(source_status[Status::Vigor]);
        source_status[Status::Vigor] = 0;

        let amount = if self.player.creature.statuses[Status::Weak] > 0 {
            (amount as f32 * 0.75) as u16
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

    fn apply_statuses(
        mut self,
        target: CharacterIndex,
        status_change: EnumMap<Status, i16>,
    ) -> Distribution<Self> {
        let status = match target {
            CharacterIndex::Player => &mut self.player.creature.statuses,
            CharacterIndex::Enemy(index) => &mut self.enemies[index].creature.statuses,
        };

        for (val, change) in status.values_mut().zip(status_change.into_values()) {
            *val += change;
        }

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

#[derive(Debug, Clone, Eq, Default)]
pub struct UnorderedCardSet {
    cards: Vec<Card>,
}

impl From<Vec<Card>> for UnorderedCardSet {
    fn from(value: Vec<Card>) -> Self {
        Self { cards: value }
    }
}

impl PartialEq for UnorderedCardSet {
    fn eq(&self, other: &Self) -> bool {
        self.cards.iter().counts() == other.cards.iter().counts()
        // self.cards == other.cards
    }
}

impl Deref for UnorderedCardSet {
    type Target = Vec<Card>;

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

impl DerefMut for UnorderedCardSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cards
    }
}

impl Hash for UnorderedCardSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut res: u64 = 1;
        for card in &self.cards {
            let mut card_hasher = RapidHasher::default_const();

            card.hash(&mut card_hasher);
            let card_hash = card_hasher.finish();

            res = res.wrapping_add(card_hash);
        }
        res.hash(state);
        // self.cards.hash(state);
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

#[derive(Debug, Clone, Copy, Enum, PartialEq, Eq)]
pub enum Status {
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
pub struct EnemyStateMachine {
    pub current_state: usize,
}

pub enum EnemyMoveSet {
    ConstantRotation {
        // TODO: static would be much better
        // rotation: &'static [EnemyMove],
        rotation: Vec<EnemyMove>,
    },
}

pub struct EnemyMove {
    pub attack: (u16, u16),
    block: u8,
    apply_status_self: EnumMap<Status, i16>,
    apply_status_player: EnumMap<Status, i16>,
}

const STATUS_MOVE_MAPS: EnumMap<Status, fn(i16) -> EnumMap<Status, i16>> = EnumMap::from_array([
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Strength] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Dexterity] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Vulnerable] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Weak] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Artifact] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Frail] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Focus] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::Vigor] = amount;
        v
    },
    |amount| {
        let mut v = EnumMap::from_array([0; _]);
        v[Status::BonusEnergyOnTurnStart] = amount;
        v
    },
]);

impl EnemyMove {
    const fn default() -> Self {
        Self {
            attack: (0, 0),
            block: 0,
            apply_status_self: EnumMap::from_array([0; _]),
            apply_status_player: EnumMap::from_array([0; _]),
        }
    }
}

impl EnemyStateMachine {
    fn get_intent(&self) -> Intent {
        todo!()
    }
}

enum Intent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyPrototype {
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
            Self::FuzzyWurmCrawler => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        attack: (4, 1),
                        ..EnemyMove::default()
                    },
                    EnemyMove {
                        apply_status_self: STATUS_MOVE_MAPS[Status::Strength](7),
                        ..EnemyMove::default()
                    },
                    EnemyMove {
                        attack: (4, 1),
                        ..EnemyMove::default()
                    },
                ],
            },
            Self::SmallTwigSlime => EnemyMoveSet::ConstantRotation {
                rotation: vec![EnemyMove {
                    attack: (4, 1),
                    ..EnemyMove::default()
                }],
            },
            Self::MediumTwigSlime => todo!(),
            Self::SmallLeafSlime => todo!(),
            Self::MediumLeafSlime => EnemyMoveSet::ConstantRotation {
                rotation: vec![
                    EnemyMove {
                        // TODO: Status cards
                        ..EnemyMove::default()
                    },
                    EnemyMove {
                        attack: (8, 1),
                        ..EnemyMove::default()
                    },
                ],
            },

            Self::ShrinkerBeetle => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
                        statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0]),
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
                        statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0]),
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
                hand: UnorderedCardSet {
                    cards: vec![
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
                    ],
                },
                draw_pile: UnorderedCardSet {
                    cards: vec![
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
                    ],
                },
                draw_pile_top_card: None,
                discard_pile: UnorderedCardSet { cards: vec![] },
                exhaust_pile: UnorderedCardSet { cards: vec![] },
                orbs: vec![],
                num_orb_slots: 1,
                energy: 3,
                stars: 0,
                creature: Creature {
                    hp: 62,
                    max_hp: 70,
                    block: 6,
                    statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0]),
                },
            },
            enemies: vec![
                Enemy {
                    prototype: FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::from_array([7, 0, 0, 0, 0, 0, 0, 0, 0]),
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
                        statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0]),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 1 },
                },
            ],
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
                    statuses: EnumMap::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0]),
                },
                state_machine: EnemyStateMachine { current_state: 2 },

                has_acted_this_turn: false,
            }],
        }
    }

    #[test]
    fn equality_for_card_sets() {
        assert_eq!(
            UnorderedCardSet {
                cards: vec![CardPrototype::Strike.get_normal_card()]
            },
            UnorderedCardSet {
                cards: vec![CardPrototype::Strike.get_normal_card()]
            }
        );

        assert_ne!(
            UnorderedCardSet {
                cards: vec![CardPrototype::Strike.get_normal_card()]
            },
            UnorderedCardSet {
                cards: vec![CardPrototype::Defend.get_normal_card()]
            }
        );

        assert_eq!(
            UnorderedCardSet {
                cards: vec![
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Defend.get_normal_card()
                ]
            },
            UnorderedCardSet {
                cards: vec![
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card()
                ]
            }
        );

        let hash: HashSet<UnorderedCardSet, RandomState> =
            HashSet::from_iter(iter::once(UnorderedCardSet {
                cards: vec![
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                ],
            }));

        assert!(
            hash.get(&UnorderedCardSet {
                cards: vec![
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Strike.get_normal_card()
                ]
            })
            .is_some()
        );
    }
}
