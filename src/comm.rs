use std::{ops::ControlFlow, vec};

use itertools::Itertools;
use rcon_client::{AuthRequest, RCONClient, RCONConfig, RCONRequest};
use strum::IntoEnumIterator;

use crate::{
    combat_action::CombatAction,
    distribution::{self, Distribution},
    game_state::{
        self, CombatState, Enemy, EnemyAction, EnemyPrototype, Player, RunInfo, Status,
        cards::{Card, CardEnchantment, CardPrototype, UnorderedCardSet},
        encounter::EncounterPrototype,
    },
};

pub struct Comm {
    rcon: RCONComm,
}

#[derive(Debug)]
pub struct NoValidOptionError;

impl Comm {
    pub fn new() -> Self {
        Self {
            rcon: RCONComm::new(),
        }
    }

    pub fn filter_hp(&mut self, hp: &[u16]) -> bool {
        let real = self.rcon.enemies();

        real.len() == hp.len() && real.iter().zip(hp).all(|(real, hp)| real.max_hp == *hp)
    }

    fn get_combat_state(&self) -> Vec<CombatState> {
        todo!()
    }

    pub fn guess_encounter(&mut self) -> EncounterPrototype {
        let enemies: Vec<EnemyInfo> = self.rcon.enemies();
        let enemies_ref = &enemies;

        let options = EncounterPrototype::iter()
            .filter(|encounter| {
                // TODO: Only for testing
                encounter.is_finished_implementing()
            })
            .map(|encounter_prototype| {
                (
                    encounter_prototype,
                    game_state::CombatState::get_starting_states::<
                        distribution::full::Distribution<_>,
                    >(
                        encounter_prototype,
                        &RunInfo {
                            hp: 70,
                            max_hp: 70,
                            deck: vec![],
                            relic_state: [].into_iter().collect(),
                        },
                        |_| true,
                    ),
                )
            });

        let encounter_and_matchscore =
            options.map(|(encounter, dis)| {
                (
                    encounter,
                    dis.map(move |state| {
                        if enemies_ref.iter().all(|real| {
                            state
                                .enemies
                                .iter()
                                .any(|enemy| enemy.prototype == real.kind)
                        }) && state.enemies.iter().all(|enemy| {
                            enemies_ref.iter().any(|real| real.kind == enemy.prototype)
                        }) {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .expected_value(),
                )
            });

        encounter_and_matchscore
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(v, _score)| v)
            .expect("There will always be at least one encounter prototype")
    }

    pub fn get_run_state(&mut self) -> RunInfo {
        use game_state::relics::RelicPrototype::*;
        RunInfo {
            hp: self.rcon.get_hp(),
            max_hp: self.rcon.get_max_hp(),
            deck: self
                .rcon
                .full_deck()
                .into_iter()
                .map(|card| Card {
                    prototype: card.kind,
                    upgraded: card.upgraded,
                    enchantment: card.enchantment,
                })
                .collect(),
            relic_state: [RingOfTheSnake].into_iter().collect(),
        }
    }

    pub fn find_valid_combat_states(
        &mut self,
        options: Vec<CombatState>,
    ) -> Result<CombatState, NoValidOptionError> {
        fn inner(
            comm: &mut RCONComm,
            mut options: Vec<CombatState>,
        ) -> ControlFlow<Result<CombatState, NoValidOptionError>> {
            dbg!(options.first());
            // let mut options = is_single_choice(options)?;
            println!("Start: {}", options.len());

            let hand = comm.hand();
            options.retain(|state| state.player.hand.satisfies(&hand));
            if options.is_empty() {
                dbg!(&hand);
            }
            println!("Post hand: {}", options.len());

            let player_hp = comm.get_hp();
            options.retain(|state| state.player.creature.hp == player_hp);
            println!("Post player hp: {}", options.len());
            // let mut options = is_single_choice(options)?;

            let player_block = comm.get_block();
            options.retain(|state| state.player.creature.block == player_block);
            println!("Post player block: {}", options.len());
            // let mut options = is_single_choice(options)?;

            let enemies = comm.enemies();
            // FIXME: DEBUG
            let old_options = options.clone();
            options.retain(|state| {
                if state.enemies.len() != enemies.len() {
                    return false;
                }

                state
                    .enemies
                    .iter()
                    .zip(enemies.iter())
                    .all(|(enemy, info)| info.satisfies(enemy))
            });
            if options.is_empty() {
                let intents = old_options
                    .into_iter()
                    .map(|state| {
                        state
                            .enemies
                            .into_iter()
                            .map(|enemy| (enemy.prototype, enemy.state_machine))
                            .collect_vec()
                    })
                    .collect_vec();
                // dbg!(&intents);
            }
            println!("Post enemies: {}", options.len());
            // let mut options = is_single_choice(options)?;

            let draw_pile = comm.draw_pile();
            options.retain(|state| state.player.draw_pile.satisfies(&draw_pile));
            if options.is_empty() {
                dbg!(&draw_pile);
            }
            println!("Post Draw pile: {}", options.len());

            // FIXME: I currently do the discard pile wrong for cards with choices.
            // let discard_pile = comm.discard_pile();
            // options.retain(|state| state.player.discard_pile.satisfies(&discard_pile));
            // if options.is_empty() {
            //     dbg!(&discard_pile);
            // }
            // println!("Post Discard pile: {}", options.len());

            // TODO: Other piles (draw/discard/exhaust)

            let player = comm.get_player_info();
            options.retain(|state| player.satisfies(&state.player));
            if options.is_empty() {
                dbg!(player);
            }
            println!("Post statuses: {}", options.len());

            let remaining_options = is_single_choice(options)?;
            dbg!(remaining_options);
            todo!("Could not limit to single state")
        }

        assert!(options.iter().all_unique());

        let res = inner(&mut self.rcon, options);

        res.break_value().unwrap_or_else(|| unreachable!())
    }

    pub fn apply_action(&mut self, action: CombatAction) {
        self.rcon.apply_action(action);
    }
}

fn is_single_choice<T>(mut options: Vec<T>) -> ControlFlow<Result<T, NoValidOptionError>, Vec<T>> {
    if options.len() <= 1 {
        if options.is_empty() {
            ControlFlow::Break(Err(NoValidOptionError))
        } else {
            ControlFlow::Break(Ok(options.pop().expect("Checked before")))
        }
    } else {
        ControlFlow::Continue(options)
    }
}

struct RCONComm {
    client: rcon_client::RCONClient,
}

#[derive(Debug, serde::Deserialize)]
struct EnemyInfo {
    kind: EnemyPrototype,
    current_hp: u16,
    max_hp: u16,
    block: u16,

    powers: vec::Vec<(Status, i16)>,

    intent: vec::Vec<IntentInfo>,
}

#[derive(Debug, serde::Deserialize)]
struct PlayerCreatureInfo {
    kind: String,
    current_hp: u16,
    max_hp: u16,
    block: u16,

    powers: vec::Vec<(Status, i16)>,
}

impl PlayerCreatureInfo {
    fn satisfies(&self, player: &Player) -> bool {
        if self.current_hp != player.creature.hp {
            return false;
        }

        if self.max_hp != player.creature.max_hp {
            return false;
        }

        if self.block != player.creature.block {
            return false;
        }

        let all_real_present = self
            .powers
            .iter()
            .all(|(power, amount)| player.creature.statuses[*power] == *amount);

        if !all_real_present {
            return false;
        }

        let all_sim_present = player
            .creature
            .statuses
            .iter()
            .all(|(power, amount)| *amount == 0 || self.powers.contains(&(power, *amount)));

        if !all_sim_present {
            return false;
        }

        true
    }
}

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(tag = "kind")]
enum IntentInfo {
    Attack { damage: u16, repeat: u16 },
    Buff {},
    Debuff {},
    DebuffStrong {},
    Defend {},
    StatusCard { count: u8 },
    Stun {},
    Sleep {},
}

impl EnemyInfo {
    fn satisfies(&self, enemy: &Enemy) -> bool {
        if enemy.prototype != self.kind {
            return false;
        }

        if enemy.creature.hp != self.current_hp {
            return false;
        }

        if enemy.creature.max_hp != self.max_hp {
            return false;
        }

        if enemy.creature.block != self.block {
            return false;
        }

        // TODO: Statuses
        let all_real_present = self
            .powers
            .iter()
            .all(|(power, amount)| enemy.creature.statuses[*power] == *amount);

        if !all_real_present {
            return false;
        }

        let all_sim_present = enemy
            .creature
            .statuses
            .iter()
            .all(|(power, amount)| *amount == 0 || self.powers.contains(&(power, *amount)));

        if !all_sim_present {
            return false;
        }

        // TODO: is_alone
        let intent = enemy.prototype.get_moveset().eval(
            &enemy.state_machine,
            &enemy.creature.statuses,
            false,
        );

        // dbg!(&self.intent);
        // dbg!((enemy.prototype, &enemy.state_machine, intent));

        if !intent.actions.iter().all(|v| match v {
            crate::game_state::EnemyAction::Attack {
                base_damage: _,
                repeat,
            } => self.intent.iter().any(|intent| {
                matches!(
                    intent,
                    IntentInfo::Attack {
                        damage: _,
                        repeat: comm_repeat,
                    } if repeat == comm_repeat
                )
            }),
            crate::game_state::EnemyAction::Block { .. } => {
                self.intent.contains(&IntentInfo::Defend {})
            }
            crate::game_state::EnemyAction::ApplyStatusSelf { diff, .. } => {
                // Some enemies remove buffs from themselves. This is not communicated via the intent system (i.e. Spiny Toad removing its own Thorns)
                *diff <= 0 || self.intent.contains(&IntentInfo::Buff {})
            }
            crate::game_state::EnemyAction::ApplyStatusPlayer { .. } => {
                self.intent.contains(&IntentInfo::Debuff {})
                    || self.intent.contains(&IntentInfo::DebuffStrong {})
            }
            crate::game_state::EnemyAction::ShuffleCards { count, .. } => self
                .intent
                .contains(&IntentInfo::StatusCard { count: *count }),
        }) {
            dbg!((enemy.prototype, intent));
            return false;
        }

        if !self.intent.iter().all(|intent_info| match intent_info {
            IntentInfo::Attack { repeat, .. } => intent.actions.iter().any(|action| {
                matches!(
                    action,
                    EnemyAction::Attack {
                        base_damage: _,
                        repeat: comm_repeat,
                    } if repeat == comm_repeat
                )
            }),
            IntentInfo::Buff {} => intent
                .actions
                .iter()
                .any(|action| matches!(action, EnemyAction::ApplyStatusSelf { .. })),
            IntentInfo::Debuff {} => intent
                .actions
                .iter()
                .any(|action| matches!(action, EnemyAction::ApplyStatusPlayer { .. })),
            IntentInfo::DebuffStrong {} => intent
                .actions
                .iter()
                .any(|action| matches!(action, EnemyAction::ApplyStatusPlayer { .. })),
            IntentInfo::Defend {} => intent
                .actions
                .iter()
                .any(|action| matches!(action, EnemyAction::Block { .. })),
            IntentInfo::StatusCard { count } => intent
                .actions
                .iter()
                .any(|action| matches!(action, EnemyAction::ShuffleCards { count: comm_count, .. } if count == comm_count)),
            IntentInfo::Stun {} => {
                intent.actions.is_empty()
            },
            IntentInfo::Sleep {} => {
                intent.actions.is_empty()
            },
        }) {
            dbg!(intent);
            return false;
        }

        true
    }
}

#[derive(Debug, serde::Deserialize)]
struct CardInfo {
    kind: CardPrototype,
    upgraded: bool,
    enchantment: Option<CardEnchantment>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CardState {
    draw: vec::Vec<CardInfo>,
    hand: vec::Vec<CardInfo>,
    discard: vec::Vec<CardInfo>,
    exhaust: vec::Vec<CardInfo>,
    play: vec::Vec<CardInfo>,
    deck: vec::Vec<CardInfo>,
}

impl UnorderedCardSet {
    fn satisfies(&self, cards: &[CardInfo]) -> bool {
        let counts: std::collections::HashMap<(CardPrototype, bool), usize> =
            cards.iter().map(|card| (card.kind, card.upgraded)).counts();
        let state_counts: std::collections::HashMap<(CardPrototype, bool), usize> = self
            .cards
            .iter()
            .filter_map(|(card, count)| {
                (*count > 0).then_some(((card.prototype, card.upgraded), *count as usize))
            })
            .collect();

        // dbg!(&state_counts);

        counts == state_counts
    }
}

impl RCONComm {
    fn new() -> Self {
        let mut client = RCONClient::new(RCONConfig {
            url: "127.0.0.1:27015".to_string(),
            write_timeout: None,
            read_timeout: None,
        })
        .expect("Unable to connect");

        client
            .auth(AuthRequest::new("changeme".to_string()))
            .expect("Failed auth");
        Self { client }
    }

