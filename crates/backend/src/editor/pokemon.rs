use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
};
use sqlx::SqlitePool;
use engine::models::{Pokemon, Type, PokemonAbility, Evolution, PokemonForm, Learnset, Ability, Move};
use crate::auth::EditorSession;
use crate::filters;
use super::{JsonEditForm, PokemonFull};

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "editor/pokemon/list.html")]
pub struct PokemonListTemplate {
    pub pokemon: Vec<(Pokemon, Type, Option<Type>)>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "editor/pokemon/edit.html")]
pub struct PokemonEditTemplate {
    pub slug: String,
    pub json_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
    pub all_types: Vec<Type>,
    pub all_abilities: Vec<Ability>,
    pub all_moves: Vec<Move>,
    pub all_pokemon: Vec<Pokemon>,
}

// ── Helper ───────────────────────────────────────────────────────────────────

async fn fetch_metadata(pool: &SqlitePool) -> Result<(Vec<Type>, Vec<Ability>, Vec<Move>, Vec<Pokemon>), crate::error::AppError> {
    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types").fetch_all(pool).await?;
    let all_abilities = sqlx::query_as::<_, Ability>("SELECT * FROM abilities").fetch_all(pool).await?;
    let all_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves").fetch_all(pool).await?;
    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon ORDER BY dex_number ASC").fetch_all(pool).await?;
    Ok((all_types, all_abilities, all_moves, all_pokemon))
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn pokemon_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon ORDER BY dex_number ASC")
        .fetch_all(&pool)
        .await?;

    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool)
        .await?;

    let mut mapped_pokemon = Vec::new();
    for p in pokemon {
        let t1 = match all_types.iter().find(|t| t.id == p.type1_id) {
            Some(t) => t.clone(),
            None => continue,
        };
        let t2 = p.type2_id.and_then(|id| all_types.iter().find(|t| t.id == id).cloned());
        mapped_pokemon.push((p, t1, t2));
    }

    Ok(Html(
        PokemonListTemplate {
            pokemon: mapped_pokemon,
            user_role: Some("editor".into()),
        }
        .render()?,
    ).into_response())
}

pub async fn pokemon_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM pokemon")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let next_id = max_id + 1;
    let max_dex: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(dex_number), 0) FROM pokemon")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let next_dex = max_dex + 1;

    let default_p = Pokemon {
        id: next_id,
        dex_number: next_dex,
        name: "Novo Pokemon".to_string(),
        sprite: "sprite.png".to_string(),
        sprite_shiny: "sprite_shiny.png".to_string(),
        type1_id: 1,
        type2_id: None,
        hp: 10, atk: 10, def: 10, spa: 10, spd: 10, spe: 10,
        pokedex_entry: "Nova descrição".to_string(),
    };

    let full = PokemonFull {
        pokemon: default_p,
        abilities: vec![PokemonAbility { pokemon_id: next_id, ability_id: 0, slot: 1 }],
        evolutions: vec![],
        forms: vec![],
        learnset: vec![],
    };

    let json_content = serde_json::to_string(&full).unwrap_or_default();
    let (all_types, all_abilities, all_moves, all_pokemon) = fetch_metadata(&pool).await?;

    Ok(Html(
        PokemonEditTemplate {
            slug: "new".to_string(),
            json_content,
            error: None,
            user_role: Some("editor".into()),
            all_types,
            all_abilities,
            all_moves,
            all_pokemon,
        }
        .render()?,
    ).into_response())
}

