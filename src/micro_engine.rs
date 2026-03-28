use core::todo;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    iter::Sum,
    ops::{Add, ControlFlow, Mul},
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use itertools::Itertools;
use rapidhash::{HashMapExt, RapidHashMap};

use crate::{
    combat_action::CombatAction,
    distribution::Distribution,
    game_state::{CombatState, PostCombatState},
};

#[derive(Debug)]
pub struct MicroEngine<F: EvaluationFunction> {
    // arena: bumpalo::Bump,
    eval_function: F,

    transposition_table:
        rapidhash::RapidHashMap<CombatState, TranspositionTableEntry<F::EvalResult>>,

    stop_signal: Arc<AtomicBool>,
}

#[derive(Debug)]
struct TranspositionTableEntry<EvalResult> {
    eval: EvalRunResult<EvalResult>,

    depth_searched: u8,
}

#[derive(Debug)]
enum EvalRunResult<EvalResult> {
    UpperBound(EvalResult),
    Exact {
        eval: EvalResult,
        action: CombatAction,
    },
}

pub trait EvaluationFunction {
    type EvalResult: Debug
        + Copy
        + Sum<Self::EvalResult>
        + Mul<f32, Output = Self::EvalResult>
        + Add<Self::EvalResult, Output = Self::EvalResult>
        + EvalResult
        + PartialOrd
        + Into<f32>;
    fn evaluate_postgame_state(&self, post_combat_state: PostCombatState) -> Self::EvalResult;
    fn best_possible_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult;
    fn expected_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult;
}

pub trait EvalResult {
    const MIN: Self;
    const ZERO: Self;
}