    fn get_hp(&mut self) -> u16 {
        let res = self
            .client
            .execute(RCONRequest::new("get_hp".to_owned()))
            .expect("RCON Failed")
            .body;

        res.parse().expect("Could not parse HP")
    }

    fn get_max_hp(&mut self) -> u16 {
        let res = self
            .client
            .execute(RCONRequest::new("get_max_hp".to_owned()))
            .expect("RCON Failed")
            .body;

        res.parse().expect("Could not parse HP")
    }

    fn get_block(&mut self) -> u16 {
        let res = self
            .client
            .execute(RCONRequest::new("get_combat_player_state".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<PlayerCreatureInfo, _> = serde_path_to_error::deserialize(jd);
        let creature = match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{}, with source: {res}", err.path());
            }
        };

        creature.block
    }

    fn get_player_info(&mut self) -> PlayerCreatureInfo {
        let res = self
            .client
            .execute(RCONRequest::new("get_combat_player_state".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<PlayerCreatureInfo, _> = serde_path_to_error::deserialize(jd);
        let creature = match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{}, with source: {res}", err.path());
            }
        };

        creature
    }

    fn enemies(&mut self) -> vec::Vec<EnemyInfo> {
        let res = self
            .client
            .execute(RCONRequest::new("get_enemies".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<_, _> = serde_path_to_error::deserialize(jd);
        match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{}, with source: {res}", err.path());
            }
        }
    }

