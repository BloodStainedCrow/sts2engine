#![feature(iterator_try_collect)]
#![feature(allocator_api)]

use std::{
    thread,
    time::{Duration, Instant},
};

use itertools::Itertools;

use crate::{
    combat_action::CombatAction,
    comm::Comm,
    game_state::{
        CombatState, PostCombatState, RunInfo, Status,
        cards::{CardKind, CardPrototype},
    },
    micro_engine::{EvalResult, EvaluationFunction},
};

mod combat_action;
mod comm;
mod distribution;
mod game_state;
mod micro_engine;

struct TestEngineCurrentHp {}

impl EvalResult for f32 {
    const MIN: Self = Self::MIN;
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

        let damage_done_per_turn = 10.0;
        let damage_taken_per_turn_base = 5.0;

        let mut enemies: Vec<_> = combat_state.enemies.iter().collect();

        let incoming_damage: u16 = enemies
            .iter()
            .map(|enemy| {
                let mov = enemy.prototype.get_moveset().eval(&enemy.state_machine);
                mov.actions
                    .iter()
                    .filter_map(|action| match action {
                        game_state::EnemyAction::Attack {
                            base_damage,
                            repeat,
                        } => Some((*base_damage, *repeat)),
                        _ => None,
                    })
                    .map(|(damage, repeat)| {
                        damage.saturating_add_signed(enemy.creature.statuses[Status::Strength])
                            * repeat
                    })
                    .sum::<u16>()
            })
            .sum();

        enemies.sort_by_key(|enemy| enemy.creature.hp);

        let turns_per_enemy = enemies
            .into_iter()
            .map(|hp| hp.creature.hp as f32 / damage_done_per_turn)
            .collect_vec();

        let mut damage = 0.0;
        for start in 0..turns_per_enemy.len() {
            damage += turns_per_enemy[start]
                * (turns_per_enemy.len() - start) as f32
                * ((turns_per_enemy.len() + combat_state.turn_counter as usize - start) as f32
                    / 3.0)
                * damage_taken_per_turn_base;
        }

        let eval = f32::from(combat_state.player.creature.hp)
            - damage
            - f32::from(incoming_damage.saturating_sub(combat_state.player.creature.block));

        // dbg!(eval);

        eval - f32::from(combat_state.turn_counter)
    }
}

fn main() {
    // TODO: Assume specific fight
    let pre_first_turn_state = game_state::CombatState::get_starting_states(
        game_state::EncounterPrototype::FuzzyWurmCrawler,
        &RunInfo {
            hp: 66,
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
                CardPrototype::Neutralize.get_normal_card(),
                CardPrototype::Survivor.get_normal_card(),
            ],
        },
    );

    let mut state = pre_first_turn_state;

    let mut engine = micro_engine::MicroEngine::new(TestEngineCurrentHp {});

    let mut comm = Comm::new();

    loop {
        state.dedup();
        let real_state =
            comm.find_valid_combat_states(state.entries.drain(..).map(|(v, _)| v).collect());

        let Ok(real_state) = real_state else {
            todo!("Could not determine state")
        };
        dbg!(
            &real_state.enemies[0]
                .prototype
                .get_moveset()
                .eval(&real_state.enemies[0].state_machine)
        );

        let action = engine.next_combat_action(&real_state, 99, Duration::from_secs(2), |msg| {
            eprintln!("{msg}");
        });

        let Some(action) = action.0 else {
            break;
        };

        if let CombatAction::PlayCard { card, .. } = action
            && card.prototype.get_kind() == CardKind::Skill
        {
            let has_attack = real_state
                .player
                .hand
                .iter()
                .any(|card| card.prototype.get_kind() == CardKind::Attack);

            let enemy_is_attacking = real_state.enemies[0]
                .prototype
                .get_moveset()
                .eval(&real_state.enemies[0].state_machine)
                .actions
                .iter()
                .any(|action| matches!(action, game_state::EnemyAction::Attack { .. }));

            assert!(
                !has_attack || enemy_is_attacking,
                "Unneeded defend over attacking in state {real_state:?}"
            );
        }

        state = real_state.apply(action);
        comm.apply_action(action);

        // After applying the action on the game, we need to wait for stuff to settle (I do not know what the game returns while the animations are playing)
        let start = Instant::now();
        let presumed_done = start + Duration::from_secs(6);

        // Use the time on calcs insread of just waiting
        if state.entries.len() == 1 {
            let assumed_next_state = &state.entries.first().unwrap().0;
            engine.next_combat_action(assumed_next_state, 99, Duration::from_secs(7), |_| {});
        } else {
            engine.next_combat_action(&real_state, 99, Duration::from_secs(7), |_| {});
        }

        thread::sleep(presumed_done - start);
    }

    println!("No more action, is the fight over?");
}
