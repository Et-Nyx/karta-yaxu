// Submódulos de entidade
mod pokemon;
mod moves;
mod types;
mod abilities;
mod natures;
mod status;
mod upload;

// Re-export de todos os handlers para uso no main.rs
pub use pokemon::{pokemon_list, pokemon_new, pokemon_create, pokemon_edit, pokemon_save, pokemon_delete};
pub use moves::{move_list, move_new, move_create, move_edit, move_save, move_delete};
pub use types::{type_list, type_new, type_create, type_edit, type_save, type_delete};
pub use abilities::{ability_list, ability_new, ability_create, ability_edit, ability_save, ability_delete};
pub use natures::{nature_list, nature_new, nature_create, nature_edit, nature_save, nature_delete};
pub use status::{status_list, status_new, status_create, status_edit, status_save, status_delete};

use axum::{routing::{get, post}, Router};
use sqlx::SqlitePool;

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(index))
        .route("/pokemon", get(pokemon_list))
        .route("/pokemon/new", get(pokemon_new).post(pokemon_create))
        .route("/pokemon/{slug}", get(pokemon_edit).post(pokemon_save))
        .route("/pokemon/{slug}/delete", post(pokemon_delete))
        .route("/moves", get(move_list))
        .route("/moves/new", get(move_new).post(move_create))
        .route("/moves/{slug}", get(move_edit).post(move_save))
        .route("/moves/{slug}/delete", post(move_delete))
        .route("/types", get(type_list))
        .route("/types/new", get(type_new).post(type_create))
        .route("/types/{slug}", get(type_edit).post(type_save))
        .route("/types/{slug}/delete", post(type_delete))
        .route("/abilities", get(ability_list))
        .route("/abilities/new", get(ability_new).post(ability_create))
        .route("/abilities/{slug}", get(ability_edit).post(ability_save))
        .route("/abilities/{slug}/delete", post(ability_delete))
        .route("/natures", get(nature_list))
        .route("/natures/new", get(nature_new).post(nature_create))
        .route("/natures/{slug}", get(nature_edit).post(nature_save))
        .route("/natures/{slug}/delete", post(nature_delete))
        .route("/status", get(status_list))
        .route("/status/new", get(status_new).post(status_create))
        .route("/status/{slug}", get(status_edit).post(status_save))
        .route("/status/{slug}/delete", post(status_delete))
        .route("/upload/{folder}", post(upload::upload_image))
}
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

/// Form genérico para JSON (usado no novo layout de pokemon).
#[derive(Deserialize)]
pub struct JsonEditForm {
    pub json_content: String,
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
