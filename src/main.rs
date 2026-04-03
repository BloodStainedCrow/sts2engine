#![feature(iterator_try_collect)]
#![feature(allocator_api)]

use bumpalo::{Bump, collections::CollectIn, vec};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::{
    thread,
    time::{Duration, Instant},
};

use itertools::Itertools;

use crate::{
    comm::Comm,
    game_state::{
        CombatState, PostCombatState, RunInfo, Status,
        cards::{Card, CardEnchantment, CardPrototype},
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
    use game_state::relics::RelicPrototype::*;
    let mut bump: Bump = Bump::new();
    let mut temp_bump = Bump::new();

    // TODO: Assume specific fight
    let pre_first_turn_state = game_state::CombatState::get_starting_states(
        game_state::EncounterPrototype::SlumberParty,
        &RunInfo {
            hp: 30,
            deck: vec![in &bump;
                Card { prototype: CardPrototype::Strike, upgraded: false, enchantment: Some(CardEnchantment::TezcatarasEmber)},
                Card { prototype: CardPrototype::Strike, upgraded: false, enchantment: Some(CardEnchantment::TezcatarasEmber)},
                Card { prototype: CardPrototype::Strike, upgraded: false, enchantment: Some(CardEnchantment::TezcatarasEmber)},
                Card { prototype: CardPrototype::Strike, upgraded: false, enchantment: Some(CardEnchantment::TezcatarasEmber)},
                Card { prototype: CardPrototype::Strike, upgraded: false, enchantment: Some(CardEnchantment::TezcatarasEmber)},
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Neutralize.get_normal_card(),
                CardPrototype::Survivor.get_normal_card(),
                CardPrototype::Greed.get_normal_card(),
                CardPrototype::LegSweep.get_normal_card(),
                Card { prototype: CardPrototype::NoxiousFumes, upgraded: true, enchantment: None },
                CardPrototype::PreciseCut.get_normal_card(),
                CardPrototype::Fasten.get_normal_card(),
                CardPrototype::Anticipate.get_normal_card(),
                Card { prototype: CardPrototype::DodgeAndRoll, upgraded: true, enchantment: None },
                Card { prototype: CardPrototype::CloakAndDagger, upgraded: true, enchantment: None },
                Card { prototype: CardPrototype::CloakAndDagger, upgraded: true, enchantment: None },
                CardPrototype::LeadingStrike.get_normal_card(),
                CardPrototype::Tracking.get_normal_card(),
                CardPrototype::SuckerPunch.get_normal_card(),
                Card { prototype: CardPrototype::Haze, upgraded: true, enchantment: None },
                CardPrototype::Squash.get_normal_card(),
                CardPrototype::Accuracy.get_normal_card(),
                Card { prototype: CardPrototype::Dash, upgraded: true, enchantment: None },
                Card { prototype: CardPrototype::Burst, upgraded: true, enchantment: None },
                Card { prototype: CardPrototype::BladeDance, upgraded: true, enchantment: None },
            ],
            relic_state: [
                RingOfTheSnake,
                CursedPearl,
                ToxicEgg,
                OddlySmoothStone,
                NutritiousSoup,
                Gorget,
                MealTicket,
                Vajra,
            ]
            .into_iter()
            .collect(),
        },
        |hp| hp == [48, 42, 86],
        // |_| true,
        &bump,
    );

    let mut state = pre_first_turn_state;

    let mut engine: micro_engine::MicroEngine<TestEngineCurrentHp> =
        micro_engine::MicroEngine::new(TestEngineCurrentHp {});

    let mut comm = Comm::new();

    loop {
        state.dedup();
        let real_state_res = comm
            .find_valid_combat_states(state.entries.drain(..).map(|(v, _)| v).collect_in(&bump));
        std::mem::drop(state);

        let real_state = real_state_res.unwrap();

        // dbg!(
        //     &real_state.enemies[0]
        //         .prototype
        //         .get_moveset(&bump)
        //         .eval(&real_state.enemies[0].state_machine)
        // );

        let real_state = real_state.launder(&temp_bump);
        bump.reset();
        let real_state = real_state.launder(&bump);
        temp_bump.reset();

        let action = engine.next_combat_action(
            &real_state,
            99,
            Duration::from_secs(10),
            |msg| {
                eprintln!("{msg}");
            },
            &bump,
        );

        let Some(action) = action.0 else {
            break;
        };

        // if let CombatAction::PlayCard { card, .. } = action
        //     && card.prototype.get_kind() == CardKind::Skill
        // {
        //     let has_attack = real_state
        //         .player
        //         .hand
        //         .iter()
        //         .any(|card| card.prototype.get_kind() == CardKind::Attack);

        //     let enemy_is_attacking = real_state.enemies[0]
        //         .prototype
        //         .get_moveset()
        //         .eval(&real_state.enemies[0].state_machine)
        //         .actions
        //         .iter()
        //         .any(|action| matches!(action, game_state::EnemyAction::Attack { .. }));

        //     assert!(
        //         !has_attack || enemy_is_attacking,
        //         "Unneeded defend over attacking in state {real_state:?}"
        //     );
        // }

        state = real_state.apply(action, &bump);
        comm.apply_action(action);

        // After applying the action on the game, we need to wait for stuff to settle (I do not know what the game returns while the animations are playing)
        let start = Instant::now();
        let presumed_done = start + Duration::from_secs(4);

        // Use the time on calcs insread of just waiting
        if state.entries.len() == 1 {
            let assumed_next_state = &state.entries.first().unwrap().0;
            engine.next_combat_action(
                assumed_next_state,
                99,
                Duration::from_secs(4),
                |_| {},
                &bump,
            );
        } else {
            engine.next_combat_action(&real_state, 99, Duration::from_secs(4), |_| {}, &bump);
        }

        thread::sleep(presumed_done - start);
    }

    println!("No more action, is the fight over?");
}
