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

pub async fn run_seed_cli(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌱 Iniciando processo completo de Seed (Em Transação)...");

    let base = std::env::var("SEED_DATA_PATH")
        .unwrap_or_else(|_| {
            if std::path::Path::new("/app/template").exists() {
                "/app/template".to_string()
            } else {
                "data/template".to_string()
            }
        });

    let mut tx = pool.begin().await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON").execute(&mut *tx).await?;

    let c = fs::read_to_string(format!("{base}/type.toml"))
        .map_err(|e| format!("Erro ao ler type.toml: {e}"))?;
    for t in toml::from_str::<TypeSeed>(&c)
        .map_err(|e| format!("Erro ao parsear type.toml: {e}"))?.types {
        sqlx::query!("INSERT OR REPLACE INTO types (id, name, hexcolor, icon) VALUES (?, ?, ?, ?)", t.id, t.name, t.hexcolor, t.icon).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir type '{}': {e}", t.name))?;
    }

    let c = fs::read_to_string(format!("{base}/ability.toml"))
        .map_err(|e| format!("Erro ao ler ability.toml: {e}"))?;
    for a in toml::from_str::<AbilitySeed>(&c)
        .map_err(|e| format!("Erro ao parsear ability.toml: {e}"))?.ability {
        sqlx::query!("INSERT OR REPLACE INTO abilities (id, name, description) VALUES (?, ?, ?)", a.id, a.name, a.description).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir ability '{}': {e}", a.name))?;
    }

    let c = fs::read_to_string(format!("{base}/item.toml"))
        .map_err(|e| format!("Erro ao ler item.toml: {e}"))?;
    for i in toml::from_str::<ItemSeed>(&c)
        .map_err(|e| format!("Erro ao parsear item.toml: {e}"))?.item {
        sqlx::query!("INSERT OR REPLACE INTO items (id, name, description, icon) VALUES (?, ?, ?, ?)", i.id, i.name, i.description, i.icon).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir item '{}': {e}", i.name))?;
    }

    let c = fs::read_to_string(format!("{base}/status_condition.toml"))
        .map_err(|e| format!("Erro ao ler status_condition.toml: {e}"))?;
    for s in toml::from_str::<StatusSeed>(&c)
        .map_err(|e| format!("Erro ao parsear status_condition.toml: {e}"))?.status_condition {
        let iv = if s.is_volatile { 1 } else { 0 };
        sqlx::query!("INSERT OR REPLACE INTO status_conditions (id, name, description, icon, is_volatile) VALUES (?, ?, ?, ?, ?)", s.id, s.name, s.description, s.icon, iv).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir status_condition '{}': {e}", s.name))?;
    }

    let c = fs::read_to_string(format!("{base}/nature.toml"))
        .map_err(|e| format!("Erro ao ler nature.toml: {e}"))?;
    for n in toml::from_str::<NatureSeed>(&c)
        .map_err(|e| format!("Erro ao parsear nature.toml: {e}"))?.nature {
        sqlx::query!("INSERT OR REPLACE INTO natures (id, name, increased_stat, decreased_stat) VALUES (?, ?, ?, ?)", n.id, n.name, n.increased_stat, n.decreased_stat).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir nature '{}': {e}", n.name))?;
    }

    let c = fs::read_to_string(format!("{base}/move.toml"))
        .map_err(|e| format!("Erro ao ler move.toml: {e}"))?;
    for m in toml::from_str::<MoveSeed>(&c)
        .map_err(|e| format!("Erro ao parsear move.toml: {e}"))?.moves {
        let cat = match m.category { MoveCategory::Physical=>"physical", MoveCategory::Special=>"special", MoveCategory::Status=>"status" };
        sqlx::query!("INSERT OR REPLACE INTO moves (id, name, type_id, category, power, accuracy, pp, description) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", m.id, m.name, m.type_id, cat, m.power, m.accuracy, m.pp, m.description).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir move '{}': {e}", m.name))?;
    }

    let c = fs::read_to_string(format!("{base}/pokemon.toml"))
        .map_err(|e| format!("Erro ao ler pokemon.toml: {e}"))?;
    for p in toml::from_str::<PokemonSeed>(&c)
        .map_err(|e| format!("Erro ao parsear pokemon.toml: {e}"))?.pokemon {
        sqlx::query!("INSERT OR REPLACE INTO pokemon (id, dex_number, name, sprite, sprite_shiny, type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", p.id, p.dex_number, p.name, p.sprite, p.sprite_shiny, p.type1_id, p.type2_id, p.hp, p.atk, p.def, p.spa, p.spd, p.spe, p.pokedex_entry).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir pokemon '{}': {e}", p.name))?;
    }

    let c = fs::read_to_string(format!("{base}/type_effectiveness.toml"))
        .map_err(|e| format!("Erro ao ler type_effectiveness.toml: {e}"))?;
    for t in toml::from_str::<EffectivenessSeed>(&c)
        .map_err(|e| format!("Erro ao parsear type_effectiveness.toml: {e}"))?.type_effectiveness {
        sqlx::query!("INSERT OR REPLACE INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)", t.attacker_id, t.defender_id, t.multiplier).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir type_effectiveness ({}->{}) : {e}", t.attacker_id, t.defender_id))?;
    }

    let c = fs::read_to_string(format!("{base}/pokemon_ability.toml"))
        .map_err(|e| format!("Erro ao ler pokemon_ability.toml: {e}"))?;
    for p in toml::from_str::<PokeAbilitySeed>(&c)
        .map_err(|e| format!("Erro ao parsear pokemon_ability.toml: {e}"))?.pokemon_ability {
        sqlx::query!("INSERT OR REPLACE INTO pokemon_abilities (pokemon_id, ability_id, slot) VALUES (?, ?, ?)", p.pokemon_id, p.ability_id, p.slot).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir pokemon_ability (pokemon {}): {e}", p.pokemon_id))?;
    }

    let c = fs::read_to_string(format!("{base}/evolution.toml"))
        .map_err(|e| format!("Erro ao ler evolution.toml: {e}"))?;
    for e in toml::from_str::<EvolutionSeed>(&c)
        .map_err(|e| format!("Erro ao parsear evolution.toml: {e}"))?.evolution {
        sqlx::query!("INSERT OR REPLACE INTO evolutions (from_id, to_id, condition, condition_value) VALUES (?, ?, ?, ?)", e.from_id, e.to_id, e.condition, e.condition_value).execute(&mut *tx).await
            .map_err(|err| format!("Erro ao inserir evolution ({}->{}) : {err}", e.from_id, e.to_id))?;
    }

    let c = fs::read_to_string(format!("{base}/pokemon_form.toml"))
        .map_err(|e| format!("Erro ao ler pokemon_form.toml: {e}"))?;
    for f in toml::from_str::<FormSeed>(&c)
        .map_err(|e| format!("Erro ao parsear pokemon_form.toml: {e}"))?.pokemon_form {
        sqlx::query!("INSERT OR REPLACE INTO pokemon_forms (base_id, form_id, form_name) VALUES (?, ?, ?)", f.base_id, f.form_id, f.form_name).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir pokemon_form ({}->{}) : {e}", f.base_id, f.form_id))?;
    }

    let c = fs::read_to_string(format!("{base}/learnset.toml"))
        .map_err(|e| format!("Erro ao ler learnset.toml: {e}"))?;
    for l in toml::from_str::<LearnsetSeed>(&c)
        .map_err(|e| format!("Erro ao parsear learnset.toml: {e}"))?.learnset {
        let met = match l.method { LearnMethod::LevelUp=>"level_up", LearnMethod::Tm=>"tm", LearnMethod::Egg=>"egg", LearnMethod::Tutor=>"tutor", LearnMethod::Gmax=>"gmax", LearnMethod::Max=>"max" };
        sqlx::query!("INSERT OR REPLACE INTO learnset (pokemon_id, move_id, method, level) VALUES (?, ?, ?, ?)", l.pokemon_id, l.move_id, met, l.level).execute(&mut *tx).await
            .map_err(|e| format!("Erro ao inserir learnset (pokemon {}, move {}): {e}", l.pokemon_id, l.move_id))?;
    }

    tx.commit().await?;

    println!("MEGA SEED Completo! O banco está vivo e 100% populado de modo seguro transacional!");
    Ok(())
}
