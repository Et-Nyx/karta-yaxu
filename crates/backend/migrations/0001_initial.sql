-- Tipos
CREATE TABLE IF NOT EXISTS types (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    hexcolor TEXT NOT NULL,
    icon TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS type_effectiveness (
    attacker_id INTEGER NOT NULL REFERENCES types(id),
    defender_id INTEGER NOT NULL REFERENCES types(id),
    multiplier REAL NOT NULL,
    PRIMARY KEY (attacker_id, defender_id)
);

-- Items
CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    icon TEXT NOT NULL
);

-- Natures
CREATE TABLE IF NOT EXISTS natures (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    increased_stat TEXT,
    decreased_stat TEXT
);

-- Abilities
CREATE TABLE IF NOT EXISTS abilities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT ''
);

-- Pokémon
CREATE TABLE IF NOT EXISTS pokemon (
    id INTEGER PRIMARY KEY,
    dex_number INTEGER NOT NULL,
    name TEXT NOT NULL UNIQUE,
    sprite TEXT NOT NULL,
    sprite_shiny TEXT NOT NULL,
    type1_id INTEGER NOT NULL REFERENCES types(id),
    type2_id INTEGER REFERENCES types(id),
    hp INTEGER NOT NULL,
    atk INTEGER NOT NULL,
    def INTEGER NOT NULL,
    spa INTEGER NOT NULL,
    spd INTEGER NOT NULL,
    spe INTEGER NOT NULL,
    pokedex_entry TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS pokemon_abilities (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    ability_id INTEGER NOT NULL REFERENCES abilities(id),
    slot INTEGER NOT NULL,
    PRIMARY KEY (pokemon_id, slot)
);

CREATE TABLE IF NOT EXISTS evolutions (
    from_id INTEGER NOT NULL REFERENCES pokemon(id),
    to_id INTEGER NOT NULL REFERENCES pokemon(id),
    condition TEXT NOT NULL,
    condition_value TEXT,
    PRIMARY KEY (from_id, to_id)
);

CREATE TABLE IF NOT EXISTS pokemon_forms (
    base_id INTEGER NOT NULL REFERENCES pokemon(id),
    form_id INTEGER NOT NULL REFERENCES pokemon(id),
    form_name TEXT NOT NULL,
    PRIMARY KEY (base_id, form_id)
);

-- Moves
CREATE TABLE IF NOT EXISTS moves (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    type_id INTEGER NOT NULL REFERENCES types(id),
    category TEXT NOT NULL CHECK(category IN ('physical', 'special', 'status')),
    power INTEGER,
    accuracy INTEGER,
    pp INTEGER NOT NULL,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS learnset (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    move_id INTEGER NOT NULL REFERENCES moves(id),
    method TEXT NOT NULL CHECK(method IN ('level_up', 'tm', 'egg', 'tutor')),
    level INTEGER,
    PRIMARY KEY (pokemon_id, move_id, method)
);

-- Trainers
CREATE TABLE IF NOT EXISTS trainers (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK(role IN ('viewer', 'editor')),
    is_npc INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS teams (
    id INTEGER PRIMARY KEY,
    trainer_id TEXT NOT NULL REFERENCES trainers(id),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS team_members (
    id INTEGER PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES teams(id),
    slot INTEGER NOT NULL CHECK(slot >= 1 AND slot <= 6),
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    ability_id INTEGER NOT NULL REFERENCES abilities(id),
    nature_id INTEGER NOT NULL REFERENCES natures(id),
    held_item_id INTEGER REFERENCES items(id),
    move1_id INTEGER REFERENCES moves(id),
    move2_id INTEGER REFERENCES moves(id),
    move3_id INTEGER REFERENCES moves(id),
    move4_id INTEGER REFERENCES moves(id)
);

CREATE TABLE IF NOT EXISTS battles (
    id INTEGER PRIMARY KEY,
    trainer_id_1 TEXT NOT NULL REFERENCES trainers(id),
    trainer_id_2 TEXT NOT NULL REFERENCES trainers(id),
    team_id_1 INTEGER NOT NULL REFERENCES teams(id),
    team_id_2 INTEGER NOT NULL REFERENCES teams(id),
    winner_id TEXT REFERENCES trainers(id),
    log TEXT NOT NULL DEFAULT '',
    played_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS edits (
    id INTEGER PRIMARY KEY,
    trainer_id TEXT NOT NULL REFERENCES trainers(id),
    table_name TEXT NOT NULL,
    record_id INTEGER NOT NULL,
    diff TEXT NOT NULL,
    edited_at TEXT NOT NULL DEFAULT (datetime('now'))
);
