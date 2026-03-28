use std::ops::ControlFlow;

use itertools::Itertools;
use rcon_client::{AuthRequest, RCONClient, RCONConfig, RCONRequest};

use crate::{
    combat_action::CombatAction,
    distribution::Distribution,
    game_state::{
        CombatState, Enemy, EnemyPrototype, Status,
        cards::{CardPrototype, UnorderedCardSet},
    },
};

pub struct Comm {
    rcon: RCONComm,
}

pub struct NoValidOptionError;

impl Comm {
    pub fn new() -> Self {
        Self {
            rcon: RCONComm::new(),
        }
    }

    fn get_combat_state(&self) -> Vec<CombatState> {
        todo!()
    }

    pub fn find_valid_combat_states(
        &mut self,
        options: Vec<CombatState>,
    ) -> Result<CombatState, NoValidOptionError> {
        fn inner(
            comm: &mut RCONComm,
            mut options: Vec<CombatState>,
        ) -> ControlFlow<Result<CombatState, NoValidOptionError>> {
            // dbg!(options.first());
            // let mut options = is_single_choice(options)?;

            println!("Pre player hp: {}", options.len());
            let player_hp = comm.get_hp();
            options.retain(|state| state.player.creature.hp == player_hp);
            println!("Post player hp: {}", options.len());
            // let mut options = is_single_choice(options)?;

            let player_block = comm.get_block();
            options.retain(|state| state.player.creature.block == player_block);
            println!("Post player block: {}", options.len());
            // let mut options = is_single_choice(options)?;

            let enemies = comm.enemies();
            options.retain(|state| {
                state
                    .enemies
                    .iter()
                    .zip(enemies.iter())
                    .all(|(enemy, info)| info.satisfies(enemy))
            });
            println!("Post enemies: {}", options.len());
            // let mut options = is_single_choice(options)?;

            let hand = comm.hand();
            options.retain(|state| state.player.hand.satisfies(&hand));
            println!("Post hand: {}", options.len());

            // TODO: Other piles (draw/discard/exhaust)

            let remaining_options = is_single_choice(options)?;
            dbg!(remaining_options);
            todo!("Could not limit to single state")
        }

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
            ControlFlow::Break(Ok(options.pop().unwrap()))
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

    powers: Vec<(Status, i16)>,

    intent: Vec<IntentInfo>,
}

#[derive(Debug, serde::Deserialize)]
struct PlayerCreatureInfo {
    kind: String,
    current_hp: u16,
    max_hp: u16,
    block: u16,

    powers: Vec<(Status, i16)>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "kind")]
enum IntentInfo {
    Attack { damage: u16, repeat: u16 },
    Buff {},
    DebuffStrong {},
    Defend {},
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

        // TODO: Intent

        true
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct CardInfo {
    kind: CardPrototype,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CardState {
    draw: Vec<CardInfo>,
    hand: Vec<CardInfo>,
    discard: Vec<CardInfo>,
    exhaust: Vec<CardInfo>,
    play: Vec<CardInfo>,
    deck: Vec<CardInfo>,
}

impl UnorderedCardSet {
    fn satisfies(&self, cards: &[CardInfo]) -> bool {
        dbg!(self);
        let counts: std::collections::HashMap<CardPrototype, usize> =
            cards.iter().map(|card| card.kind).counts();
        let state_counts: std::collections::HashMap<CardPrototype, usize> = self
            .cards
            .iter()
            .filter_map(|(card, count)| (*count > 0).then_some((card.prototype, *count as usize)))
            .collect();

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

    fn enemies(&mut self) -> Vec<EnemyInfo> {
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

    fn hand(&mut self) -> Vec<CardInfo> {
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

    fn apply_action(&mut self, action: CombatAction) {
        let command = match action {
            CombatAction::PlayCard { card, target } => {
                let hand = self.hand();

                let card_index = hand
                    .iter()
                    .position(|info| info.kind == card.prototype)
                    .unwrap_or_else(|| panic!("Could not find card {card:?} in game hand"));

                match target {
                    Some(target) => format!("play_hand_card {card_index} {target}"),
                    None => format!("play_hand_card {card_index}"),
                }
            }
            CombatAction::UsePotion { index } => todo!(),
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
