use crate::game_state::cards::Card;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatAction {
    PlayCard { card: Card, target: Option<u8> },

    Choice { card: Card },

    UsePotion { index: u8 },

    EndTurn,
}