    fn hand(&mut self) -> vec::Vec<CardInfo> {
        let res = self
            .client
            .execute(RCONRequest::new("get_combat_card_state".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<CardState, _> = serde_path_to_error::deserialize(jd);
        let state = match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{} with source: {res}", err.path());
            }
        };

        state.hand
    }

    fn full_deck(&mut self) -> vec::Vec<CardInfo> {
        let res = self
            .client
            .execute(RCONRequest::new("get_combat_card_state".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<CardState, _> = serde_path_to_error::deserialize(jd);
        let state = match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{} with source: {res}", err.path());
            }
        };

        state.deck
    }

    fn draw_pile(&mut self) -> vec::Vec<CardInfo> {
        let res = self
            .client
            .execute(RCONRequest::new("get_combat_card_state".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<CardState, _> = serde_path_to_error::deserialize(jd);
        let state = match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{} with source: {res}", err.path());
            }
        };

        state.draw
    }

    fn discard_pile(&mut self) -> vec::Vec<CardInfo> {
        let res = self
            .client
            .execute(RCONRequest::new("get_combat_card_state".to_owned()))
            .expect("RCON Failed")
            .body;

        let jd = &mut serde_json::Deserializer::from_str(&res);

        let result: Result<CardState, _> = serde_path_to_error::deserialize(jd);
        let state = match result {
            Ok(v) => v,
            Err(err) => {
                panic!("{} with source: {res}", err.path());
            }
        };

        state.discard
    }

    fn apply_action(&mut self, action: CombatAction) {
        let command = match action {
            CombatAction::PlayCard { card, target } => {
                let hand = self.hand();

                let card_index = hand
                    .iter()
                    .position(|info| {
                        info.kind == card.prototype
                            && info.upgraded == card.upgraded
                            && info.enchantment == card.enchantment
                    })
                    .unwrap_or_else(|| panic!("Could not find card {card:?} in game hand"));

                match target {
                    Some(target) => format!("play_hand_card {card_index} {target}"),
                    None => format!("play_hand_card {card_index}"),
                }
            }
            CombatAction::UsePotion { index } => todo!(),
            CombatAction::Choice { card } => {
                let hand = self.hand();

                let card_index = hand
                    .iter()
                    .position(|info| {
                        info.kind == card.prototype
                            && info.upgraded == card.upgraded
                            && info.enchantment == card.enchantment
                    })
                    .unwrap_or_else(|| panic!("Could not find card {card:?} in game hand"));

                format!("choose_card_from_hand {card_index}")
            }
            CombatAction::EndTurn => "end_turn".to_string(),
        };

        let res = self
            .client
            .execute(RCONRequest::new(command))
            .expect("RCON Failed")
            .body;

        assert_eq!("OK", res);
    }
}