pub async fn pokemon_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<JsonEditForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let full: PokemonFull = match serde_json::from_str(&payload.json_content) {
        Ok(p) => p,
        Err(e) => {
            let (all_types, all_abilities, all_moves, all_pokemon) = fetch_metadata(&pool).await?;
            return Ok(Html(
                PokemonEditTemplate {
                    slug: "new".to_string(),
                    json_content: payload.json_content,
                    error: Some(format!("Erro no JSON: {}", e)),
                    user_role: Some("editor".into()),
                    all_types,
                    all_abilities,
                    all_moves,
                    all_pokemon,
                }
                .render()?,
            )
            .into_response());
        }
    };
    let new_p = full.pokemon;

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO pokemon (id, dex_number, name, sprite, sprite_shiny,
            type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(new_p.id).bind(new_p.dex_number).bind(&new_p.name).bind(&new_p.sprite).bind(&new_p.sprite_shiny)
    .bind(new_p.type1_id).bind(new_p.type2_id).bind(new_p.hp).bind(new_p.atk).bind(new_p.def)
    .bind(new_p.spa).bind(new_p.spd).bind(new_p.spe).bind(&new_p.pokedex_entry)
    .execute(&mut *tx).await?;

    for a in &full.abilities {
        sqlx::query("INSERT INTO pokemon_abilities (pokemon_id, ability_id, slot) VALUES (?, ?, ?)")
            .bind(new_p.id).bind(a.ability_id).bind(a.slot).execute(&mut *tx).await?;
    }
    for e in &full.evolutions {
        sqlx::query("INSERT INTO evolutions (from_id, to_id, condition, condition_value) VALUES (?, ?, ?, ?)")
            .bind(new_p.id).bind(e.to_id).bind(&e.condition).bind(&e.condition_value).execute(&mut *tx).await?;
    }
    for f in &full.forms {
        sqlx::query("INSERT INTO pokemon_forms (base_id, form_id, form_name) VALUES (?, ?, ?)")
            .bind(new_p.id).bind(f.form_id).bind(&f.form_name).execute(&mut *tx).await?;
    }
    for l in &full.learnset {
        sqlx::query("INSERT INTO learnset (pokemon_id, move_id, method, level) VALUES (?, ?, ?, ?)")
            .bind(new_p.id).bind(l.move_id).bind(&l.method).bind(l.level).execute(&mut *tx).await?;
    }

    tx.commit().await?;

    sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
        .bind(&trainer.id).bind("pokemon").bind(new_p.id).bind(&format!("CREATE---{}", payload.json_content))
        .execute(&pool).await?;
        
    Ok(Redirect::to(&format!("/wiki/pokemon/{}", filters::to_slug(&new_p.name))).into_response())
}

pub async fn pokemon_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon")
        .fetch_all(&pool)
        .await?;

    let pokemon = all_pokemon.into_iter().find(|p| filters::to_slug(&p.name) == slug);

    if let Some(p) = pokemon {
        let abilities = sqlx::query_as::<_, PokemonAbility>("SELECT * FROM pokemon_abilities WHERE pokemon_id = ?")
            .bind(p.id).fetch_all(&pool).await?;
        let evolutions = sqlx::query_as::<_, Evolution>("SELECT * FROM evolutions WHERE from_id = ?")
            .bind(p.id).fetch_all(&pool).await?;
        let forms = sqlx::query_as::<_, PokemonForm>("SELECT * FROM pokemon_forms WHERE base_id = ?")
            .bind(p.id).fetch_all(&pool).await?;
        let learnset = sqlx::query_as::<_, Learnset>("SELECT * FROM learnset WHERE pokemon_id = ?")
            .bind(p.id).fetch_all(&pool).await?;

        let full = PokemonFull { pokemon: p, abilities, evolutions, forms, learnset };
        let json_content = serde_json::to_string(&full).unwrap_or_default();
        let (all_types, all_abilities, all_moves, all_pokemon) = fetch_metadata(&pool).await?;
        Ok(Html(
            PokemonEditTemplate { slug, json_content, error: None, user_role: Some("editor".into()),
            all_types, all_abilities, all_moves, all_pokemon }
                .render()?,
        )
        .into_response())
    } else {
        Err(crate::error::AppError::NotFound)
    }
}

