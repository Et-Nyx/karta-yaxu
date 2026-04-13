use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::{
    Pokemon, Move, Type, Ability, Nature, StatusCondition,
    PokemonAbility, Evolution, PokemonForm, Learnset, TypeEffectiveness
};
use crate::auth::EditorSession;
use crate::filters;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PokemonFull {
    #[serde(flatten)]
    pub pokemon: Pokemon,
    pub abilities: Vec<PokemonAbility>,
    pub evolutions: Vec<Evolution>,
    pub forms: Vec<PokemonForm>,
    pub learnset: Vec<Learnset>,
}

#[derive(Serialize, Deserialize)]
pub struct TypeFull {
    #[serde(flatten)]
    pub types: Type, // Nomeado 'types' para compatibilidade com outros campos pluralizados ou se quiser um campo único.
    pub effectiveness: Vec<TypeEffectiveness>,
}

#[derive(Template)]
#[template(path = "editor/pokemon/list.html")]
pub struct PokemonListTemplate {
    pub pokemon: std::vec::Vec<(engine::models::Pokemon, engine::models::Type, std::option::Option<engine::models::Type>)>,
    pub user_role: Option<String>,
}

pub async fn pokemon_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let pokemon = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon ORDER BY dex_number ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types").fetch_all(&pool).await.unwrap_or_default();
    
    let mut mapped_pokemon = Vec::new();
    for p in pokemon {
        let t1 = all_types.iter().find(|t| t.id == p.type1_id).unwrap().clone();
        let t2 = p.type2_id.and_then(|id| all_types.iter().find(|t| t.id == id).cloned());
        mapped_pokemon.push((p, t1, t2));
    }

    Html(PokemonListTemplate { pokemon: mapped_pokemon, user_role: Some("editor".into()) }.render().unwrap())
}

#[derive(Template)]
#[template(path = "editor/pokemon/edit.html")]
pub struct PokemonEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

pub async fn pokemon_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM pokemon").fetch_one(&pool).await.unwrap_or(0);
    let next_id = max_id + 1;
    let max_dex: i64 = sqlx::query_scalar("SELECT MAX(dex_number) FROM pokemon").fetch_one(&pool).await.unwrap_or(0);
    let next_dex = max_dex + 1;

    let default_p = Pokemon {
        id: next_id,
        dex_number: next_dex,
        name: "Novo Pokemon".to_string(),
        sprite: "sprite.png".to_string(),
        sprite_shiny: "sprite_shiny.png".to_string(),
        type1_id: 1, // Normal
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

    let toml_content = toml::to_string(&full).unwrap();
    Html(PokemonEditTemplate {
        slug: "new".to_string(),
        toml_content,
        error: None,
        user_role: Some("editor".into()),
    }.render().unwrap())
}

