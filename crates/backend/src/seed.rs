use axum::{extract::State, response::IntoResponse};
use sqlx::SqlitePool;
use std::fs;
use engine::models::*;
use serde::Deserialize;

#[derive(Deserialize)] struct TypeSeed { #[serde(rename = "type")] types: Vec<Type> }
#[derive(Deserialize)] struct AbilitySeed { ability: Vec<Ability> }
#[derive(Deserialize)] struct MoveSeed { #[serde(rename = "move")] moves: Vec<Move> }
#[derive(Deserialize)] struct PokemonSeed { pokemon: Vec<Pokemon> }
#[derive(Deserialize)] struct ItemSeed { item: Vec<Item> }
#[derive(Deserialize)] struct NatureSeed { nature: Vec<Nature> }
#[derive(Deserialize)] struct StatusSeed { status_condition: Vec<StatusCondition> }
#[derive(Deserialize)] struct EffectivenessSeed { type_effectiveness: Vec<TypeEffectiveness> }
#[derive(Deserialize)] struct PokeAbilitySeed { pokemon_ability: Vec<PokemonAbility> }
#[derive(Deserialize)] struct EvolutionSeed { evolution: Vec<Evolution> }
#[derive(Deserialize)] struct FormSeed { pokemon_form: Vec<PokemonForm> }
#[derive(Deserialize)] struct LearnsetSeed { learnset: Vec<Learnset> }

pub async fn run_seed(State(pool): State<SqlitePool>) -> impl IntoResponse {
    println!("🌱 Iniciando processo completo de Seed...");

    if let Ok(c) = fs::read_to_string("data/template/type.toml") {
        for t in toml::from_str::<TypeSeed>(&c).unwrap().types {
            sqlx::query!("INSERT OR REPLACE INTO types (id, name, hexcolor, icon) VALUES (?, ?, ?, ?)", t.id, t.name, t.hexcolor, t.icon).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/ability.toml") {
        for a in toml::from_str::<AbilitySeed>(&c).unwrap().ability {
            sqlx::query!("INSERT OR REPLACE INTO abilities (id, name, description) VALUES (?, ?, ?)", a.id, a.name, a.description).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/item.toml") {
        for i in toml::from_str::<ItemSeed>(&c).unwrap().item {
            sqlx::query!("INSERT OR REPLACE INTO items (id, name, description, icon) VALUES (?, ?, ?, ?)", i.id, i.name, i.description, i.icon).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/status_condition.toml") {
        for s in toml::from_str::<StatusSeed>(&c).unwrap().status_condition {
            let iv = if s.is_volatile { 1 } else { 0 };
            sqlx::query!("INSERT OR REPLACE INTO status_conditions (id, name, description, icon, is_volatile) VALUES (?, ?, ?, ?, ?)", s.id, s.name, s.description, s.icon, iv).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/nature.toml") {
        for n in toml::from_str::<NatureSeed>(&c).unwrap().nature {
            sqlx::query!("INSERT OR REPLACE INTO natures (id, name, increased_stat, decreased_stat) VALUES (?, ?, ?, ?)", n.id, n.name, n.increased_stat, n.decreased_stat).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/move.toml") {
        for m in toml::from_str::<MoveSeed>(&c).unwrap().moves {
            let cat = match m.category { MoveCategory::Physical=>"physical", MoveCategory::Special=>"special", MoveCategory::Status=>"status" };
            sqlx::query!("INSERT OR REPLACE INTO moves (id, name, type_id, category, power, accuracy, pp, description) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", m.id, m.name, m.type_id, cat, m.power, m.accuracy, m.pp, m.description).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/pokemon.toml") {
        for p in toml::from_str::<PokemonSeed>(&c).unwrap().pokemon {
            sqlx::query!("INSERT OR REPLACE INTO pokemon (id, dex_number, name, sprite, sprite_shiny, type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", p.id, p.dex_number, p.name, p.sprite, p.sprite_shiny, p.type1_id, p.type2_id, p.hp, p.atk, p.def, p.spa, p.spd, p.spe, p.pokedex_entry).execute(&pool).await.unwrap();
        }
    }

    if let Ok(c) = fs::read_to_string("data/template/type_effectiveness.toml") {
        for t in toml::from_str::<EffectivenessSeed>(&c).unwrap().type_effectiveness {
            sqlx::query!("INSERT OR REPLACE INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)", t.attacker_id, t.defender_id, t.multiplier).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/pokemon_ability.toml") {
        for p in toml::from_str::<PokeAbilitySeed>(&c).unwrap().pokemon_ability {
            sqlx::query!("INSERT OR REPLACE INTO pokemon_abilities (pokemon_id, ability_id, slot) VALUES (?, ?, ?)", p.pokemon_id, p.ability_id, p.slot).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/evolution.toml") {
        for e in toml::from_str::<EvolutionSeed>(&c).unwrap().evolution {
            sqlx::query!("INSERT OR REPLACE INTO evolutions (from_id, to_id, condition, condition_value) VALUES (?, ?, ?, ?)", e.from_id, e.to_id, e.condition, e.condition_value).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/pokemon_form.toml") {
        for f in toml::from_str::<FormSeed>(&c).unwrap().pokemon_form {
            sqlx::query!("INSERT OR REPLACE INTO pokemon_forms (base_id, form_id, form_name) VALUES (?, ?, ?)", f.base_id, f.form_id, f.form_name).execute(&pool).await.unwrap();
        }
    }
    if let Ok(c) = fs::read_to_string("data/template/learnset.toml") {
        for l in toml::from_str::<LearnsetSeed>(&c).unwrap().learnset {
            let met = match l.method { LearnMethod::LevelUp=>"level_up", LearnMethod::Tm=>"tm", LearnMethod::Egg=>"egg", LearnMethod::Tutor=>"tutor" };
            sqlx::query!("INSERT OR REPLACE INTO learnset (pokemon_id, move_id, method, level) VALUES (?, ?, ?, ?)", l.pokemon_id, l.move_id, met, l.level).execute(&pool).await.unwrap();
        }
    }

    "MEGA SEED Completo! O banco está vivo e 100% populado com todas as tabelas!"
}
