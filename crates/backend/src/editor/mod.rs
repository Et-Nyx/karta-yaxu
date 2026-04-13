// Submódulos de entidade
mod pokemon;
mod moves;
mod types;
mod abilities;
mod natures;
mod status;

// Re-export de todos os handlers para uso no main.rs
pub use pokemon::{pokemon_list, pokemon_new, pokemon_create, pokemon_edit, pokemon_save, pokemon_delete};
pub use moves::{move_list, move_new, move_create, move_edit, move_save, move_delete};
pub use types::{type_list, type_new, type_create, type_edit, type_save, type_delete};
pub use abilities::{ability_list, ability_new, ability_create, ability_edit, ability_save, ability_delete};
pub use natures::{nature_list, nature_new, nature_create, nature_edit, nature_save, nature_delete};
pub use status::{status_list, status_new, status_create, status_edit, status_save, status_delete};

// Re-export do index
pub use index::index;
mod index;

// ── Tipos compartilhados ─────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use engine::models::{
    Pokemon, PokemonAbility, Evolution, PokemonForm, Learnset,
    Type, TypeEffectiveness,
};

/// Form genérico do editor — todo endpoint POST recebe um campo `toml_content`.
#[derive(Deserialize)]
pub struct EditForm {
    pub toml_content: String,
}

/// Pokémon com todas as relações (abilities, evos, forms, learnset).
#[derive(Serialize, Deserialize)]
pub struct PokemonFull {
    #[serde(flatten)]
    pub pokemon: Pokemon,
    pub abilities: Vec<PokemonAbility>,
    pub evolutions: Vec<Evolution>,
    pub forms: Vec<PokemonForm>,
    pub learnset: Vec<Learnset>,
}

/// Type com a matriz de efetividade como atacante.
#[derive(Serialize, Deserialize)]
pub struct TypeFull {
    #[serde(flatten)]
    pub types: Type,
    pub effectiveness: Vec<TypeEffectiveness>,
}
