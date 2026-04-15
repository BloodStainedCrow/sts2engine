use crate::combat_state::cards::Card;

mod card_rarity_odds;
mod events;
mod mcts;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunState {
    current_act: Act,

    deck: Vec<Card>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunAction {
    ChooseNode { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Act {
    prototype: ActPrototype,

    map: Map,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ActPrototype {
    Overgrowth,
    Underdocks,
    Hive,
    Glory,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Map {
    layers: Vec<Vec<Node>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    // The Node indices of the next layer
    parents: Vec<usize>,

    kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeKind {
    Ancient,
    Unknown,
    Merchant,
    Trasure,
    Rest,
    Enemy,
    Elite,
    Boss,
}
