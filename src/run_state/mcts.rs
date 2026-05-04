use sts2mcts::mcts;

use crate::{
    distribution::{Distribution, full::FullFamily, single::SingleFamily},
    mcts::Eval,
    run_state::{ActPrototype, Map, RunAction, RunState},
};

impl mcts::GameState for RunState {
    type Action = RunAction;

    type Eval = crate::mcts::Eval;

    fn legal_actions(&self) -> impl Iterator<Item = Self::Action> {
        self.legal_actions()
    }

    fn get_eval(&self) -> Option<Self::Eval> {
        match &self.sub_state {
            Some(crate::run_state::SubState::Combat { combat }) => {
                if let Some(post_combat) = combat.get_post_game_state() {
                    if post_combat.victory {
                        unreachable!("We should have exited the combat when we won")
                        // None
                    } else {
                        // We died
                        Some(Eval {
                            v: self.current_position.row as f32 / 15.0
                                + f32::from(post_combat.turn_counter) / 256.0 / 15.0,
                        })
                    }
                } else {
                    None
                }
            }
            Some(crate::run_state::SubState::Shop { .. }) => None,
            Some(crate::run_state::SubState::Event { .. }) => None,
            // TODO: Double check these numbers
            Some(crate::run_state::SubState::RewardScreen { .. }) | None => {
                if self.current_act.prototype == ActPrototype::Glory
                    && self.current_position.row == 13
                {
                    Some(Eval { v: 1.0 })
                } else {
                    None
                }
            }
        }
    }

    fn apply(&mut self, action: &Self::Action) {
        // TODO: This means we abort when panicking, which seems annoying. Do try to quantify the amount this saves on performance
        take_mut::take_or_recover(
            self,
            || RunState::start_run::<SingleFamily>(Map::default()).collapse(),
            |state| RunState::apply_action::<SingleFamily>(state, *action).collapse(),
        );
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use itertools::Itertools;
    use sts2mcts::mcts::GameState;

    use crate::run_state::Map;

    use super::*;

    #[test]
    fn run_state_mcts() {
        let mut state = RunState::start_run::<FullFamily>(Map::default()).collapse();
        let mut mcts = mcts::MCTS::new(state.clone());

        loop {
            let timer = if matches!(
                state.sub_state,
                Some(crate::run_state::SubState::Combat { .. })
            ) {
                Duration::from_secs_f32(0.1)
            } else {
                Duration::from_secs_f32(3.0)
            };
            let res = mcts.search(timer);

            state = state.apply_action::<SingleFamily>(res).collapse();

            if state.get_eval().is_some() {
                break;
            }

            dbg!(mcts.expected_eval());
            dbg!(state.player_hp, state.current_position.row);
            // dbg!(&state.sub_state);
            dbg!(&state.deck.last());
            dbg!(state.player_relics.iter().collect_vec());
            mcts.clear_advance(&state);
        }

        dbg!(state);
    }
}
