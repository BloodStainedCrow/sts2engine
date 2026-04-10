#![feature(iterator_try_collect)]
#![feature(allocator_api)]

use mimalloc::MiMalloc;
use sts2mcts::mcts::MCTS;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::{
    io::{Read, stdin},
    thread,
    time::{Duration, Instant},
};

use itertools::Itertools;

use crate::{
    comm::Comm,
    distribution::Distribution,
    game_state::{
        CombatState, PostCombatState, RunInfo, Status,
        cards::{Card, CardPrototype},
    },
    micro_engine::{EvalResult, EvaluationFunction},
};

mod combat_action;
mod comm;
mod distribution;
mod game_state;
mod mcts;
mod micro_engine;

struct TestEngineCurrentHp {}

impl EvalResult for f32 {
    const MIN: Self = Self::MIN;
    const MAX: Self = Self::MAX;
    const ZERO: Self = 0.0;
}

impl EvaluationFunction for TestEngineCurrentHp {
    type EvalResult = f32;

    fn evaluate_postgame_state(&self, post_combat_state: PostCombatState) -> Self::EvalResult {
        // f32::from(post_combat_state.turn_counter)
        //     .mul_add(-0.01, f32::from(post_combat_state.hp))
        f32::from(post_combat_state.hp)
    }

    fn best_possible_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult {
        f32::from(combat_state.player.creature.hp)
        // 1.0,
    }

    fn expected_evaluation(&self, combat_state: &CombatState) -> Self::EvalResult {
        // if combat_state.get_post_game_state().is_some() {
        //     dbg!(combat_state);
        // }

        let poison_per_turn =
            f32::from(combat_state.player.creature.statuses[Status::NoxiousFumes]);
        let damage_done_per_turn = 10.0
            + 1.5 * f32::from(combat_state.player.creature.statuses[Status::Tracking])
            + 0.5 * f32::from(combat_state.player.creature.statuses[Status::Accuracy]);
        let mut damage_taken_per_turn_base = (1.0 + (f32::from(combat_state.turn_counter)))
            - f32::from(combat_state.player.creature.statuses[Status::Dexterity])
            - f32::from(combat_state.player.creature.statuses[Status::Fasten]) / 2.0;

        if damage_taken_per_turn_base < 0.0 {
            damage_taken_per_turn_base = 0.0;
        }

        let mut enemies: Vec<_> = combat_state.enemies.iter().collect();

        // let incoming_damage: u16 = enemies
        //     .iter()
        //     .map(|enemy| {
        //         let mov = enemy.prototype.get_moveset().eval(&enemy.state_machine);
        //         mov.actions
        //             .iter()
        //             .filter_map(|action| match action {
        //                 game_state::EnemyAction::Attack {
        //                     base_damage,
        //                     repeat,
        //                 } => Some((*base_damage, *repeat)),
        //                 _ => None,
        //             })
        //             .map(|(damage, repeat)| {
        //                 damage.saturating_add_signed(enemy.creature.statuses[Status::Strength])
        //                     * repeat
        //             })
        //             .sum::<u16>()
        //     })
        //     .sum();

        enemies.sort_by_key(|enemy| enemy.creature.hp);

        let turns_per_enemy = enemies
            .into_iter()
            .map(|enemy| {
                let poison_dmg_per_turn = f32::from(enemy.creature.statuses[Status::Poison])
                    / if poison_per_turn > 0.0 { 1.0 } else { 2.0 };
                f32::from(enemy.creature.hp) / (damage_done_per_turn + poison_dmg_per_turn)
            })
            .collect_vec();

        let mut damage = 0.0;
        for start in 0..turns_per_enemy.len() {
            damage += turns_per_enemy[start]
                * (turns_per_enemy.len() - start) as f32
                * damage_taken_per_turn_base;
        }

        let eval = f32::from(combat_state.player.creature.hp) - damage;
        // - f32::from(incoming_damage.saturating_sub(combat_state.player.creature.block));

        // dbg!(eval);

        eval - f32::from(combat_state.turn_counter)
    }
}

fn main() {
    run_mcts();
}

fn run_mcts() {
    let mut comm = Comm::new();
    loop {
        let pre_first_turn_state = game_state::CombatState::get_starting_states::<
            distribution::full::Distribution<_>,
        >(comm.guess_encounter(), &comm.get_run_state(), |hps| {
            comm.filter_hp(hps)
        });

        let mut state = pre_first_turn_state;

        loop {
            state.dedup();
            let real_state_res = comm.find_valid_combat_states(state.into_values().collect());

            let Ok(real_state) = real_state_res else {
                println!("No valid option!");
                // Lets assume we failed because the combat is over
                break;
            };

            if real_state.get_post_game_state().is_some() {
                break;
            }

            // TODO: Do not discard the full game tree, since previous work is still valuable
            let mut engine: MCTS<CombatState> = MCTS::new(real_state.clone());
            let action = engine.par_search(Duration::from_secs(5));
            dbg!(sts2mcts::mcts::NODES_CHECKED.load(std::sync::atomic::Ordering::Relaxed));
            dbg!(action);

            state = real_state.apply(action);
            comm.apply_action(action);

            // After applying the action on the game, we need to wait for stuff to settle (I do not know what the game returns while the animations are playing)
            // TODO: Use the time on calcs instead of just waiting
            // engine.par_search(Duration::from_secs(4));

            thread::sleep(Duration::from_secs(2));
        }

        println!("No more action, is the fight over?");
        let _ = stdin().read(&mut [0]).expect("Failed to read from stdin");
    }
}

fn run_expectimax() {
    use game_state::relics::RelicPrototype::*;

    // TODO: Assume specific fight
    let pre_first_turn_state =
        game_state::CombatState::get_starting_states::<distribution::full::Distribution<_>>(
            game_state::encounter::EncounterPrototype::RubyRaiders,
            &RunInfo {
                hp: 68,
                max_hp: 70,
                deck: vec![
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Strike.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    CardPrototype::Defend.get_normal_card(),
                    Card {
                        prototype: CardPrototype::Neutralize,
                        upgraded: true,
                        enchantment: None,
                    },
                    CardPrototype::Survivor.get_normal_card(),
                    CardPrototype::DodgeAndRoll.get_normal_card(),
                    Card {
                        prototype: CardPrototype::DaggerSpray,
                        upgraded: true,
                        enchantment: None,
                    },
                ],
                relic_state: [RingOfTheSnake, Pomander].into_iter().collect(),
            },
            |hp| hp == [23, 22, 30],
            // |_| true,
        );

    let mut state = pre_first_turn_state;

    let mut engine: micro_engine::MicroEngine<TestEngineCurrentHp> =
        micro_engine::MicroEngine::new(TestEngineCurrentHp {});

    let mut comm = Comm::new();

    loop {
        state.dedup();
        let real_state_res = comm.find_valid_combat_states(state.into_values().collect());

        let real_state = real_state_res.unwrap();

        let action = engine.next_combat_action(&real_state, 99, Duration::from_secs(10), |msg| {
            eprintln!("{msg}");
        });

        let Some(action) = action.0 else {
            break;
        };

        state = real_state.apply(action);
        comm.apply_action(action);

        // After applying the action on the game, we need to wait for stuff to settle (I do not know what the game returns while the animations are playing)
        let start = Instant::now();
        let presumed_done = start + Duration::from_secs(4);

        // Use the time on calcs insread of just waiting
        engine.next_combat_action(&real_state, 99, Duration::from_secs(4), |_| {});

        thread::sleep(presumed_done - start);
    }

    println!("No more action, is the fight over?");
}