impl<F: EvaluationFunction> MicroEngine<F> {
    pub fn new(fun: F) -> Self {
        Self {
            eval_function: fun,
            transposition_table: RapidHashMap::new(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn next_combat_action(
        &mut self,
        state: &CombatState,
        max_depth: u8,

        timeout: Duration,

        logger: impl Fn(String),
    ) -> (Option<CombatAction>, Option<F::EvalResult>) {
        if state.get_post_game_state().is_some() {
            return (None, None);
        }

        // if let Ok(single_action) = state.legal_actions().exactly_one() {
        //     return (Some(single_action), None);
        // }

        self.stop_signal
            .store(false, std::sync::atomic::Ordering::SeqCst);

        let stop = self.stop_signal.clone();
        thread::spawn(move || {
            thread::sleep(timeout);

            stop.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        for depth in 0..=max_depth {
            match self.evaluate(state.clone(), F::EvalResult::MIN, depth) {
                ControlFlow::Continue(eval) => {}
                ControlFlow::Break(()) => {
                    dbg!(depth);
                    break;
                }
            }

            let res = self
                .transposition_table
                .get(state)
                .map_or((None, None), |entry| match entry.eval {
                    EvalRunResult::Exact { eval, action } => (Some(action), Some(eval)),
                    EvalRunResult::UpperBound(upper) => {
                        unreachable!("Only found upper bound {upper:?}")
                    }
                });

            (logger)(format!("[Depth {depth}]: {res:?}",));

            // TODO: Debug
            // if let Some(action) = res.0 {
            //     let res = state.apply(action);
            //     (logger)(format!(
            //         "Resulting in e.g.: {:?}",
            //         res.entries.first().unwrap().0.player.creature
            //     ));
            // }
        }

        let final_eval = self
            .transposition_table
            .get(state)
            .map_or((None, None), |entry| match entry.eval {
                EvalRunResult::Exact { eval, action } => (Some(action), Some(eval)),
                EvalRunResult::UpperBound(upper) => {
                    unreachable!("Only found upper bound {upper:?}")
                }
            });

        (logger)(format!("[Final Eval]: {final_eval:?}"));

        final_eval
    }

    pub(crate) fn get_action_map(
        &mut self,
        state: &CombatState,
    ) -> HashMap<CombatAction, F::EvalResult> {
        self.stop_signal
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let legal_actions = state.legal_actions();

        legal_actions
            .map(|action| {
                (
                    action,
                    state
                        .apply(action)
                        .map(|state| {
                            self.evaluate(state, F::EvalResult::MIN, 3)
                                .continue_value()
                                .unwrap()
                        })
                        .expected_value(),
                )
            })
            .collect()
    }

    pub(crate) fn evaluate(
        &mut self,
        state: CombatState,
        guaranteed_reachable_expected: F::EvalResult,

        max_depth: u8,
    ) -> ControlFlow<(), F::EvalResult> {
        if self.stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
            return ControlFlow::Break(());
        }

        if let Some(cached) = self.transposition_table.get(&state)
            && cached.depth_searched >= max_depth
        {
            match cached.eval {
                EvalRunResult::UpperBound(upper_bound) => {
                    if upper_bound <= guaranteed_reachable_expected {
                        #[cfg(test)]
                        test::TIMES_CACHED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return ControlFlow::Continue(upper_bound);
                    }
                }
                EvalRunResult::Exact { eval, .. } => {
                    #[cfg(test)]
                    test::TIMES_CACHED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return ControlFlow::Continue(eval);
                }
            }
        }

        #[cfg(test)]
        test::STATES_EVALUATED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        #[cfg(test)]
        test::STATES_EVALUATED_AT_DEPTH[usize::from(max_depth)]
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if let Some(combat_done) = state.get_post_game_state() {
            return ControlFlow::Continue(self.eval_function.expected_evaluation(&state));
        }

        if max_depth == 0 {
            // dbg!(guaranteed_reachable_expected);
            return ControlFlow::Continue(self.eval_function.expected_evaluation(&state));
        }

        let legal_actions = state.legal_actions();

        // TODO: Sort actions smartly, if we look at good actions first, we cull more
        let legal_actions: Vec<_> = legal_actions
            .sorted_by_key(|action| {
                if let Some(entry) = self.transposition_table.get(&state)
                    && let EvalRunResult::Exact { action: cached, .. } = entry.eval
                    && cached == *action
                {
                    -1000
                } else {
                    match action {
                        CombatAction::PlayCard { card, target } => {
                            match card.prototype.get_kind() {
                                crate::game_state::cards::CardKind::Attack => 0,
                                crate::game_state::cards::CardKind::Skill => 2,
                                crate::game_state::cards::CardKind::Power => 1,
                                crate::game_state::cards::CardKind::Status => 3,
                                crate::game_state::cards::CardKind::Curse => 4,
                            }
                        }
                        CombatAction::UsePotion { index } => 10,
                        CombatAction::EndTurn => 20,
                    }
                }
            })
            .collect();

        let mut possible_here = guaranteed_reachable_expected;
        let mut best_action_here = None;
        'action: for action in legal_actions {
            let result: Distribution<CombatState> = state.apply(action);

            let mut eval: F::EvalResult = F::EvalResult::ZERO;

            let Distribution { mut entries } = result;
            entries.sort_by(|(_, a), (_, b)| b.total_cmp(a));

            let debug = entries.clone();

            while !entries.is_empty() {
                let upper_bound = eval
                    + entries[..entries.len()]
                        .iter()
                        .map(|(state, chance)| {
                            self.eval_function.best_possible_evaluation(state) * *chance
                        })
                        .sum();

                let (state, chance) = entries.pop().expect("We checked for empty before");

                if upper_bound <= possible_here {
                    // This action cannot be good better than something we found anymore

                    #[cfg(test)]
                    test::TIMES_CULLED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    continue 'action;
                }

                let this_state_eval = self.evaluate(state, possible_here, max_depth - 1)?;
                eval = eval + this_state_eval * chance;
            }

            if eval > possible_here {
                possible_here = eval;
                best_action_here = Some(action);
            }
        }

        let new_entry = TranspositionTableEntry {
            eval: match best_action_here {
                Some(action) => EvalRunResult::Exact {
                    eval: possible_here,
                    action,
                },
                None => EvalRunResult::UpperBound(possible_here),
            },

            depth_searched: max_depth,
        };

        match self.transposition_table.entry(state) {
            std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                if max_depth >= occupied_entry.get().depth_searched
                    && (best_action_here.is_some()
                        || matches!(occupied_entry.get().eval, EvalRunResult::UpperBound(_)))
                {
                    occupied_entry.insert(new_entry);
                }
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(new_entry);
            }
        }

        ControlFlow::Continue(possible_here)
    }
}

fn print_action_map<Eval: Display>(map: &Vec<(CombatAction, Eval)>, state: &CombatState) {
    for (action, eval) in map {
        match action {
            CombatAction::PlayCard { card, target } => {
                println!("[Play {card:?} on {target:?}]: {eval}");
            }
            CombatAction::UsePotion { index } => todo!(),
            CombatAction::EndTurn => println!("[End Turn]: {eval}"),
        }
    }
}

#[cfg(test)]
mod test {
    pub static TIMES_CACHED: AtomicUsize = AtomicUsize::new(0);
    pub static TIMES_CULLED: AtomicUsize = AtomicUsize::new(0);
    pub static TIMES_CULLED_AT_DEPTH: [AtomicUsize; 100] = [const { AtomicUsize::new(0) }; _];
    pub static STATES_EVALUATED: AtomicUsize = AtomicUsize::new(0);
    pub static STATES_EVALUATED_AT_DEPTH: [AtomicUsize; 100] = [const { AtomicUsize::new(0) }; _];

    use std::{f32, sync::atomic::AtomicUsize};

    use enum_map::EnumMap;
    use rapidhash::HashMapExt;

    use crate::{
        TestEngineCurrentHp,
        game_state::{self, Creature, Enemy, EnemyStateMachine, Player, Status},
    };

    use super::*;

    #[test]
    fn ensure_focus_is_preferred() {
        let spread = TestEngineCurrentHp {}.expected_evaluation(&CombatState {
            turn_counter: 0,
            player: Player::default(),
            enemies: vec![
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 49,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 0 },
                },
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 49,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2 },
                },
            ],
        });
        let focus = TestEngineCurrentHp {}.expected_evaluation(&CombatState {
            turn_counter: 0,
            player: Player::default(),
            enemies: vec![
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 43,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 0 },
                },
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2 },
                },
            ],
        });

        dbg!(spread, focus);
        assert!(spread < focus, "{spread} < {focus}");

        let spread = TestEngineCurrentHp {}.expected_evaluation(&CombatState {
            turn_counter: 0,
            player: Player::default(),
            enemies: vec![
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 49,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 0 },
                },
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 49,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2 },
                },
            ],
        });
        let focus = TestEngineCurrentHp {}.expected_evaluation(&CombatState {
            turn_counter: 0,
            player: Player::default(),
            enemies: vec![
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 43,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 0 },
                },
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2 },
                },
            ],
        });

        assert!(spread < focus, "{spread} < {focus}");
    }

    #[test]
    fn test_action_map() {
        let mut engine = MicroEngine {
            eval_function: TestEngineCurrentHp {},

            transposition_table: rapidhash::RapidHashMap::new(),

            stop_signal: Arc::new(AtomicBool::new(false)),
        };

        let state = CombatState {
            turn_counter: 0,
            player: Player::default(),
            enemies: vec![
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 49,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 0 },
                },
                Enemy {
                    prototype: game_state::EnemyPrototype::FuzzyWurmCrawler,
                    creature: Creature {
                        hp: 55,
                        max_hp: 55,
                        block: 0,
                        statuses: EnumMap::default(),
                    },
                    has_acted_this_turn: false,
                    state_machine: EnemyStateMachine { current_state: 2 },
                },
            ],
        };

        let map = engine
            .get_action_map(&state)
            .into_iter()
            .sorted_by(|(_, a), (_, b)| b.total_cmp(a))
            .collect_vec();

        print_action_map(&map, &state);
    }

    #[test]
    fn test_action_map_different_values() {
        let mut engine = MicroEngine {
            eval_function: TestEngineCurrentHp {},

            transposition_table: rapidhash::RapidHashMap::new(),

            stop_signal: Arc::new(AtomicBool::new(false)),
        };

        let state = game_state::test::unneeded_blocking();

        dbg!(engine.evaluate(state.clone(), f32::MIN, 4));
        let map = engine
            .get_action_map(&state)
            .into_iter()
            .sorted_by(|(_, a), (_, b)| b.total_cmp(a))
            .collect_vec();

        print_action_map(&map, &state);
    }

    #[test]
    fn test_eval() {
        let mut state = game_state::test::unneeded_blocking();

        let mut engine = MicroEngine {
            eval_function: TestEngineCurrentHp {},

            transposition_table: rapidhash::RapidHashMap::new(),

            stop_signal: Arc::new(AtomicBool::new(false)),
        };

        loop {
            dbg!(&state.enemies);
            // let map = engine
            //     .get_action_map(&state)
            //     .into_iter()
            //     .sorted_by(|(_, a), (_, b)| b.total_cmp(a))
            //     .collect_vec();

            // print_action_map(&map, &state);
            let (action, eval) =
                engine.next_combat_action(&state, 99, Duration::from_secs(10), |msg| {
                    eprintln!("{msg}");
                });
            dbg!(&action);

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
        dbg!(TIMES_CACHED.load(std::sync::atomic::Ordering::SeqCst));

        for depth in &STATES_EVALUATED_AT_DEPTH {
            dbg!(depth.load(std::sync::atomic::Ordering::SeqCst));
        }

        // assert_eq!(result, state.player.creature.hp as f32);
        dbg!(state.get_post_game_state());
    }
}
