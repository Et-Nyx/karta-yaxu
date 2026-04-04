use crate::models::{Ability, Item, Move, Pokemon, StatusEffect};

#[derive(Debug, Default, Clone)]
pub struct StatStages {
    pub atk: i8,
    pub def: i8,
    pub spa: i8,
    pub spd: i8,
    pub spe: i8,
    pub accuracy: i8,
    pub evasion: i8,
}

#[derive(Debug, Default, Clone)]
pub struct VolatileStatus {
    pub is_flinched: bool,
    pub confused_turns: u8,
    pub leech_seeded: bool,
}

#[derive(Debug, Clone)]
pub struct BattlePokemon {
    pub base: Pokemon,
    pub active_ability: Ability,
    pub held_item: Option<Item>,
    
    pub moves: Vec<Move>,
    pub current_hp: i64,
    pub status: Option<StatusEffect>,
    
    pub stat_stages: StatStages,
    pub volatile: VolatileStatus,
}

#[derive(Debug, Clone)]
pub struct BattleTeam {
    pub members: Vec<BattlePokemon>,
    pub active_index: usize, // Quem tá em campo agora?
}

#[derive(Debug, Clone)]
pub struct BattleState {
    pub turn: u32,
    pub weather: Option<String>, // Pode virar Enum dps: "Rain", "Sun", etc.
    pub terrain: Option<String>, 
    pub team1: BattleTeam,
    pub team2: BattleTeam,
}