pub async fn pokemon_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let full: PokemonFull = match toml::from_str(&payload.toml_content) {
        Ok(p) => p,
        Err(e) => {
            return Html(PokemonEditTemplate {
                slug: "new".to_string(),
                toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)),
                user_role: Some("editor".into()),
            }.render().unwrap()).into_response();
        }
    };
    let new_p = full.pokemon;

    let mut tx = pool.begin().await.unwrap();

    let res = async {
        sqlx::query(
            r#"
            INSERT INTO pokemon (id, dex_number, name, sprite, sprite_shiny,
                type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
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
    }.await;

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
                .bind(&trainer.id).bind("pokemon").bind(new_p.id).bind(&format!("CREATE---{}", payload.toml_content)).execute(&pool).await;
            Redirect::to(&format!("/wiki/pokemon/{}", filters::slugify(&new_p.name).unwrap())).into_response()
        }
        Err(e) => {
            Html(PokemonEditTemplate {
                slug: "new".to_string(),
                toml_content: payload.toml_content,
                error: Some(format!("Erro no banco de dados: {}", e)),
                user_role: Some("editor".into()),
            }.render().unwrap()).into_response()
        }
    }
}
pub async fn pokemon_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let pokemon = sqlx::query_as::<_, Pokemon>(
        "SELECT * FROM pokemon WHERE name LIKE ?"
    )
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if let Some(p) = pokemon {
        let abilities = sqlx::query_as::<_, PokemonAbility>("SELECT * FROM pokemon_abilities WHERE pokemon_id = ?").bind(p.id).fetch_all(&pool).await.unwrap_or_default();
        let evolutions = sqlx::query_as::<_, Evolution>("SELECT * FROM evolutions WHERE from_id = ?").bind(p.id).fetch_all(&pool).await.unwrap_or_default();
        let forms = sqlx::query_as::<_, PokemonForm>("SELECT * FROM pokemon_forms WHERE base_id = ?").bind(p.id).fetch_all(&pool).await.unwrap_or_default();
        let learnset = sqlx::query_as::<_, Learnset>("SELECT * FROM learnset WHERE pokemon_id = ?").bind(p.id).fetch_all(&pool).await.unwrap_or_default();

        let full = PokemonFull {
            pokemon: p,
            abilities,
            evolutions,
            forms,
            learnset,
        };

        let toml_content = toml::to_string(&full).unwrap();
        Html(PokemonEditTemplate {
            slug,
            toml_content,
            error: None,
            user_role: Some("editor".into()),
        }.render().unwrap()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[derive(Deserialize)]
pub struct EditForm {
    pub toml_content: String,
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
            return Html(PokemonEditTemplate {
                slug,
                toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)),
                user_role: Some("editor".into()),
            }.render().unwrap()).into_response();
        }
    };
    // Obter dados antigos para o log de edits (pode ser o Pokemon ou o Full, vamos usar o Pokemon base para simplicidade no diff por enquanto)
    let old_p = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon WHERE name LIKE ?")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .unwrap();

    let mut new_p = full.pokemon.clone();
    new_p.id = old_p.id; // Evita ID Hijacking
    let full = PokemonFull { pokemon: new_p.clone(), ..full };

    let old_toml = toml::to_string(&old_p).unwrap();
    let diff = format!("{}---{}", old_toml, payload.toml_content);

    let mut tx = pool.begin().await.unwrap();

    let res = async {
        sqlx::query(
            r#"
            UPDATE pokemon
            SET dex_number = ?, name = ?, sprite = ?, sprite_shiny = ?,
                type1_id = ?, type2_id = ?, hp = ?, atk = ?, def = ?,
                spa = ?, spd = ?, spe = ?, pokedex_entry = ?
            WHERE id = ?
            "#
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
    }.await;

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
        Err(e) => {
            Html(PokemonEditTemplate {
                slug,
                toml_content: payload.toml_content,
                error: Some(format!("Erro no banco de dados: {}", e)),
                user_role: Some("editor".into()),
            }.render().unwrap()).into_response()
        }
    }
}
pub async fn pokemon_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let p = sqlx::query_as::<_, Pokemon>("SELECT * FROM pokemon WHERE name LIKE ?")
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    if let Some(p) = p {
        let mut tx = pool.begin().await.unwrap();

        let res = async {
            sqlx::query("DELETE FROM pokemon_abilities WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM evolutions WHERE from_id = ? OR to_id = ?").bind(p.id).bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM pokemon_forms WHERE base_id = ? OR form_id = ?").bind(p.id).bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM learnset WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM team_members WHERE pokemon_id = ?").bind(p.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM pokemon WHERE id = ?").bind(p.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;

        match res {
            Ok(_) => {
                tx.commit().await.unwrap();
                Redirect::to("/editor/pokemon").into_response()
            }
            Err(e) => {
                let _ = tx.rollback().await;
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response()
            }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// Moves
#[derive(Template)]
#[template(path = "editor/moves/list.html")]
pub struct MoveListTemplate {
    pub moves: Vec<Move>,
    pub user_role: Option<String>,
}

pub async fn move_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let moves = sqlx::query_as::<_, Move>("SELECT * FROM moves ORDER BY name")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    Html(MoveListTemplate { moves, user_role: Some("editor".into()) }.render().unwrap())
}

#[derive(Template)]
#[template(path = "editor/moves/edit.html")]
pub struct MoveEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

pub async fn move_new(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM moves").fetch_one(&pool).await.unwrap_or(0);
    let default_m = Move { id: max_id + 1, name: "Novo Golpe".to_string(), type_id: 1, category: engine::models::MoveCategory::Physical, power: Some(40), accuracy: Some(100), pp: 20, description: "Descrição do golpe".to_string() };
    Html(MoveEditTemplate { slug: "new".to_string(), toml_content: toml::to_string(&default_m).unwrap(), error: None, user_role: Some("editor".into()) }.render().unwrap())
}

pub async fn move_create(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_m: Move = match toml::from_str(&payload.toml_content) {
        Ok(m) => m,
        Err(e) => return Html(MoveEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response(),
    };
    let result = sqlx::query("INSERT INTO moves (id, name, type_id, category, power, accuracy, pp, description) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(new_m.id).bind(&new_m.name).bind(new_m.type_id).bind(new_m.category).bind(new_m.power).bind(new_m.accuracy).bind(new_m.pp).bind(&new_m.description).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("moves").bind(new_m.id).bind(&format!("CREATE---{}", payload.toml_content)).execute(&pool).await;
            Redirect::to(&format!("/wiki/moves/{}", filters::slugify(&new_m.name).unwrap())).into_response()
        }
        Err(e) => Html(MoveEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn move_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let m = sqlx::query_as::<_, Move>("SELECT * FROM moves WHERE name LIKE ?")
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    if let Some(m) = m {
        let toml_content = toml::to_string(&m).unwrap();
        Html(MoveEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }.render().unwrap()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn move_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_m: Move = match toml::from_str(&payload.toml_content) {
        Ok(m) => m,
        Err(e) => {
            return Html(MoveEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response();
        }
    };

    let old_m = sqlx::query_as::<_, Move>("SELECT * FROM moves WHERE name LIKE ?").bind(&slug).fetch_one(&pool).await.unwrap();
    let mut new_m = new_m;
    new_m.id = old_m.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_m).unwrap(), payload.toml_content);

    let result = sqlx::query("UPDATE moves SET name = ?, type_id = ?, category = ?, power = ?, accuracy = ?, pp = ?, description = ? WHERE id = ?")
        .bind(&new_m.name).bind(new_m.type_id).bind(new_m.category).bind(new_m.power).bind(new_m.accuracy).bind(new_m.pp).bind(&new_m.description).bind(new_m.id).execute(&pool).await;

    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("moves").bind(new_m.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/moves/{}", slug)).into_response()
        }
        Err(e) => {
            Html(MoveEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
        }
    }
}

pub async fn move_delete(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let m = sqlx::query_as::<_, Move>("SELECT * FROM moves WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(m) = m {
        let mut tx = pool.begin().await.unwrap();
        let res = async {
            sqlx::query("DELETE FROM learnset WHERE move_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move1_id = NULL WHERE move1_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move2_id = NULL WHERE move2_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move3_id = NULL WHERE move3_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move4_id = NULL WHERE move4_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM moves WHERE id = ?").bind(m.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => { tx.commit().await.unwrap(); Redirect::to("/editor/moves").into_response() }
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// Types
#[derive(Template)]
#[template(path = "editor/types/list.html")]
pub struct TypeListTemplate {
    pub types: Vec<Type>,
    pub user_role: Option<String>,
}

pub async fn type_list(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let types = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE id > 0 ORDER BY id").fetch_all(&pool).await.unwrap_or_default();
    Html(TypeListTemplate { types, user_role: Some("editor".into()) }.render().unwrap())
}

#[derive(Template)]
#[template(path = "editor/types/edit.html")]
pub struct TypeEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

pub async fn type_new(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM types").fetch_one(&pool).await.unwrap_or(0);
    let default_t = Type { id: max_id + 1, name: "Novo Tipo".to_string(), hexcolor: "#FFFFFF".to_string(), icon: "icon.png".to_string() };
    let full = TypeFull {
        types: default_t,
        effectiveness: vec![TypeEffectiveness { attacker_id: max_id + 1, defender_id: 1, multiplier: 1.0 }],
    };
    Html(TypeEditTemplate { slug: "new".to_string(), toml_content: toml::to_string(&full).unwrap(), error: None, user_role: Some("editor".into()) }.render().unwrap())
}

pub async fn type_create(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let full: TypeFull = match toml::from_str(&payload.toml_content) {
        Ok(t) => t,
        Err(e) => return Html(TypeEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response(),
    };
    let new_t = full.types;

    let mut tx = pool.begin().await.unwrap();

    let res = async {
        sqlx::query("INSERT INTO types (id, name, hexcolor, icon) VALUES (?, ?, ?, ?)")
            .bind(new_t.id).bind(&new_t.name).bind(&new_t.hexcolor).bind(&new_t.icon).execute(&mut *tx).await?;

        for eff in &full.effectiveness {
            sqlx::query("INSERT INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)")
                .bind(new_t.id).bind(eff.defender_id).bind(eff.multiplier).execute(&mut *tx).await?;
        }
        Ok::<_, sqlx::Error>(())
    }.await;

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
                .bind(&trainer.id).bind("types").bind(new_t.id).bind(&format!("CREATE---{}", payload.toml_content)).execute(&pool).await;
            Redirect::to(&format!("/wiki/types/{}", filters::slugify(&new_t.name).unwrap())).into_response()
        }
        Err(e) => Html(TypeEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn type_edit(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let t = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(t) = t {
        let effectiveness = sqlx::query_as::<_, TypeEffectiveness>(
            "SELECT * FROM type_effectiveness WHERE attacker_id = ?"
        ).bind(t.id).fetch_all(&pool).await.unwrap_or_default();

        let full = TypeFull {
            types: t,
            effectiveness,
        };
        let toml_content = toml::to_string(&full).unwrap();
        Html(TypeEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }.render().unwrap()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}


pub async fn type_save(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let full: TypeFull = match toml::from_str(&payload.toml_content) {
        Ok(t) => t,
        Err(e) => {
            return Html(TypeEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response();
        }
    };
    let old_t = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE name LIKE ?").bind(&slug).fetch_one(&pool).await.unwrap();
    let mut new_t = full.types;
    new_t.id = old_t.id; // Evita ID Hijacking
    let full = TypeFull { types: new_t.clone(), ..full };
    let diff = format!("{}---{}", toml::to_string(&old_t).unwrap(), payload.toml_content);

    let mut tx = pool.begin().await.unwrap();

    let res = async {
        sqlx::query("UPDATE types SET name = ?, hexcolor = ?, icon = ? WHERE id = ?")
            .bind(&new_t.name).bind(&new_t.hexcolor).bind(&new_t.icon).bind(new_t.id).execute(&mut *tx).await?;

        sqlx::query("DELETE FROM type_effectiveness WHERE attacker_id = ?").bind(new_t.id).execute(&mut *tx).await?;
        for eff in &full.effectiveness {
            sqlx::query("INSERT INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)")
                .bind(new_t.id).bind(eff.defender_id).bind(eff.multiplier).execute(&mut *tx).await?;
        }
        Ok::<_, sqlx::Error>(())
    }.await;

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
                .bind(&trainer.id).bind("types").bind(new_t.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/types/{}", slug)).into_response()
        }
        Err(e) => Html(TypeEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn type_delete(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let t = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(t) = t {
        if t.id == 0 {
            return (StatusCode::BAD_REQUEST, "Cannot delete the fallback type").into_response();
        }
        let mut tx = pool.begin().await.unwrap();
        let res = async {
            sqlx::query("UPDATE pokemon SET type1_id = 0 WHERE type1_id = ?").bind(t.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE pokemon SET type2_id = NULL WHERE type2_id = ?").bind(t.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE moves SET type_id = 0 WHERE type_id = ?").bind(t.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM type_effectiveness WHERE attacker_id = ? OR defender_id = ?").bind(t.id).bind(t.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM types WHERE id = ?").bind(t.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => { tx.commit().await.unwrap(); Redirect::to("/editor/types").into_response() }
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// Abilities
#[derive(Template)]
#[template(path = "editor/abilities/list.html")]
pub struct AbilityListTemplate {
    pub abilities: Vec<Ability>,
    pub user_role: Option<String>,
}

pub async fn ability_list(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let abilities = sqlx::query_as::<_, Ability>("SELECT * FROM abilities ORDER BY name").fetch_all(&pool).await.unwrap_or_default();
    Html(AbilityListTemplate { abilities, user_role: Some("editor".into()) }.render().unwrap())
}

#[derive(Template)]
#[template(path = "editor/abilities/edit.html")]
pub struct AbilityEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

pub async fn ability_new(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM abilities").fetch_one(&pool).await.unwrap_or(0);
    let default_a = Ability { id: max_id + 1, name: "Nova Habilidade".to_string(), description: "Descrição da habilidade".to_string() };
    Html(AbilityEditTemplate { slug: "new".to_string(), toml_content: toml::to_string(&default_a).unwrap(), error: None, user_role: Some("editor".into()) }.render().unwrap())
}

pub async fn ability_create(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_a: Ability = match toml::from_str(&payload.toml_content) {
        Ok(a) => a,
        Err(e) => return Html(AbilityEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response(),
    };
    let result = sqlx::query("INSERT INTO abilities (id, name, description) VALUES (?, ?, ?)")
        .bind(new_a.id).bind(&new_a.name).bind(&new_a.description).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("abilities").bind(new_a.id).bind(&format!("CREATE---{}", payload.toml_content)).execute(&pool).await;
            Redirect::to(&format!("/wiki/abilities/{}", filters::slugify(&new_a.name).unwrap())).into_response()
        }
        Err(e) => Html(AbilityEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn ability_edit(
_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let a = sqlx::query_as::<_, Ability>("SELECT * FROM abilities WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(a) = a {
        let toml_content = toml::to_string(&a).unwrap();
        Html(AbilityEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }.render().unwrap()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn ability_save(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_a: Ability = match toml::from_str(&payload.toml_content) {
        Ok(a) => a,
        Err(e) => {
            return Html(AbilityEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response();
        }
    };
    let old_a = sqlx::query_as::<_, Ability>("SELECT * FROM abilities WHERE name LIKE ?").bind(&slug).fetch_one(&pool).await.unwrap();
    let mut new_a = new_a;
    new_a.id = old_a.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_a).unwrap(), payload.toml_content);
    let result = sqlx::query("UPDATE abilities SET name = ?, description = ? WHERE id = ?")
        .bind(&new_a.name).bind(&new_a.description).bind(new_a.id).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("abilities").bind(new_a.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/abilities/{}", slug)).into_response()
        }
        Err(e) => Html(AbilityEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn ability_delete(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let a = sqlx::query_as::<_, Ability>("SELECT * FROM abilities WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(a) = a {
        if a.id == 0 {
            return (StatusCode::BAD_REQUEST, "Cannot delete the fallback ability").into_response();
        }
        let mut tx = pool.begin().await.unwrap();
        let res = async {
            sqlx::query("UPDATE pokemon_abilities SET ability_id = 0 WHERE ability_id = ?").bind(a.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET ability_id = 0 WHERE ability_id = ?").bind(a.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM abilities WHERE id = ?").bind(a.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => { tx.commit().await.unwrap(); Redirect::to("/editor/abilities").into_response() }
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// Natures
#[derive(Template)]
#[template(path = "editor/natures/list.html")]
pub struct NatureListTemplate {
    pub natures: Vec<Nature>,
    pub user_role: Option<String>,
}

pub async fn nature_list(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let natures = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE id > 0 ORDER BY name").fetch_all(&pool).await.unwrap_or_default();
    Html(NatureListTemplate { natures, user_role: Some("editor".into()) }.render().unwrap())
}

#[derive(Template)]
#[template(path = "editor/natures/edit.html")]
pub struct NatureEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

pub async fn nature_new(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM natures").fetch_one(&pool).await.unwrap_or(0);
    let default_n = Nature { id: max_id + 1, name: "Nova Natureza".to_string(), increased_stat: None, decreased_stat: None };
    Html(NatureEditTemplate { slug: "new".to_string(), toml_content: toml::to_string(&default_n).unwrap(), error: None, user_role: Some("editor".into()) }.render().unwrap())
}

pub async fn nature_create(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_n: Nature = match toml::from_str(&payload.toml_content) {
        Ok(n) => n,
        Err(e) => return Html(NatureEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response(),
    };
    let result = sqlx::query("INSERT INTO natures (id, name, increased_stat, decreased_stat) VALUES (?, ?, ?, ?)")
        .bind(new_n.id).bind(&new_n.name).bind(&new_n.increased_stat).bind(&new_n.decreased_stat).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("natures").bind(new_n.id).bind(&format!("CREATE---{}", payload.toml_content)).execute(&pool).await;
            Redirect::to("/wiki/mechanics/nature").into_response()
        }
        Err(e) => Html(NatureEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn nature_edit(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let n = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(n) = n {
        let toml_content = toml::to_string(&n).unwrap();
        Html(NatureEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }.render().unwrap()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn nature_save(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_n: Nature = match toml::from_str(&payload.toml_content) {
        Ok(n) => n,
        Err(e) => {
            return Html(NatureEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response();
        }
    };
    let old_n = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE name LIKE ?").bind(&slug).fetch_one(&pool).await.unwrap();
    let mut new_n = new_n;
    new_n.id = old_n.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_n).unwrap(), payload.toml_content);
    let result = sqlx::query("UPDATE natures SET name = ?, increased_stat = ?, decreased_stat = ? WHERE id = ?")
        .bind(&new_n.name).bind(&new_n.increased_stat).bind(&new_n.decreased_stat).bind(new_n.id).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("natures").bind(new_n.id).bind(&diff).execute(&pool).await;
            Redirect::to("/wiki/mechanics/nature").into_response()
        }
        Err(e) => Html(NatureEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn nature_delete(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let n = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(n) = n {
        if n.id == 0 {
            return (StatusCode::BAD_REQUEST, "Cannot delete the fallback nature").into_response();
        }
        let mut tx = pool.begin().await.unwrap();
        let res = async {
            sqlx::query("UPDATE team_members SET nature_id = 0 WHERE nature_id = ?").bind(n.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM natures WHERE id = ?").bind(n.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => { tx.commit().await.unwrap(); Redirect::to("/editor/natures").into_response() }
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// Status Conditions
#[derive(Template)]
#[template(path = "editor/status/list.html")]
pub struct StatusListTemplate {
    pub status_list: Vec<StatusCondition>,
    pub user_role: Option<String>,
}

pub async fn status_list(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let status_list = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions ORDER BY id").fetch_all(&pool).await.unwrap_or_default();
    Html(StatusListTemplate { status_list, user_role: Some("editor".into()) }.render().unwrap())
}

#[derive(Template)]
#[template(path = "editor/status/edit.html")]
pub struct StatusEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

pub async fn status_new(_session: EditorSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM status_conditions").fetch_one(&pool).await.unwrap_or(0);
    let default_s = StatusCondition { id: max_id + 1, name: "Novo Status".to_string(), description: "Descrição do status".to_string(), icon: "icon.png".to_string(), is_volatile: false };
    Html(StatusEditTemplate { slug: "new".to_string(), toml_content: toml::to_string(&default_s).unwrap(), error: None, user_role: Some("editor".into()) }.render().unwrap())
}

pub async fn status_create(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_s: StatusCondition = match toml::from_str(&payload.toml_content) {
        Ok(s) => s,
        Err(e) => return Html(StatusEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response(),
    };
    let result = sqlx::query("INSERT INTO status_conditions (id, name, description, icon, is_volatile) VALUES (?, ?, ?, ?, ?)")
        .bind(new_s.id).bind(&new_s.name).bind(&new_s.description).bind(&new_s.icon).bind(new_s.is_volatile).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("status_conditions").bind(new_s.id).bind(&format!("CREATE---{}", payload.toml_content)).execute(&pool).await;
            Redirect::to("/wiki/mechanics/status").into_response()
        }
        Err(e) => Html(StatusEditTemplate { slug: "new".to_string(), toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn status_edit(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let s = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(s) = s {
        let toml_content = toml::to_string(&s).unwrap();
        Html(StatusEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }.render().unwrap()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn status_save(EditorSession(trainer): EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>, Form(payload): Form<EditForm>) -> impl IntoResponse {
    let new_s: StatusCondition = match toml::from_str(&payload.toml_content) {
        Ok(s) => s,
        Err(e) => {
            return Html(StatusEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response();
        }
    };
    let old_s = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions WHERE name LIKE ?").bind(&slug).fetch_one(&pool).await.unwrap();
    let mut new_s = new_s;
    new_s.id = old_s.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_s).unwrap(), payload.toml_content);
    let result = sqlx::query("UPDATE status_conditions SET name = ?, description = ?, icon = ?, is_volatile = ? WHERE id = ?")
        .bind(&new_s.name).bind(&new_s.description).bind(&new_s.icon).bind(new_s.is_volatile).bind(new_s.id).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("status_conditions").bind(new_s.id).bind(&diff).execute(&pool).await;
            Redirect::to("/wiki/mechanics/status").into_response()
        }
        Err(e) => Html(StatusEditTemplate { slug, toml_content: payload.toml_content, error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()) }.render().unwrap()).into_response()
    }
}

pub async fn status_delete(_session: EditorSession, State(pool): State<SqlitePool>, Path(slug): Path<String>) -> impl IntoResponse {
    let s = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions WHERE name LIKE ?").bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(s) = s {
        let result = sqlx::query("DELETE FROM status_conditions WHERE id = ?").bind(s.id).execute(&pool).await;
        match result {
            Ok(_) => Redirect::to("/editor/status").into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response(),
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
