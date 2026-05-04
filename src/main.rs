#![feature(iterator_try_collect)]
#![feature(allocator_api)]
#![feature(vec_try_remove)]
#![feature(never_type)]
#![feature(associated_type_defaults)]

use mimalloc::MiMalloc;
use sts2mcts::mcts::ParMCTS;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::{
    io::{Read, stdin},
    thread,
    time::{Duration, Instant},
};

use itertools::Itertools;

use crate::{
    combat_state::{
        CombatState, Creature, Enemy, Player, PostCombatState, RunInfo, Status,
        cards::{Card, CardPrototype},
        relics::FullRelicState,
    },
    comm::Comm,
    distribution::Distribution,
    micro_engine::{EvalResult, EvaluationFunction},
};

mod combat_action;
mod combat_state;
mod comm;
mod distribution;
mod mcts;
mod micro_engine;
mod run_state;

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
    dbg!(size_of::<CombatState>());
    dbg!(size_of::<Player>());
    dbg!(size_of::<Enemy>());
    dbg!(size_of::<Creature>());
    dbg!(size_of::<FullRelicState>());

    let mut comm = Comm::new();
    loop {
        let pre_first_turn_state = combat_state::CombatState::get_starting_states::<
            FullFamily,
            _,
            _,
        >(comm.guess_encounter(), &comm.get_run_state(), |hps| {
            comm.filter_hp(hps)
        });

        let mut state = pre_first_turn_state;

        let real_state_res = comm.find_valid_combat_states(state.into_values().collect());
        let Ok(mut real_state) = real_state_res else {
            panic!("No valid option for starting state. Are the relics correct?");
        };

        if real_state.get_post_game_state().is_none() {
            // Run the engine
            let mut engine: ParMCTS<CombatState> = ParMCTS::new(real_state.clone());

            loop {
                // TODO: Maybe dont discard the full game tree, since previous work is still valuable?
                // This does significantly increase maximum memory consumption
                // engine.advance(&real_state);
                engine.clear_advance(&real_state);

                let action = engine.par_search(Duration::from_secs(5));
                dbg!(engine.principal_chain().collect_vec());
                dbg!(
                    sts2mcts::mcts::NODES_CHECKED
                        .fetch_min(0, std::sync::atomic::Ordering::Relaxed)
                );
                dbg!(action);

                comm.apply_action(action);
                let animations_done = Instant::now() + Duration::from_secs(2);
                state = real_state.apply(action);

                // After applying the action on the game, we need to wait for stuff to settle (I do not know what the game returns while the animations are playing)
                // TODO: Use the time on calcs instead of just waiting
                // engine.par_search(Duration::from_secs(4));
                // engine.trim_pre_advance();
                engine.clear();

                thread::sleep(animations_done - Instant::now());

                state.dedup();
                let real_state_res = comm.find_valid_combat_states(state.into_values().collect());

                let Ok(new_real_state) = real_state_res else {
                    println!("No valid option!");
                    // Lets assume we failed because the combat is over
                    break;
                };

                if new_real_state.get_post_game_state().is_some() {
                    break;
                }

                real_state = new_real_state;
            }
        }

        println!("No more action, is the fight over?");
        let _ = stdin().read(&mut [0]).expect("Failed to read from stdin");
    }
}

fn run_expectimax() {
    use combat_state::relics::RelicPrototype::*;

    // TODO: Assume specific fight
    let pre_first_turn_state = combat_state::CombatState::get_starting_states::<
        distribution::full::Distribution<_>,
        &FullRelicState,
        &[Card],
    >(
        combat_state::encounter::EncounterPrototype::RubyRaiders,
        &RunInfo {
            hp: 68,
            max_hp: 70,
            deck: &[
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

                    ..Card::default()
                },
                CardPrototype::Survivor.get_normal_card(),
                CardPrototype::DodgeAndRoll.get_normal_card(),
                Card {
                    prototype: CardPrototype::DaggerSpray,
                    upgraded: true,

                    ..Card::default()
                },
            ],
            relic_state: &[(RingOfTheSnake, 0), (Pomander, 0)].into_iter().collect(),
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
        // engine.next_combat_action(&real_state, 99, Duration::from_secs(4), |_| {});

        thread::sleep(presumed_done - start);
    }

    println!("No more action, is the fight over?");
}

#[cfg(test)]
mod test {
    use std::{borrow::Borrow, iter::once, time::Duration};

    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    use strum::IntoEnumIterator;
    use sts2mcts::mcts::MCTS;

    use crate::{
        combat_state::{
            self, RunInfo,
            cards::{Card, CardPrototype},
            encounter::EncounterPrototype,
            relics::{FullRelicState, RelicPrototype},
        },
        distribution::{self, Distribution},
    };

