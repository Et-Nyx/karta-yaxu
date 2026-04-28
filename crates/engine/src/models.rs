use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Type {
    pub id: i64,
    pub name: String,
    pub hexcolor: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Ability {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(type_name = "TEXT", rename_all = "lowercase"))]
#[serde(rename_all = "snake_case")]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

impl std::fmt::Display for MoveCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveCategory::Physical => write!(f, "Físico"),
            MoveCategory::Special => write!(f, "Especial"),
            MoveCategory::Status => write!(f, "Status"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StatusCondition {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub is_volatile: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusEffect {
    Poison,
    BadPoison,
    Paralysis,
    Burn,
    Sleep,
    Freeze,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Move {
    pub id: i64,
    pub name: String,
    pub type_id: i64,
    pub category: MoveCategory,
    pub power: Option<i64>,
    pub accuracy: Option<i64>,
    pub pp: i64,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Pokemon {
    pub id: i64,
    pub dex_number: i64,
    pub name: String,
    pub sprite: String,
    pub sprite_shiny: String,
    pub type1_id: i64,
    pub type2_id: Option<i64>,
    pub hp: i64,
    pub atk: i64,
    pub def: i64,
    pub spa: i64,
    pub spd: i64,
    pub spe: i64,
    pub pokedex_entry: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Nature {
    pub id: i64,
    pub name: String,
    pub increased_stat: Option<String>,
    pub decreased_stat: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct TypeEffectiveness {
    pub attacker_id: i64,
    pub defender_id: i64,
    pub multiplier: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct PokemonAbility {
    pub pokemon_id: i64,
    pub ability_id: i64,
    pub slot: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Evolution {
    pub from_id: i64,
    pub to_id: i64,
    pub condition: String,
    pub condition_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct PokemonForm {
    pub base_id: i64,
    pub form_id: i64,
    pub form_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(type_name = "TEXT", rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum LearnMethod {
    LevelUp,
    Tm,
    Egg,
    Tutor,
    Gmax,
    Max,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Learnset {
    pub pokemon_id: i64,
    pub move_id: i64,
    pub method: LearnMethod,
    pub level: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_category_serialization() {
        assert_eq!(serde_json::to_string(&MoveCategory::Physical).unwrap(), "\"physical\"");
        assert_eq!(serde_json::to_string(&MoveCategory::Special).unwrap(), "\"special\"");
        assert_eq!(serde_json::to_string(&MoveCategory::Status).unwrap(), "\"status\"");
        
        let cat: MoveCategory = serde_json::from_str("\"physical\"").unwrap();
        assert_eq!(cat, MoveCategory::Physical);
    }

    #[test]
    fn test_learn_method_serialization() {
        assert_eq!(serde_json::to_string(&LearnMethod::LevelUp).unwrap(), "\"level_up\"");
        assert_eq!(serde_json::to_string(&LearnMethod::Tm).unwrap(), "\"tm\"");
        
        let method: LearnMethod = serde_json::from_str("\"level_up\"").unwrap();
        assert_eq!(method, LearnMethod::LevelUp);
    }

    #[test]
    fn test_pokemon_serialization() {
        let p = Pokemon {
            id: 1,
            dex_number: 1,
            name: "Bulbasaur".to_string(),
            sprite: "bulbasaur.png".to_string(),
            sprite_shiny: "bulbasaur_shiny.png".to_string(),
            type1_id: 1,
            type2_id: Some(2),
            hp: 45, atk: 49, def: 49, spa: 65, spd: 65, spe: 45,
            pokedex_entry: "Seed Pokemon".to_string(),
        };
        
        let json = serde_json::to_string(&p).unwrap();
        let p2: Pokemon = serde_json::from_str(&json).unwrap();
        assert_eq!(p.name, p2.name);
        assert_eq!(p.type2_id, p2.type2_id);
    }
}
