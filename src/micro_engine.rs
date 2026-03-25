use std::{fmt::Debug, iter::Sum, ops::Mul};

use itertools::Itertools;

use crate::{
    combat_action::CombatAction,
    distribution::Distribution,
    game_state::{CombatState, PostCombatState},
};

#[derive(Debug)]
struct MicroEngine<F: EvaluationFunction> {
    eval_function: F,

    transposition_table:
        rapidhash::RapidHashMap<CombatState, TranspositionTableEntry<F::EvalResult>>,
}

#[derive(Debug)]
struct TranspositionTableEntry<EvalResult> {
    eval: EvalResult,
    action: CombatAction,

    depth_searched: u8,
}

trait EvaluationFunction {
    type EvalResult: Debug
        + Copy
        + Sum<Self::EvalResult>
        + Mul<f32, Output = Self::EvalResult>
        + EvalResult
        + PartialOrd;
    fn evaluate_postgame_state(&self, post_combat_state: PostCombatState) -> Self::EvalResult;
    fn best_possible_evaluation(&self, combat_state: &CombatState) -> Option<Self::EvalResult>;
    fn expected_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult;
}

trait EvalResult {
    const MIN: Self;
}

impl<F: EvaluationFunction> MicroEngine<F> {
    fn next_combat_action(
        &mut self,
        state: &CombatState,
        max_depth: u8,
    ) -> (Option<CombatAction>, F::EvalResult) {
        for depth in 0..max_depth {
            self.evaluate(state, F::EvalResult::MIN, depth);
        }

        let eval = self.evaluate(state, F::EvalResult::MIN, max_depth);

        if let Some(entry) = self.transposition_table.get(state) {
            (Some(entry.action), entry.eval)
        } else {
            (None, eval)
        }
    }

    pub(crate) fn evaluate(
        &mut self,
        state: &CombatState,
        guaranteed_reachable_expected: F::EvalResult,

        max_depth: u8,
    ) -> F::EvalResult {
        if let Some(cached) = self.transposition_table.get(state)
            && cached.depth_searched >= max_depth
        {
            return cached.eval;
        }

        #[cfg(test)]
        test::STATES_EVALUATED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        #[cfg(test)]
        test::STATES_EVALUATED_AT_DEPTH[usize::from(max_depth)]
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if max_depth == 0 {
            return self.eval_function.expected_evaluation(state);
        }

        if let Some(combat_done) = state.get_post_game_state() {
            return self.eval_function.evaluate_postgame_state(combat_done);
        }

        // if let Some(best_possible) = self.eval_function.best_possible_evaluation(state)
        {
            let best_possible = self.eval_function.expected_evaluation(state);
            if guaranteed_reachable_expected >= best_possible {
                // Cull
                #[cfg(test)]
                test::TIMES_CULLED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                #[cfg(test)]
                test::TIMES_CULLED_AT_DEPTH[usize::from(max_depth)]
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                return F::EvalResult::MIN;
            }
        }

        let legal_actions = state.legal_actions();

        // TODO: Sort actions smartly, if we look at good actions first, we cull more
        let legal_actions = legal_actions.sorted_by_key(|action| {
            if let Some(entry) = self.transposition_table.get(state)
                && entry.action == *action
            {
                -1000
            } else {
                match action {
                    CombatAction::PlayCard { index, target } => {
                        match state.player.hand[*index as usize].prototype.get_kind() {
                            crate::game_state::CardKind::Attack => 0,
                            crate::game_state::CardKind::Skill => 2,
                            crate::game_state::CardKind::Power => 1,
                            crate::game_state::CardKind::Status => 3,
                            crate::game_state::CardKind::Curse => 4,
                        }
                    }
                    CombatAction::UsePotion { index } => 10,
                    CombatAction::EndTurn => 20,
                }
            }
        });

        let mut possible_here = guaranteed_reachable_expected;
        let mut best_action_here = None;
        for action in legal_actions {
            let result: Distribution<CombatState> = state.apply(action);

            let result_with_this_action = result
                .map(|state| self.evaluate(&state, possible_here, max_depth - 1))
                .expected_value();
            if result_with_this_action > possible_here {
                possible_here = result_with_this_action;
                best_action_here = Some(action);
            }
        }

        if best_action_here.is_none() {
            return possible_here;
        }

        self.transposition_table.insert(
            state.clone(),
            TranspositionTableEntry {
                eval: possible_here,
                action: best_action_here.expect("Each state always has at least one legal action"),

                depth_searched: max_depth,
            },
        );

        possible_here
    }
}

#[cfg(test)]
mod test {
    pub static TIMES_CULLED: AtomicUsize = AtomicUsize::new(0);
    pub static TIMES_CULLED_AT_DEPTH: [AtomicUsize; 20] = [const { AtomicUsize::new(0) }; 20];
    pub static STATES_EVALUATED: AtomicUsize = AtomicUsize::new(0);
    pub static STATES_EVALUATED_AT_DEPTH: [AtomicUsize; 20] = [const { AtomicUsize::new(0) }; 20];

    use std::sync::atomic::AtomicUsize;

    use rapidhash::HashMapExt;

    use crate::game_state;

    use super::*;

    struct TestEngineCurrentHp {}

    impl EvalResult for f32 {
        const MIN: Self = 0.0;
    }

    impl EvaluationFunction for TestEngineCurrentHp {
        type EvalResult = f32;

        fn evaluate_postgame_state(&self, post_combat_state: PostCombatState) -> Self::EvalResult {
            f32::from(post_combat_state.turn_counter)
                .mul_add(-0.01, f32::from(post_combat_state.hp))
        }

        fn best_possible_evaluation(&self, combat_state: &CombatState) -> Option<Self::EvalResult> {
            Some(
                f32::from(combat_state.player.creature.hp),
                // 1.0,
            )
        }
        fn expected_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult {
            f32::from(combat_state.player.creature.hp)
                - combat_state
                    .enemies
                    .iter()
                    .map(|enemy| f32::from(enemy.creature.hp) * 0.1)
                    .sum::<f32>()
        }
    }

    #[test]
    fn test_eval() {
        let mut state = game_state::test::simple_test_combat_state();

        let mut engine = MicroEngine {
            eval_function: TestEngineCurrentHp {},

            transposition_table: rapidhash::RapidHashMap::new(),
        };

        loop {
            let (action, result) = engine.next_combat_action(&state, dbg!(17));

            if let Some(action) = action {
                let result = state.apply(action).collapse();
                state = result;
            } else {
                break;
            }

            // engine.transposition_table = HashMap::new();
        }

        dbg!(STATES_EVALUATED.load(std::sync::atomic::Ordering::SeqCst));
        dbg!(TIMES_CULLED.load(std::sync::atomic::Ordering::SeqCst));

        for depth in &STATES_EVALUATED_AT_DEPTH {
            dbg!(depth.load(std::sync::atomic::Ordering::SeqCst));
        }

        // assert_eq!(result, state.player.creature.hp as f32);
        dbg!(state);
    }
}
