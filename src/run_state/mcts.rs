use sts2mcts::mcts;

use crate::run_state::{RunAction, RunState};

impl mcts::GameState for RunState {
    type Action = RunAction;

    type Eval = crate::mcts::Eval;

    fn legal_actions(&self) -> (usize, impl Iterator<Item = Self::Action>) {
        (todo!(), vec![todo!()].into_iter())
    }

    fn rollout_action(&self) -> Self::Action {
        todo!()
    }

    fn get_eval(&self) -> Option<Self::Eval> {
        todo!()
    }

    fn apply(&mut self, action: &Self::Action) {
        todo!()
    }
}
