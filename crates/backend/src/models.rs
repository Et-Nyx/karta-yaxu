use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trainer {
    pub id: String, // Guid (uuid)
    pub username: String,
    pub password_hash: String,
    pub role: String, // "viewer" | "editor"
    pub is_npc: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Team {
    pub id: i64,
    pub trainer_id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamMember {
    pub id: i64,
    pub team_id: i64,
    pub slot: i64,
    pub pokemon_id: i64,
    pub ability_id: i64,
    pub nature_id: i64,
    
    // Slots opcionais (podem vir nulos do banco de dados)
    pub held_item_id: Option<i64>,
    pub move1_id: Option<i64>,
    pub move2_id: Option<i64>,
    pub move3_id: Option<i64>,
    pub move4_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Battle {
    pub id: i64,
    pub trainer_id_1: String,
    pub trainer_id_2: String,
    pub team_id_1: i64,
    pub team_id_2: i64,
    pub winner_id: Option<String>,
    pub log: String,
    pub played_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Edit {
    pub id: i64,
    pub trainer_id: String,
    pub table_name: String,
    pub record_id: i64,
    pub diff: String,
    pub edited_at: String,
}
