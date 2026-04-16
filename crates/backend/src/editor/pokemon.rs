use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::{Pokemon, Type, PokemonAbility, Evolution, PokemonForm, Learnset};
use crate::auth::EditorSession;
use crate::filters;
use super::{EditForm, PokemonFull};

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
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn pokemon_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon ORDER BY dex_number ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let mut mapped_pokemon = Vec::new();
    for p in pokemon {
        let t1 = match all_types.iter().find(|t| t.id == p.type1_id) {
            Some(t) => t.clone(),
            None => continue,
        };
        let t2 = p.type2_id.and_then(|id| all_types.iter().find(|t| t.id == id).cloned());
        mapped_pokemon.push((p, t1, t2));
    }

    Html(
        PokemonListTemplate {
            pokemon: mapped_pokemon,
            user_role: Some("editor".into()),
        }
        .render()
        .unwrap_or_default(),
    )
}

pub async fn pokemon_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
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

    let toml_content = toml::to_string(&full).unwrap_or_default();
    Html(
        PokemonEditTemplate {
            slug: "new".to_string(),
            toml_content,
            error: None,
            user_role: Some("editor".into()),
        }
        .render()
        .unwrap_or_default(),
    )
}

pub async fn pokemon_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let full: PokemonFull = match toml::from_str(&payload.toml_content) {
        Ok(p) => p,
        Err(e) => {
            return Html(
                PokemonEditTemplate {
                    slug: "new".to_string(),
                    toml_content: payload.toml_content,
                    error: Some(format!("Erro no TOML: {}", e)),
                    user_role: Some("editor".into()),
                }
                .render()
                .unwrap_or_default(),
            )
            .into_response();
        }
    };
    let new_p = full.pokemon;

    let tx = pool.begin().await;
    let mut tx = match tx {
        Ok(tx) => tx,
        Err(e) => {
            return Html(
                PokemonEditTemplate {
                    slug: "new".to_string(),
                    toml_content: payload.toml_content,
                    error: Some(format!("Erro ao iniciar transação: {}", e)),
                    user_role: Some("editor".into()),
                }
                .render()
                .unwrap_or_default(),
            )
            .into_response();
        }
    };

    let res = async {
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
        Ok::<_, sqlx::Error>(())
    }
    .await;

    let result = match res {
        Ok(_) => tx.commit().await,
        Err(e) => {
            let _ = tx.rollback().await;
            Err(e)
        }
    };

    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("pokemon").bind(new_p.id).bind(&format!("CREATE---{}", payload.toml_content))
                .execute(&pool).await;
            Redirect::to(&format!("/wiki/pokemon/{}", filters::to_slug(&new_p.name))).into_response()
        }
        Err(e) => Html(
            PokemonEditTemplate {
                slug: "new".to_string(),
                toml_content: payload.toml_content,
                error: Some(format!("Erro no banco de dados: {}", e)),
                user_role: Some("editor".into()),
            }
            .render()
            .unwrap_or_default(),
        )
        .into_response(),
    }
}

pub async fn pokemon_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let pokemon = all_pokemon.into_iter().find(|p| filters::to_slug(&p.name) == slug);

    if let Some(p) = pokemon {
        let abilities = sqlx::query_as::<_, PokemonAbility>("SELECT * FROM pokemon_abilities WHERE pokemon_id = ?")
            .bind(p.id).fetch_all(&pool).await.unwrap_or_default();
        let evolutions = sqlx::query_as::<_, Evolution>("SELECT * FROM evolutions WHERE from_id = ?")
            .bind(p.id).fetch_all(&pool).await.unwrap_or_default();
        let forms = sqlx::query_as::<_, PokemonForm>("SELECT * FROM pokemon_forms WHERE base_id = ?")
            .bind(p.id).fetch_all(&pool).await.unwrap_or_default();
        let learnset = sqlx::query_as::<_, Learnset>("SELECT * FROM learnset WHERE pokemon_id = ?")
            .bind(p.id).fetch_all(&pool).await.unwrap_or_default();

        let full = PokemonFull { pokemon: p, abilities, evolutions, forms, learnset };
        let toml_content = toml::to_string(&full).unwrap_or_default();
        Html(
            PokemonEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
                .render().unwrap_or_default(),
        )
        .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn pokemon_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let full: PokemonFull = match toml::from_str(&payload.toml_content) {
        Ok(p) => p,
        Err(e) => {
            return Html(
                PokemonEditTemplate {
                    slug,
                    toml_content: payload.toml_content,
                    error: Some(format!("Erro no TOML: {}", e)),
                    user_role: Some("editor".into()),
                }
                .render()
                .unwrap_or_default(),
            )
            .into_response();
        }
    };

    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let old_p = match all_pokemon.into_iter().find(|p| filters::to_slug(&p.name) == slug) {
        Some(p) => p,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let mut new_p = full.pokemon.clone();
    new_p.id = old_p.id; // Evita ID Hijacking
    let full = PokemonFull { pokemon: new_p.clone(), ..full };

    let old_toml = toml::to_string(&old_p).unwrap_or_default();
    let diff = format!("{}---{}", old_toml, payload.toml_content);

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Html(
                PokemonEditTemplate {
                    slug,
                    toml_content: payload.toml_content,
                    error: Some(format!("Erro ao iniciar transação: {}", e)),
                    user_role: Some("editor".into()),
                }
                .render()
                .unwrap_or_default(),
            )
            .into_response();
        }
    };

    let res = async {
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
        Ok::<_, sqlx::Error>(())
    }
    .await;

    let result = match res {
        Ok(_) => tx.commit().await,
        Err(e) => {
            let _ = tx.rollback().await;
            Err(e)
        }
    };

    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("pokemon").bind(new_p.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/pokemon/{}", slug)).into_response()
        }
        Err(e) => Html(
            PokemonEditTemplate {
                slug,
                toml_content: payload.toml_content,
                error: Some(format!("Erro no banco de dados: {}", e)),
                user_role: Some("editor".into()),
            }
            .render()
            .unwrap_or_default(),
        )
        .into_response(),
    }
}

pub async fn pokemon_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let p = all_pokemon.into_iter().find(|p| filters::to_slug(&p.name) == slug);

    if let Some(p) = p {
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
        };

        let res = async {
            sqlx::query("DELETE FROM pokemon_abilities WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM evolutions WHERE from_id = ? OR to_id = ?").bind(p.id).bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM pokemon_forms WHERE base_id = ? OR form_id = ?").bind(p.id).bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM learnset WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM team_members WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM pokemon WHERE id = ?").bind(p.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }
        .await;

        match res {
            Ok(_) => match tx.commit().await {
                Ok(_) => Redirect::to("/editor/pokemon").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao commitar: {}", e)).into_response(),
            },
            Err(e) => {
                let _ = tx.rollback().await;
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response()
            }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