pub async fn pokemon_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<JsonEditForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let full: PokemonFull = match serde_json::from_str(&payload.json_content) {
        Ok(p) => p,
        Err(e) => {
            let (all_types, all_abilities, all_moves, all_pokemon) = fetch_metadata(&pool).await?;
            return Ok(Html(
                PokemonEditTemplate {
                    slug,
                    json_content: payload.json_content,
                    error: Some(format!("Erro no JSON: {}", e)),
                    user_role: Some("editor".into()),
                    all_types,
                    all_abilities,
                    all_moves,
                    all_pokemon,
                }
                .render()?,
            )
            .into_response());
        }
    };

    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon")
        .fetch_all(&pool)
        .await?;

    let old_p = match all_pokemon.into_iter().find(|p| filters::to_slug(&p.name) == slug) {
        Some(p) => p,
        None => return Err(crate::error::AppError::NotFound),
    };

    let mut new_p = full.pokemon.clone();
    new_p.id = old_p.id; // Evita ID Hijacking
    let full = PokemonFull { pokemon: new_p.clone(), ..full };

    let old_json = serde_json::to_string(&old_p).unwrap_or_default();
    let diff = format!("{}---{}", old_json, payload.json_content);

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE pokemon
        SET dex_number = ?, name = ?, sprite = ?, sprite_shiny = ?,
            type1_id = ?, type2_id = ?, hp = ?, atk = ?, def = ?,
            spa = ?, spd = ?, spe = ?, pokedex_entry = ?
        WHERE id = ?
        "#,
    )
    .bind(new_p.dex_number).bind(&new_p.name).bind(&new_p.sprite).bind(&new_p.sprite_shiny)
    .bind(new_p.type1_id).bind(new_p.type2_id).bind(new_p.hp).bind(new_p.atk).bind(new_p.def)
    .bind(new_p.spa).bind(new_p.spd).bind(new_p.spe).bind(&new_p.pokedex_entry)
    .bind(new_p.id)
    .execute(&mut *tx).await?;

    sqlx::query("DELETE FROM pokemon_abilities WHERE pokemon_id = ?").bind(new_p.id).execute(&mut *tx).await?;
    for a in &full.abilities {
        sqlx::query("INSERT INTO pokemon_abilities (pokemon_id, ability_id, slot) VALUES (?, ?, ?)")
            .bind(new_p.id).bind(a.ability_id).bind(a.slot).execute(&mut *tx).await?;
    }

    sqlx::query("DELETE FROM evolutions WHERE from_id = ?").bind(new_p.id).execute(&mut *tx).await?;
    for e in &full.evolutions {
        sqlx::query("INSERT INTO evolutions (from_id, to_id, condition, condition_value) VALUES (?, ?, ?, ?)")
            .bind(new_p.id).bind(e.to_id).bind(&e.condition).bind(&e.condition_value).execute(&mut *tx).await?;
    }

    sqlx::query("DELETE FROM pokemon_forms WHERE base_id = ?").bind(new_p.id).execute(&mut *tx).await?;
    for f in &full.forms {
        sqlx::query("INSERT INTO pokemon_forms (base_id, form_id, form_name) VALUES (?, ?, ?)")
            .bind(new_p.id).bind(f.form_id).bind(&f.form_name).execute(&mut *tx).await?;
    }

    sqlx::query("DELETE FROM learnset WHERE pokemon_id = ?").bind(new_p.id).execute(&mut *tx).await?;
    for l in &full.learnset {
        sqlx::query("INSERT INTO learnset (pokemon_id, move_id, method, level) VALUES (?, ?, ?, ?)")
            .bind(new_p.id).bind(l.move_id).bind(&l.method).bind(l.level).execute(&mut *tx).await?;
    }

    tx.commit().await?;

    sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
        .bind(&trainer.id).bind("pokemon").bind(new_p.id).bind(&diff).execute(&pool).await?;
        
    Ok(Redirect::to(&format!("/wiki/pokemon/{}", slug)).into_response())
}

pub async fn pokemon_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon")
        .fetch_all(&pool)
        .await?;

    let p = all_pokemon.into_iter().find(|p| filters::to_slug(&p.name) == slug);

    if let Some(p) = p {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM pokemon_abilities WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM evolutions WHERE from_id = ? OR to_id = ?").bind(p.id).bind(p.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pokemon_forms WHERE base_id = ? OR form_id = ?").bind(p.id).bind(p.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM learnset WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM team_members WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pokemon WHERE id = ?").bind(p.id).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(Redirect::to("/editor/pokemon").into_response())
    } else {
        Err(crate::error::AppError::NotFound)
    }
}
