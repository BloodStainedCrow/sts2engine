use std::ops::AddAssign;

use rand::{rng, seq::IteratorRandom};
use sts2mcts::mcts;

use crate::{
    combat_action::CombatAction,
    distribution::{Distribution, single},
    game_state::CombatState,
};

#[derive(Debug, Clone, Copy)]
pub struct Eval {
    v: f32,
}

impl mcts::Eval for Eval {
    const ZERO: Self = Self { v: 0.0 };
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

impl mcts::GameState for CombatState {
    type Action = CombatAction;

    type Eval = Eval;

    fn legal_actions(&self) -> (usize, impl Iterator<Item = Self::Action>) {
        let count = self.legal_actions().count();

        (count, self.legal_actions())
    }

    fn rollout_action(&self) -> Self::Action {
        // Only click end turn during rollout, if no other options exist. This should prevent us from dying *quite as hard*
        // A higher percentage of games leading to *any* kind of victory should help us isolate the better action better
        self.legal_actions()
            .filter(|action| *action != CombatAction::EndTurn)
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

        self.get_post_game_state().map(|state| {
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

    fn apply(&mut self, action: &Self::Action) {
        let res = CombatState::apply::<single::Distribution<CombatState>>(self, *action).collapse();
        *self = res;
    }
}