    fn eval_across_encounters(
        run_state: &RunInfo<FullRelicState, Vec<Card>>,
        encounter_filter: impl Fn(&EncounterPrototype) -> bool,
    ) -> f32 {
        let num_starts_per_encounter = rayon::current_num_threads();

        let mut total_eval: f32 = 0.0;

        for encounter in EncounterPrototype::iter()
            .filter(|encounter| encounter.is_finished_implementing())
            .filter(|e| (encounter_filter)(e))
        {
            dbg!(encounter);
            total_eval += (0..1)
                .into_par_iter()
                .map(|_| {
                    let starting_state = combat_state::CombatState::get_starting_states::<
                        distribution::single::Distribution<_>,
                        _,
                        _,
                    >(encounter, run_state, |_hps| true)
                    .collapse();

                    let mut engine = MCTS::new(starting_state);

                    engine.search(Duration::from_secs_f64(0.1));

                    engine.expected_eval()
                })
                .sum::<f32>();
        }

        dbg!(total_eval)
    }

    #[test]
    fn best_card_to_add() {
        let run_state = RunInfo {
            hp: 49,
            max_hp: 70,
            deck: vec![
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Strike.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Defend.get_normal_card(),
                CardPrototype::Neutralize.get_normal_card().upgraded(),
                CardPrototype::Survivor.get_normal_card(),
                CardPrototype::DodgeAndRoll.get_normal_card().upgraded(),
                CardPrototype::DaggerSpray.get_normal_card().upgraded(),
                CardPrototype::CloakAndDagger.get_normal_card().upgraded(),
                CardPrototype::PiercingWail.get_normal_card(),
                CardPrototype::NoxiousFumes.get_normal_card().upgraded(),
                CardPrototype::PiercingWail.get_normal_card(),
                CardPrototype::CloakAndDagger.get_normal_card().upgraded(),
                CardPrototype::LegSweep.get_normal_card().upgraded(),
                CardPrototype::Footwork.get_normal_card().upgraded(),
                CardPrototype::PoisonedStab.get_normal_card().upgraded(),
                CardPrototype::PoisonedStab.get_normal_card(),
                CardPrototype::DeadlyPoison.get_normal_card(),
                CardPrototype::Blur.get_normal_card(),
            ],
            relic_state: [
                (RelicPrototype::RingOfTheDrake, 0),
                (RelicPrototype::Vajra, 0),
                (RelicPrototype::OddlySmoothStone, 0),
                (RelicPrototype::MeatOnTheBone, 0),
                (RelicPrototype::HornCleat, 0),
                (RelicPrototype::MrStruggles, 0),
                (RelicPrototype::BagOfMarbles, 0),
                (RelicPrototype::Candelabra, 0),
                (RelicPrototype::Sai, 0),
            ]
            .into_iter()
            .collect(),
        };

        let past_encounters = [
            // EncounterPrototype::ShrinkerBeetle,
            // EncounterPrototype::SlimesWeak,
            // EncounterPrototype::SingleNibbit,
            // EncounterPrototype::FuzzyWurmCrawler,
            // EncounterPrototype::PhrogParasite,
            // EncounterPrototype::BygoneEffigy,
            // EncounterPrototype::Byrdonis,
            // EncounterPrototype::Vantom,
            // EncounterPrototype::DoubleNibbit,
            // EncounterPrototype::BeetleAndFuzzy,
            // EncounterPrototype::SingleCubexConstruct,
            // EncounterPrototype::RubyRaiders,
            // EncounterPrototype::TheKin,
            // EncounterPrototype::SoloTunneler,
            // EncounterPrototype::BowlbugsWeak,
            // EncounterPrototype::BowlbugsStrong,
            // EncounterPrototype::InfestedPrism,
            // EncounterPrototype::SpinyToad,
            // EncounterPrototype::Entomancer,
            // EncounterPrototype::DevotedSculptor,
            // EncounterPrototype::TurretOperator,
            // EncounterPrototype::SoulNexus,
            // EncounterPrototype::OwlMagistrate,
            // EncounterPrototype::MechaKnight,
            // EncounterPrototype::Knights,
            // EncounterPrototype::SlimedBerserker,
            // EncounterPrototype::ConstructGang,
        ];

        // let cards: [Vec<_>; _] = [vec![CardPrototype::Apotheosis.get_normal_card().upgraded()]];

        let cards: Vec<Vec<_>> = CardPrototype::iter()
            .filter(|c| *c != CardPrototype::FranticEscape)
            .map(|p| vec![p.get_normal_card()])
            .collect();

        let best_card = cards
            .into_iter()
            .map(|card| {
                let mut state = run_state.clone();

                for card in &card {
                    dbg!(card);
                    state.deck.push(*card);
                }

                (
                    card,
                    eval_across_encounters(&state, |e| !past_encounters.contains(e)),
                )
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;

        dbg!(best_card);
    }
}
