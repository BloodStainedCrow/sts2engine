use std::ops::AddAssign;

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

    fn get_eval(&self) -> Option<Self::Eval> {
        self.get_post_game_state().map(|state| Eval {
            v: (f32::from(state.hp) - f32::from(state.turn_counter) * 0.3)
                / f32::from(state.max_hp),
        })
    }

    fn apply(&mut self, action: &Self::Action) {
        let res = CombatState::apply::<single::Distribution<CombatState>>(self, *action).collapse();
        *self = res;
    }
}
