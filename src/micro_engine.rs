use core::todo;
use std::{
    cmp::{max, min},
    collections::HashMap,
    fmt::{Debug, Display},
    iter::Sum,
    ops::{Add, ControlFlow, Mul, Sub},
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use itertools::Itertools;
use rapidhash::{HashMapExt, RapidHashMap};

use crate::{
    combat_action::CombatAction,
    game_state::{CombatState, PostCombatState},
};

#[derive(Debug)]
pub struct MicroEngine<F: EvaluationFunction> {
    // arena: bumpalo::Bump,
    eval_function: F,

    chance_node_transposition_table: rapidhash::RapidHashMap<
        (CombatState, CombatAction),
        TranspositionTableEntry<F::EvalResult>,
    >,

    choice_node_transposition_table:
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
    LowerBound(EvalResult),
    Exact {
        eval: EvalResult,
        action: CombatAction,
    },
}

pub trait EvaluationFunction {
    type EvalResult: Debug + Copy + EvalResult + PartialOrd + Into<f32>;
    fn evaluate_postgame_state(&self, post_combat_state: PostCombatState) -> Self::EvalResult;
    fn best_possible_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult;
    fn expected_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult;
}

pub trait EvalResult:
    Sum<Self>
    + Mul<f32, Output = Self>
    + PartialEq
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
{
    const MIN: Self;
    const MAX: Self;
    const ZERO: Self;
}

struct EventInfo<Eval: Copy> {
    lower_bound: Eval,
    upper_bound: Eval,
    chance: f32,
}

trait EventInfoVecExt<Eval> {
    fn lower_bound(&self) -> Eval;
    fn upper_bound(&self) -> Eval;
    fn exact_value(&self) -> Eval;

    fn compute_successor_min(&self, i: usize, alpha: Eval) -> Eval;
    fn compute_successor_max(&self, i: usize, beta: Eval) -> Eval;
}

impl<Eval: Copy + EvalResult> EventInfoVecExt<Eval> for Vec<EventInfo<Eval>> {
    fn lower_bound(&self) -> Eval {
        self.iter()
            .map(|entry| entry.lower_bound * entry.chance)
            .sum()
    }

    fn upper_bound(&self) -> Eval {
        self.iter()
            .map(|entry| entry.upper_bound * entry.chance)
            .sum()
    }

    fn exact_value(&self) -> Eval {
        assert!(self.lower_bound() == self.upper_bound());
        self.lower_bound()
    }

    fn compute_successor_min(&self, i: usize, alpha: Eval) -> Eval {
        let mut cur_alpha = alpha - self.upper_bound();
        cur_alpha = cur_alpha + self[i].upper_bound * self[i].chance;
        cur_alpha = cur_alpha * (1.0 / self[i].chance);
        cur_alpha
    }

    fn compute_successor_max(&self, i: usize, beta: Eval) -> Eval {
        let mut cur_beta = beta - self.lower_bound();
        cur_beta = cur_beta + self[i].lower_bound * self[i].chance;
        cur_beta = cur_beta * (1.0 / self[i].chance);
        cur_beta
    }
}

const PROBING_FACTOR: usize = 1;

impl<F: EvaluationFunction> MicroEngine<F> {
    pub fn new(fun: F) -> Self {
        Self {
            eval_function: fun,
            chance_node_transposition_table: RapidHashMap::new(),
            choice_node_transposition_table: RapidHashMap::new(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cull_table(&mut self, current_state: &CombatState) {
        let current_turn = current_state.turn_counter;

        self.chance_node_transposition_table
            .retain(|(state, _), _| state.turn_counter >= current_turn);
        self.choice_node_transposition_table
            .retain(|state, _| state.turn_counter >= current_turn);
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
            match self.get_max(
                state,
                F::EvalResult::MIN,
                F::EvalResult::MAX,
                // self.eval_function.best_possible_evaluation(state),
                depth,
            ) {
                ControlFlow::Continue(eval) => {}
                ControlFlow::Break(()) => {
                    dbg!(depth);
                    break;
                }
            }

            // let res =
            //     self.choice_node_transposition_table
            //         .get(state)
            //         .map_or((None, None), |entry| match entry.eval {
            //             EvalRunResult::Exact { eval, action } => (Some(action), Some(eval)),
            //             EvalRunResult::UpperBound(bound) => {
            //                 unreachable!("Only found upper bound {bound:?}")
            //             }
            //             EvalRunResult::LowerBound(bound) => {
            //                 unreachable!("Only found lower bound {bound:?}")
            //             }
            //         });

            // (logger)(format!("[Depth {depth}]: {res:?}",));

            // TODO: Debug
            // if let Some(action) = res.0 {
            //     let res = state.apply(action);
            //     (logger)(format!(
            //         "Resulting in e.g.: {:?}",
            //         res.entries.first().unwrap().0.player.creature
            //     ));
            // }
        }

        let final_eval = self.choice_node_transposition_table.get(state).map_or(
            (None, None),
            |entry| match entry.eval {
                EvalRunResult::Exact { eval, action } => (Some(action), Some(eval)),
                EvalRunResult::UpperBound(bound) => {
                    unreachable!("Only found upper bound {bound:?}")
                }
                EvalRunResult::LowerBound(bound) => {
                    unreachable!("Only found lower bound {bound:?}")
                }
            },
        );

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
                    self.get_expected(
                        (state.clone(), action),
                        F::EvalResult::MIN,
                        self.eval_function.best_possible_evaluation(state),
                        5,
                    )
                    .continue_value()
                    .unwrap(),
                )
            })
            .collect()
    }

    fn get_max(
        &mut self,
        state: &CombatState,
        mut alpha: F::EvalResult,
        mut beta: F::EvalResult,
        depth: u8,
    ) -> ControlFlow<(), F::EvalResult> {
        if self.stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
            return ControlFlow::Break(());
        }

        if let Some(combat_done) = state.get_post_game_state() {
            return ControlFlow::Continue(self.eval_function.evaluate_postgame_state(combat_done));
        }

        if depth == 0 {
            return ControlFlow::Continue(self.eval_function.expected_evaluation(&state));
        }

        if let Some(entry) = self
            .choice_node_transposition_table
            // FIXME: I hate this clone
            .get(state)
            && entry.depth_searched >= depth
        {
            match entry.eval {
                EvalRunResult::Exact { eval, action } => {
                    return std::ops::ControlFlow::Continue(eval);
                }
                EvalRunResult::LowerBound(bound) => {
                    if bound >= beta {
                        return std::ops::ControlFlow::Continue(bound);
                    }
                    if bound > alpha {
                        alpha = bound;
                    }
                }
                EvalRunResult::UpperBound(bound) => {
                    if bound <= alpha {
                        return std::ops::ControlFlow::Continue(bound);
                    }
                    if bound < beta {
                        beta = bound;
                    }
                }
            }
        }

        let legal_actions = state.legal_actions();
        // TODO: SORT ACTIONS

        let mut value = F::EvalResult::MIN;
        let mut best = None;
        for action in legal_actions {
            let expected = self.get_expected((state.clone(), action), alpha, beta, depth)?;

            if expected > value {
                best = Some(action);
                value = expected;
            }

            if value >= beta {
                self.choice_node_transposition_table.insert(
                    state.clone(),
                    TranspositionTableEntry {
                        eval: EvalRunResult::LowerBound(value),
                        depth_searched: depth,
                    },
                );
                return ControlFlow::Continue(value);
            }
            if value > alpha {
                alpha = value;
            }
        }

        self.choice_node_transposition_table.insert(
            state.clone(),
            TranspositionTableEntry {
                eval: EvalRunResult::Exact {
                    action: best.expect("Each state should have at least one action"),
                    eval: value,
                },
                depth_searched: depth,
            },
        );
        ControlFlow::Continue(value)
    }

    fn get_max_probe(
        &mut self,
        state: &CombatState,
        mut alpha: F::EvalResult,
        mut beta: F::EvalResult,
        depth: u8,

        probing_factor: usize,
    ) -> ControlFlow<(), F::EvalResult> {
        if self.stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
            return ControlFlow::Break(());
        }

        if let Some(combat_done) = state.get_post_game_state() {
            return ControlFlow::Continue(self.eval_function.evaluate_postgame_state(combat_done));
        }

        if depth == 0 {
            return ControlFlow::Continue(self.eval_function.expected_evaluation(state));
        }

        if let Some(entry) = self
            .choice_node_transposition_table
            // FIXME: I hate this clone
            .get(state)
            && entry.depth_searched >= depth
        {
            match entry.eval {
                EvalRunResult::Exact { eval, action } => {
                    return std::ops::ControlFlow::Continue(eval);
                }
                EvalRunResult::LowerBound(bound) => {
                    if bound >= beta {
                        return std::ops::ControlFlow::Continue(bound);
                    }
                    if bound > alpha {
                        alpha = bound;
                    }
                }
                EvalRunResult::UpperBound(bound) => {
                    if bound <= alpha {
                        return std::ops::ControlFlow::Continue(bound);
                    }
                    if bound < beta {
                        beta = bound;
                    }
                }
            }
        }

        let mut legal_actions = state.legal_actions().collect_vec();

        legal_actions.sort_by_key(|action| {
            if let Some(entry) = self.choice_node_transposition_table.get(state)
                && let EvalRunResult::Exact {
                    action: prev_best, ..
                } = entry.eval
                && *action == prev_best
            {
                return -1000;
            }

            match action {
                CombatAction::PlayCard { card, target } => {
                    // Look at rarer cards first
                    match card.get_rarity() {
                        crate::game_state::cards::Rarity::Basic => 4,
                        crate::game_state::cards::Rarity::Common => 3,
                        crate::game_state::cards::Rarity::Uncommon => 2,
                        crate::game_state::cards::Rarity::Rare => 1,
                    }
                }
                CombatAction::UsePotion { index } => 50,
                CombatAction::EndTurn => 100,
            }
        });

        // TODO: SORT ACTIONS

        let mut value = F::EvalResult::MIN;

        // Only consider the best probing_factor actions
        for action in legal_actions.into_iter().take(probing_factor) {
            let expected = self.get_expected((state.clone(), action), alpha, beta, depth)?;

            if expected > value {
                value = expected;
            }

            if value >= beta {
                self.choice_node_transposition_table.insert(
                    state.clone(),
                    TranspositionTableEntry {
                        eval: EvalRunResult::LowerBound(value),
                        depth_searched: depth,
                    },
                );
                return ControlFlow::Continue(value);
            }
            if value > alpha {
                alpha = value;
            }
        }

        self.choice_node_transposition_table.insert(
            state.clone(),
            TranspositionTableEntry {
                eval: EvalRunResult::LowerBound(value),
                depth_searched: depth,
            },
        );
        ControlFlow::Continue(value)
    }

    fn get_expected(
        &mut self,
        (state, action): (CombatState, CombatAction),
        mut alpha: F::EvalResult,
        mut beta: F::EvalResult,

        depth: u8,
    ) -> ControlFlow<(), F::EvalResult> {
        if self.stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
            return ControlFlow::Break(());
        }

        if let Some(combat_done) = state.get_post_game_state() {
            return ControlFlow::Continue(self.eval_function.evaluate_postgame_state(combat_done));
        }

        if depth == 0 {
            return ControlFlow::Continue(self.eval_function.expected_evaluation(&state));
        }

        // probe transposition table for node information
        if let Some(entry) = self
            .chance_node_transposition_table
            // FIXME: I hate this clone
            .get(&(state.clone(), action))
            && entry.depth_searched >= depth
        {
            match entry.eval {
                EvalRunResult::Exact { eval, action } => {
                    return std::ops::ControlFlow::Continue(eval);
                }
                EvalRunResult::LowerBound(bound) => {
                    if bound >= beta {
                        return std::ops::ControlFlow::Continue(bound);
                    }
                    if bound > alpha {
                        alpha = bound;
                    }
                }
                EvalRunResult::UpperBound(bound) => {
                    if bound <= alpha {
                        return std::ops::ControlFlow::Continue(bound);
                    }
                    if bound < beta {
                        beta = bound;
                    }
                }
            }
        }

        let mut successors = state.apply(action);
        // Sort successors by rising probability (and as such by rising influence on the expected value)
        successors.entries.sort_by(|a, b| b.1.total_cmp(&a.1));
        // TODO: Sort?

        let mut event_info = successors
            .entries
            .iter()
            .map(|(state, chance)| EventInfo {
                lower_bound: F::EvalResult::MIN,
                upper_bound: self.eval_function.best_possible_evaluation(state),
                chance: *chance,
            })
            .collect_vec();

        // probe transposition table for successor information
        for (i, (post_action_state, chance)) in successors.entries.iter().enumerate() {
            if let Some(entry) = self.choice_node_transposition_table.get(&post_action_state)
                && entry.depth_searched >= depth - 1
            {
                match entry.eval {
                    EvalRunResult::Exact { eval, .. } => {
                        event_info[i].lower_bound = eval;
                        if event_info.lower_bound() >= beta {
                            // TODO: Replacement rules
                            self.chance_node_transposition_table.insert(
                                (state, action),
                                TranspositionTableEntry {
                                    eval: EvalRunResult::LowerBound(event_info.lower_bound()),
                                    depth_searched: depth,
                                },
                            );
                            return std::ops::ControlFlow::Continue(event_info.lower_bound());
                        }

                        event_info[i].upper_bound = eval;
                        if event_info.upper_bound() <= alpha {
                            // TODO: Replacement rules
                            self.chance_node_transposition_table.insert(
                                (state, action),
                                TranspositionTableEntry {
                                    eval: EvalRunResult::UpperBound(event_info.upper_bound()),
                                    depth_searched: depth,
                                },
                            );
                            return std::ops::ControlFlow::Continue(event_info.upper_bound());
                        }
                    }
                    EvalRunResult::LowerBound(eval) => {
                        event_info[i].lower_bound = eval;
                        if event_info.lower_bound() >= beta {
                            // TODO: Replacement rules
                            self.chance_node_transposition_table.insert(
                                (state, action),
                                TranspositionTableEntry {
                                    eval: EvalRunResult::LowerBound(event_info.lower_bound()),
                                    depth_searched: depth,
                                },
                            );
                            return std::ops::ControlFlow::Continue(event_info.lower_bound());
                        }
                    }
                    EvalRunResult::UpperBound(eval) => {
                        event_info[i].upper_bound = eval;
                        if event_info.upper_bound() <= alpha {
                            // TODO: Replacement rules
                            self.chance_node_transposition_table.insert(
                                (state, action),
                                TranspositionTableEntry {
                                    eval: EvalRunResult::UpperBound(event_info.upper_bound()),
                                    depth_searched: depth,
                                },
                            );
                            return std::ops::ControlFlow::Continue(event_info.upper_bound());
                        }
                    }
                }
            }
        }

        // modified Star2-like probing phase
        // for (i, (post_action_state, chance)) in successors.entries.iter().enumerate() {
        //     // Exact eq like this is bad for floats
        //     // TODO: The source paper had `node_info[i].UpperBound`, but I assume that is a typo
        //     if event_info.lower_bound() != event_info[i].upper_bound {
        //         let cur_beta = event_info.compute_successor_max(i, beta);
        //         let max_possible = self
        //             .eval_function
        //             .best_possible_evaluation(post_action_state);

        //         let bx = if max_possible < cur_beta {
        //             max_possible
        //         } else {
        //             cur_beta
        //         };

        //         let search_val = self.get_max_probe(
        //             post_action_state,
        //             event_info[i].lower_bound,
        //             bx,
        //             depth - 1,
        //             PROBING_FACTOR,
        //         )?;

        //         event_info[i].lower_bound = search_val;

        //         if search_val >= cur_beta {
        //             self.chance_node_transposition_table.insert(
        //                 (state, action),
        //                 TranspositionTableEntry {
        //                     eval: EvalRunResult::LowerBound(event_info.lower_bound()),
        //                     depth_searched: depth,
        //                 },
        //             );
        //             return std::ops::ControlFlow::Continue(event_info.lower_bound());
        //         }
        //     }
        // }

        // Star1 search phase
        for (i, (post_action_state, chance)) in successors.entries.iter().enumerate() {
            let cur_alpha = event_info.compute_successor_min(i, alpha);
            let cur_beta = event_info.compute_successor_max(i, beta);

            let ax = if cur_alpha > F::EvalResult::MIN {
                cur_alpha
            } else {
                F::EvalResult::MIN
            };
            let max_possible = self
                .eval_function
                .best_possible_evaluation(post_action_state);
            let bx = if max_possible < cur_beta {
                max_possible
            } else {
                cur_beta
            };

            let search_val = self.get_max(post_action_state, ax, bx, depth - 1)?;

            event_info[i].lower_bound = search_val;
            event_info[i].upper_bound = search_val;

            if search_val >= cur_beta {
                self.chance_node_transposition_table.insert(
                    (state, action),
                    TranspositionTableEntry {
                        eval: EvalRunResult::LowerBound(event_info.lower_bound()),
                        depth_searched: depth,
                    },
                );
                return std::ops::ControlFlow::Continue(event_info.lower_bound());
            }

            if search_val <= cur_alpha {
                self.chance_node_transposition_table.insert(
                    (state, action),
                    TranspositionTableEntry {
                        eval: EvalRunResult::UpperBound(event_info.upper_bound()),
                        depth_searched: depth,
                    },
                );
                return std::ops::ControlFlow::Continue(event_info.upper_bound());
            }
        }

        self.chance_node_transposition_table.insert(
            (state, action),
            TranspositionTableEntry {
                eval: EvalRunResult::Exact {
                    eval: event_info.exact_value(),
                    action,
                },
                depth_searched: depth,
            },
        );
        std::ops::ControlFlow::Continue(event_info.exact_value())
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

    use crate::{
        TestEngineCurrentHp,
        game_state::{self, Creature, Enemy, EnemyStateMachine, Player},
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
        let mut engine = MicroEngine::new(TestEngineCurrentHp {});

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
        let mut engine = MicroEngine::new(TestEngineCurrentHp {});

        let state = game_state::test::unneeded_blocking();

        let map = engine
            .get_action_map(&state)
            .into_iter()
            .sorted_by(|(_, a), (_, b)| b.total_cmp(a))
            .collect_vec();
        dbg!(engine.get_max(
            &state,
            f32::MIN,
            engine.eval_function.best_possible_evaluation(&state),
            20
        ));
        dbg!(engine.choice_node_transposition_table.get(&state));

        print_action_map(&map, &state);
    }

    #[test]
    fn test_eval() {
        let mut state = game_state::test::simple_test_combat_state();

        let mut engine = MicroEngine::new(TestEngineCurrentHp {});

        loop {
            // dbg!(&state.enemies);
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
