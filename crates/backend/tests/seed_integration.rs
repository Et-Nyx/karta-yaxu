use sqlx::SqlitePool;
use std::fs;
use tempfile::tempdir;
use backend::seed::run_seed_cli;

#[tokio::test]
async fn test_seed_integration() {
    let dir = tempdir().unwrap();
    let base_path = dir.path();
    unsafe {
        std::env::set_var("SEED_DATA_PATH", base_path);
    }

    // Create minimal TOML files
    fs::write(base_path.join("type.toml"), r##"
[[type]]
id = 1
name = "Normal"
hexcolor = "#A8A77A"
icon = "normal.png"
"##).unwrap();

    fs::write(base_path.join("ability.toml"), r##"
[[ability]]
id = 1
name = "Overgrow"
description = "Powers up Grass-type moves."
"##).unwrap();

    fs::write(base_path.join("item.toml"), r##"
[[item]]
id = 1
name = "Potion"
description = "Restores 20 HP."
icon = "potion.png"
"##).unwrap();

    fs::write(base_path.join("status_condition.toml"), r##"
[[status_condition]]
id = 1
name = "Burn"
description = "Burns the target."
icon = "burn.png"
is_volatile = false
"##).unwrap();

    fs::write(base_path.join("nature.toml"), r##"
[[nature]]
id = 1
name = "Hardy"
"##).unwrap();

    fs::write(base_path.join("move.toml"), r##"
[[move]]
id = 1
name = "Tackle"
type_id = 1
category = "physical"
power = 40
accuracy = 100
pp = 35
description = "A physical attack in which the user charges and slams into the target with its whole body."
"##).unwrap();

    fs::write(base_path.join("type_effectiveness.toml"), r##"
[[type_effectiveness]]
attacker_id = 1
defender_id = 1
multiplier = 1.0
"##).unwrap();

    fs::write(base_path.join("pokemon.toml"), r##"
[[pokemon]]
id = 1
dex_number = 1
name = "Bulbasaur"
sprite = "bulbasaur.png"
sprite_shiny = "bulbasaur_s.png"
type1_id = 1
hp = 45
atk = 49
def = 49
spa = 65
spd = 65
spe = 45
pokedex_entry = "A strange seed."
"##).unwrap();

    fs::write(base_path.join("pokemon_ability.toml"), r##"
[[pokemon_ability]]
pokemon_id = 1
ability_id = 1
slot = 1
"##).unwrap();

    fs::write(base_path.join("evolution.toml"), r##"evolution = []"##).unwrap();

    fs::write(base_path.join("pokemon_form.toml"), r##"pokemon_form = []"##).unwrap();

    fs::write(base_path.join("learnset.toml"), r##"
[[learnset]]
pokemon_id = 1
move_id = 1
method = "level_up"
level = 1
"##).unwrap();

    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Run seed
    run_seed_cli(&pool, vec![]).await.unwrap();

    // Verify
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pokemon").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    
    let pkmn: engine::models::Pokemon = sqlx::query_as("SELECT * FROM pokemon WHERE id = 1").fetch_one(&pool).await.unwrap();
    assert_eq!(pkmn.name, "Bulbasaur");
}
