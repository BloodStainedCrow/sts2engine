use std::{
    iter::Sum,
    ops::{Add, AddAssign, Mul},
};

use rand::{rng, seq::IteratorRandom};
use sts2mcts::mcts;

use crate::{
    combat_action::CombatAction,
    combat_state::{CombatSide, CombatState, Player},
    distribution::{Distribution, single},
};

#[derive(Debug, Clone, Copy)]
pub struct Eval {
    v: f32,
}

impl mcts::Eval for Eval {
    const ZERO: Self = Self { v: 0.0 };
}

impl Add for Eval {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self { v: self.v + rhs.v }
    }
}

impl AddAssign for Eval {
    fn add_assign(&mut self, rhs: Self) {
        self.v += rhs.v;
    }
}

impl From<Eval> for f32 {
    fn from(value: Eval) -> Self {
        value.v
    }
}

impl Mul<f32> for Eval {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self { v: self.v * rhs }
    }
}

impl Sum for Eval {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self {
            v: iter.map(|v| v.v).sum(),
        }
    }
}

impl mcts::GameState for CombatState {
    type Action = CombatAction;

    type Eval = Eval;

    fn legal_actions(&self) -> impl Iterator<Item = Self::Action> {
        self.legal_actions()
    }

    fn rollout_action(&self) -> Self::Action {
        // TODO: One technique I read about, which seems very promising is the idea of recording the average utility of an action across the entire search tree, and use this information to bias the rollout.
        // This seems like a good idea, since it should bias the rollout toward playing the "better" actions and hopefully should increase the number of victories in simulation. (Which I think I want...?)
        // Source: https://www.sciencedirect.com/science/article/pii/S0004370214001052#se0060 (4.3.2. Move-average sampling technique (MAST))

        // Only click end turn during rollout, if no other options exist. This should prevent us from dying *quite as hard*
        // A higher percentage of games leading to *any* kind of victory should help us isolate the better action better
        self.legal_actions_without_turn_end()
            .choose(&mut rng())
            .unwrap_or(CombatAction::EndTurn)
    }

    fn get_eval(&self) -> Option<Self::Eval> {
        assert!(
            self.player.creature.hp <= self.player.creature.max_hp,
            "HP: {}, MAX HP: {}",
            self.player.creature.hp,
            self.player.creature.max_hp
        );

        self.get_post_game_state().map(|mut state| {
            if state.hp > 0
                && state.hp <= state.max_hp / 2
                && self
                    .relic_state
                    .contains(crate::combat_state::relics::RelicPrototype::MeatOnTheBone)
            {
                state.hp = std::cmp::min(state.max_hp, state.hp + 12)
            }

            if state.hp == 0 {
                // When we die, reward taking longer to die!
                return Eval {
                    v: f32::from(state.turn_counter) / 256.0,
                };
            }
            Eval {
                // Reward faster kills, but only if they do not result in up losing *any* hp
                v: (f32::from(state.hp) - f32::from(state.turn_counter) / 256.0)
                    / f32::from(state.max_hp),
            }
        })
    }

    // TODO: Maybe track the number of possible outcomes (without deduplication of course) which should allow us to mark choice nodes as fully explored (and maybe even mark states as "pre terminal" with known outcome (forced loss))
    fn apply(&mut self, action: &Self::Action) {
        // TODO: This means we abort when panicking, which seems annoying. Do try to quantify the amount this saves on performance
        take_mut::take_or_recover(
            self,
            || CombatState {
                turn_counter: 0,
                current_turn_side: CombatSide::Player,
                player: Box::new(Player::default()),
                enemies: vec![].into(),
                relic_state: [].into_iter().collect(),
            },
            |state| {
                CombatState::apply::<single::Distribution<CombatState>>(state, *action).collapse()
            },
        );
    }
}
