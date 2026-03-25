#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombatAction {
    PlayCard { index: u8, target: Option<u8> },

    UsePotion { index: u8 },

    EndTurn,
}
