#!/usr/bin/env python3
"""
fetch_extras.py — Fetch extra Pokémon data from PokeAPI.

Generates TOML entries for Mega Evolutions, Gigantamax forms, Regional forms,
Regional evolutions, Convergent species, Paradox forms, Pre-evolutions, and
Cross-gen evolutions of Gen 1 Pokémon.

APPENDS to existing TOML files in data/template/ and OVERWRITES
pokemon_form.toml. Must be run AFTER fetch_pokeapi.py.

ID Scheme:
    Forms:      offset + base_dex * 10 + variant_index
    New species: National Dex number (directly)

    Offsets: Mega=10000, Gmax=20000, Alolan=30000, Galarian=40000,
             Hisuian=50000, Paldean=60000

Usage:
    python3 scripts/fetch_pokeapi.py   # First, generate base data
    python3 scripts/fetch_extras.py    # Then, add extra forms/species
"""

import json
import os
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path
from collections import defaultdict

# ── Configuration ────────────────────────────────────────────────────────────

POKEAPI_BASE = "https://pokeapi.co/api/v2"
SPRITE_BASE = "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon"

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CACHE_DIR = PROJECT_ROOT / "scripts" / ".cache"
DATA_DIR = PROJECT_ROOT / "data" / "template"
SPRITE_DIR = PROJECT_ROOT / "static" / "sprites" / "pokemon"
SHINY_DIR = PROJECT_ROOT / "static" / "sprites" / "shiny"

RATE_LIMIT_S = 0.1
_last_request = 0.0

# ── ID Formula ──────────────────────────────────────────────────────────────
# id = OFFSET + base_dex * 10 + variant

OFFSET_MEGA = 10_000
OFFSET_GMAX = 20_000
OFFSET_ALOLAN = 30_000
OFFSET_GALARIAN = 40_000
OFFSET_HISUIAN = 50_000
OFFSET_PALDEAN = 60_000


def form_id(offset, base_dex, variant=0):
    return offset + base_dex * 10 + variant


# ── Special name handling ────────────────────────────────────────────────────

SPECIAL_NAMES = {
    "nidoran-f": "Nidoran\u2640",
    "nidoran-m": "Nidoran\u2642",
    "mr-mime": "Mr. Mime",
    "farfetchd": "Farfetch'd",
    "mr-rime": "Mr. Rime",
    "sirfetched": "Sirfetch'd",
    "mime-jr": "Mime Jr.",
    "porygon-z": "Porygon-Z",
    "type-null": "Type: Null",
}


def api_name_to_display(name):
    if name in SPECIAL_NAMES:
        return SPECIAL_NAMES[name]
    return name.replace("-", " ").title()


def escape_toml_string(s):
    s = s.replace("\\", "\\\\")
    s = s.replace('"', '\\"')
    s = s.replace("\n", " ")
    s = s.replace("\r", "")
    s = s.replace("\f", " ")
    while "  " in s:
        s = s.replace("  ", " ")
    return s.strip()


# ── HTTP / Cache ─────────────────────────────────────────────────────────────

def fetch_json(url, cache_subdir="extras"):
    global _last_request
    cache_key = url.replace(POKEAPI_BASE + "/", "").rstrip("/").replace("/", "_")
    cache_path = CACHE_DIR / cache_subdir / f"{cache_key}.json"
    cache_path.parent.mkdir(parents=True, exist_ok=True)

    if cache_path.exists():
        with open(cache_path) as f:
            return json.load(f)

    elapsed = time.time() - _last_request
    if elapsed < RATE_LIMIT_S:
        time.sleep(RATE_LIMIT_S - elapsed)

    try:
        req = urllib.request.Request(url, headers={"User-Agent": "KartaYaxu/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  \u26a0 HTTP {e.code} for {url}")
        raise
    finally:
        _last_request = time.time()

    with open(cache_path, "w") as f:
        json.dump(data, f)
    return data


def download_file(url, dest):
    if dest.exists():
        return False
    dest.parent.mkdir(parents=True, exist_ok=True)

    global _last_request
    elapsed = time.time() - _last_request
    if elapsed < RATE_LIMIT_S:
        time.sleep(RATE_LIMIT_S - elapsed)

    try:
        req = urllib.request.Request(url, headers={"User-Agent": "KartaYaxu/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            dest.write_bytes(resp.read())
        _last_request = time.time()
        return True
    except Exception as e:
        print(f"  \u26a0 Failed to download {url}: {e}")
        _last_request = time.time()
        return False


# ── Read existing TOML IDs ───────────────────────────────────────────────────

def read_existing_ids(toml_path, field="id"):
    ids = set()
    if not Path(toml_path).exists():
        return ids
    for line in Path(toml_path).read_text().split("\n"):
        stripped = line.strip()
        if stripped.startswith(f"{field} = "):
            try:
                ids.add(int(stripped.split("= ")[1]))
            except ValueError:
                pass
    return ids


# ── Extra Pokémon Manifest ───────────────────────────────────────────────────
# Format: (pokeapi_slug, display_name, base_dex_or_national_dex, variant_index, version_group)
# For forms: base_dex is the Gen 1 dex number, ID = offset + base_dex*10 + variant
# For new species: national_dex is the National Dex number, used directly as ID

# Category groups with their offset (None for new species that use national dex)

# --- Mega Evolutions (Gen 6) ---
# (slug, display_name, base_dex, variant, mega_stone)
MEGAS_GEN6 = [
    ("venusaur-mega",     "Mega Venusaur",      3, 0, "Venusaurite"),
    ("charizard-mega-x",  "Mega Charizard X",   6, 0, "Charizardite X"),
    ("charizard-mega-y",  "Mega Charizard Y",   6, 1, "Charizardite Y"),
    ("blastoise-mega",    "Mega Blastoise",     9, 0, "Blastoisinite"),
    ("beedrill-mega",     "Mega Beedrill",      15, 0, "Beedrillite"),
    ("pidgeot-mega",      "Mega Pidgeot",       18, 0, "Pidgeotite"),
    ("alakazam-mega",     "Mega Alakazam",      65, 0, "Alakazite"),
    ("slowbro-mega",      "Mega Slowbro",       80, 0, "Slowbronite"),
    ("gengar-mega",       "Mega Gengar",        94, 0, "Gengarite"),
    ("kangaskhan-mega",   "Mega Kangaskhan",   115, 0, "Kangaskhanite"),
    ("pinsir-mega",       "Mega Pinsir",       127, 0, "Pinsirite"),
    ("gyarados-mega",     "Mega Gyarados",     130, 0, "Gyaradosite"),
    ("aerodactyl-mega",   "Mega Aerodactyl",   142, 0, "Aerodactylite"),
    ("mewtwo-mega-x",     "Mega Mewtwo X",     150, 0, "Mewtwonite X"),
    ("mewtwo-mega-y",     "Mega Mewtwo Y",     150, 1, "Mewtwonite Y"),
]

# --- Mega Evolutions (Legends: Z-A) ---
MEGAS_ZA = [
    ("raichu-mega-x",    "Mega Raichu X",      26, 0, "Raichunite X"),
    ("raichu-mega-y",    "Mega Raichu Y",      26, 1, "Raichunite Y"),
    ("clefable-mega",    "Mega Clefable",       36, 0, "Clefablite"),
    ("victreebel-mega",  "Mega Victreebel",     71, 0, "Victreebelite"),
    ("starmie-mega",     "Mega Starmie",       120, 0, "Starmieite"),
    ("dragonite-mega",   "Mega Dragonite",     149, 0, "Dragoniteite"),
]

# --- Gigantamax Forms ---
# (slug, display_name, base_dex, variant)
# Gmax stats = base Pokemon stats; no learnset of their own
GMAX_FORMS = [
    ("venusaur-gmax",    "Gigantamax Venusaur",     3, 0),
    ("charizard-gmax",   "Gigantamax Charizard",     6, 0),
    ("blastoise-gmax",   "Gigantamax Blastoise",    9, 0),
    ("butterfree-gmax",  "Gigantamax Butterfree",   12, 0),
    ("pikachu-gmax",     "Gigantamax Pikachu",      25, 0),
    ("meowth-gmax",      "Gigantamax Meowth",       52, 0),
    ("machamp-gmax",     "Gigantamax Machamp",      68, 0),
    ("gengar-gmax",      "Gigantamax Gengar",       94, 0),
    ("kingler-gmax",     "Gigantamax Kingler",      99, 0),
    ("lapras-gmax",      "Gigantamax Lapras",      131, 0),
    ("eevee-gmax",       "Gigantamax Eevee",       133, 0),
    ("snorlax-gmax",     "Gigantamax Snorlax",     143, 0),
]

# --- Alolan Forms ---
ALOLAN_FORMS = [
    ("rattata-alola",    "Alolan Rattata",      19, 0),
    ("raticate-alola",   "Alolan Raticate",      20, 0),
    ("raichu-alola",     "Alolan Raichu",        26, 0),
    ("sandshrew-alola",  "Alolan Sandshrew",     27, 0),
    ("sandslash-alola",  "Alolan Sandslash",     28, 0),
    ("vulpix-alola",     "Alolan Vulpix",        37, 0),
    ("ninetales-alola",  "Alolan Ninetales",     38, 0),
    ("diglett-alola",    "Alolan Diglett",       50, 0),
    ("dugtrio-alola",    "Alolan Dugtrio",        51, 0),
    ("meowth-alola",     "Alolan Meowth",        52, 0),
    ("persian-alola",    "Alolan Persian",         53, 0),
    ("geodude-alola",    "Alolan Geodude",       74, 0),
    ("graveler-alola",  "Alolan Graveler",       75, 0),
    ("golem-alola",      "Alolan Golem",         76, 0),
    ("grimer-alola",     "Alolan Grimer",         88, 0),
    ("muk-alola",        "Alolan Muk",            89, 0),
    ("exeggutor-alola",  "Alolan Exeggutor",     103, 0),
    ("marowak-alola",    "Alolan Marowak",       105, 0),
]

# --- Galarian Forms ---
GALARIAN_FORMS = [
    ("meowth-galar", "Galarian Meowth", 52, 0),
    ("ponyta-galar", "Galarian Ponyta", 77, 0),
    ("rapidash-galar", "Galarian Rapidash", 78, 0),
    ("slowpoke-galar", "Galarian Slowpoke", 79, 0),
    ("slowbro-galar", "Galarian Slowbro", 80, 0),
    ("slowking-galar", "Galarian Slowking", 199, 0),
    ("farfetchd-galar", "Galarian Farfetch'd", 83, 0),
    ("weezing-galar", "Galarian Weezing", 110, 0),
    ("mr-mime-galar", "Galarian Mr. Mime", 122, 0),
    ("articuno-galar", "Galarian Articuno", 144, 0),
    ("zapdos-galar", "Galarian Zapdos", 145, 0),
    ("moltres-galar", "Galarian Moltres", 146, 0),
]

# --- Hisuian Forms ---
HISUIAN_FORMS = [
    ("growlithe-hisui",  "Hisuian Growlithe",   58, 0),
    ("arcanine-hisui",   "Hisuian Arcanine",     59, 0),
    ("voltorb-hisui",    "Hisuian Voltorb",      100, 0),
    ("electrode-hisui",  "Hisuian Electrode",    101, 0),
]

# --- Paldean Forms ---
PALDEAN_FORMS = [
    ("tauros-paldea-combat-breed", "Paldean Tauros (Combat)", 128, 0),
    ("tauros-paldea-blaze-breed",  "Paldean Tauros (Blaze)",  128, 1),
    ("tauros-paldea-aqua-breed",   "Paldean Tauros (Aqua)",   128, 2),
]

# --- Pre-evolutions (Baby Pokémon) ---
# (slug, display_name, national_dex, version_group)
PRE_EVOLUTIONS = [
    ("pichu",      "Pichu",     172, "heartgold-soulsilver"),
    ("cleffa",     "Cleffa",    173, "heartgold-soulsilver"),
    ("igglybuff",  "Igglybuff", 174, "heartgold-soulsilver"),
    ("tyrogue",    "Tyrogue",   236, "heartgold-soulsilver"),
    ("smoochum",   "Smoochum",  238, "heartgold-soulsilver"),
    ("elekid",     "Elekid",    239, "platinum"),
    ("magby",      "Magby",     240, "platinum"),
    ("mime-jr",    "Mime Jr.",  439, "platinum"),
    ("happiny",    "Happiny",   440, "platinum"),
    ("munchlax",   "Munchlax",  446, "platinum"),
]

# --- Cross-generation Evolutions ---
CROSS_GEN_EVOLUTIONS = [
    ("crobat",      "Crobat",      169, "heartgold-soulsilver"),
    ("bellossom",   "Bellossom",   182, "heartgold-soulsilver"),
    ("politoed",    "Politoed",    186, "heartgold-soulsilver"),
    ("slowking",    "Slowking",    199, "heartgold-soulsilver"),
    ("steelix",     "Steelix",     208, "heartgold-soulsilver"),
    ("kingdra",     "Kingdra",     230, "heartgold-soulsilver"),
    ("scizor",      "Scizor",     212, "heartgold-soulsilver"),
    ("espeon",      "Espeon",     196, "heartgold-soulsilver"),
    ("umbreon",     "Umbreon",    197, "heartgold-soulsilver"),
    ("porygon2",    "Porygon2",   233, "heartgold-soulsilver"),
    ("blissey",     "Blissey",    242, "heartgold-soulsilver"),
    ("hitmontop",   "Hitmontop",  237, "heartgold-soulsilver"),
    ("magnezone",   "Magnezone",  462, "platinum"),
    ("lickilicky",  "Lickilicky", 463, "platinum"),
    ("rhyperior",   "Rhyperior",  464, "platinum"),
    ("tangrowth",   "Tangrowth",  465, "platinum"),
    ("electivire",  "Electivire", 466, "platinum"),
    ("magmortar",   "Magmortar",  467, "platinum"),
    ("porygon-z",   "Porygon-Z", 469, "platinum"),
    ("leafeon",     "Leafeon",    470, "platinum"),
    ("glaceon",     "Glaceon",    471, "platinum"),
    ("sylveon",     "Sylveon",    700, "x-y"),
    ("kleavor",     "Kleavor",    900, "legends-arceus"),
    ("annihilape",  "Annihilape", 979, "scarlet-violet"),
]

# --- Regional Evolutions (from Galarian forms) ---
REGIONAL_EVOLUTIONS = [
    ("perrserker", "Perrserker", 862, "sword-shield"),
    ("sirfetchd", "Sirfetch'd", 865, "sword-shield"),
    ("mr-rime", "Mr. Rime", 866, "sword-shield"),
]

# --- Convergent Species ---
CONVERGENT_SPECIES = [
    ("toedscool",    "Toedscool",     948, "scarlet-violet"),
    ("toedscruel",   "Toedscruel",    949, "scarlet-violet"),
    ("wiglett",      "Wiglett",        960, "scarlet-violet"),
    ("wugtrio",      "Wugtrio",        961, "scarlet-violet"),
]

# --- Paradox Forms ---
PARADOX_FORMS = [
    ("scream-tail",   "Scream Tail",    985, "scarlet-violet"),
    ("sandy-shocks",  "Sandy Shocks",   989, "scarlet-violet"),
]

# --- Evolution links for new species ---
# (from_id, to_id, condition, condition_value)
# These are manually curated because cross-gen evolution conditions
# aren't always available from PokeAPI's chain data for new species.
EVOLUTION_LINKS = [
    # Pre-evolutions → Gen 1 Pokémon
    (172, 25, "happiness", None),
    (173, 35, "happiness", None),
    (174, 39, "happiness", None),
    (236, 106, "level", "20"),
    (236, 107, "level", "20"),
    (236, 237, "level", "20"),
    (238, 124, "level", "30"),
    (239, 125, "level", "30"),
    (240, 126, "level", "30"),
    (446, 143, "happiness", None),
    (440, 113, "item", "Oval Stone"),
    (439, 122, "level", "15"),
    # Cross-gen evolutions from Gen 1 Pokémon
    (42, 169, "happiness", None),
    (44, 182, "item", "Sun Stone"),
    (61, 186, "trade", "King's Rock"),
    (79, 199, "trade", "King's Rock"),
    (95, 208, "trade", "Metal Coat"),
    (117, 230, "trade", "Dragon Scale"),
    (123, 212, "trade", "Metal Coat"),
    (123, 900, "item", "Black Augurite"),
    (133, 196, "happiness", "day"),
    (133, 197, "happiness", "night"),
    (133, 470, "item", "Leaf Stone"),
    (133, 471, "item", "Ice Stone"),
    (133, 700, "affection", "Fairy move"),
    (137, 233, "trade", "Upgrade"),
    (233, 469, "trade", "Dubious Disc"),
    (113, 242, "happiness", None),
    (82, 462, "item", "Thunder Stone"),
    (108, 463, "level", "33"),
    (112, 464, "trade", "Protector"),
    (114, 465, "level", "34"),
    (125, 466, "trade", "Electirizer"),
    (126, 467, "trade", "Magmarizer"),
    (57, 979, "level", "35"),
    # Regional evolutions
    (40520, 862, "level", "28"),
    (40830, 865, "level", "25"),
    (41220, 866, "level", "42"),
    (40790, 41990, "trade", "Galarica Wreath"),
]

# --- Regional form evolution links ---
# (from_id, to_id, condition, condition_value)
REGIONAL_EVOLUTION_LINKS = [
    # Alolan lines
    (30190, 30200, "level", "20"),
    (30270, 30280, "item", "Ice Stone"),
    (30370, 30380, "item", "Ice Stone"),
    (30500, 30510, "level", "26"),
    (30520, 30530, "happiness", None),
    (30740, 30750, "level", "25"),
    (30750, 30760, "trade", None),
    (30880, 30890, "level", "38"),
    # Cross-branch evolutions (standard Pokémon → Alolan form)
    (25, 30260, "item", "Thunder Stone"),
    (102, 31030, "item", "Leaf Stone"),
    (104, 31050, "level", "28"),
    # Galarian lines (NOTE: Galarian Meowth -> Perrserker already in EVOLUTION_LINKS as 40520->862)
    (40790, 40800, "item", "Galarica Cuff"),
]

# --- Form definitions for pokemon_form.toml ---
# (base_dex, form_id, form_name)
# Built programmatically from manifests


# ── STAT_ABBREV ───────────────────────────────────────────────────────────────

STAT_ABBREV = {
    "hp": "hp",
    "attack": "atk",
    "defense": "def",
    "special-attack": "spa",
    "special-defense": "spd",
    "speed": "spe",
}


# ── Phase 1: Fetch Form Pokémon (Mega, Regional, Gmax) ──────────────────────

def fetch_form_pokemon(slug, display_name, form_id_val, base_dex, category):
    """Fetch Pokemon data for a form (Mega, Regional, etc.) from PokeAPI."""
    print(f"  Fetching {display_name} ({slug})...", end="\r")
    try:
        poke_data = fetch_json(f"{POKEAPI_BASE}/pokemon/{slug}")
    except urllib.error.HTTPError:
        print(f"  \u26a0 {display_name} not found in PokeAPI, creating placeholder")
        return None

    types = sorted(poke_data["types"], key=lambda t: t["slot"])
    type1_id = int(types[0]["type"]["url"].rstrip("/").split("/")[-1])
    type2_id = int(types[1]["type"]["url"].rstrip("/").split("/")[-1]) if len(types) > 1 else None

    stats = {}
    for s in poke_data["stats"]:
        stat_name = s["stat"]["name"]
        stats[STAT_ABBREV.get(stat_name, stat_name)] = s["base_stat"]

    abilities = []
    for ab in poke_data["abilities"]:
        ab_id = int(ab["ability"]["url"].rstrip("/").split("/")[-1])
        slot = ab["slot"]
        hidden = ab.get("is_hidden", False)
        abilities.append((ab_id, slot, hidden))

    sprite_url = (
        poke_data["sprites"].get("front_default")
        or poke_data["sprites"].get("other", {}).get("official-artwork", {}).get("front_default", "")
        or ""
    )
    sprite_shiny_url = (
        poke_data["sprites"].get("front_shiny")
        or poke_data["sprites"].get("other", {}).get("official-artwork", {}).get("front_shiny", "")
        or ""
    )

    species_data = fetch_json(f"{POKEAPI_BASE}/pokemon-species/{poke_data['species']['name']}")
    pokedex_entry = ""
    for entry in species_data.get("flavor_text_entries", []):
        if entry.get("language", {}).get("name") == "en":
            pokedex_entry = escape_toml_string(entry.get("flavor_text", ""))
            break
    if not pokedex_entry:
        pokedex_entry = f"{display_name} form of {api_name_to_display(poke_data['species']['name'])}."

    learnset_data = []
    if category not in ("mega", "gmax"):
        learnset_data = fetch_learnset_for_form(slug, category)

    return {
        "id": form_id_val,
        "dex_number": base_dex,
        "name": display_name,
        "type1_id": type1_id,
        "type2_id": type2_id,
        "hp": stats.get("hp", 50),
        "atk": stats.get("atk", 50),
        "def": stats.get("def", 50),
        "spa": stats.get("spa", 50),
        "spd": stats.get("spd", 50),
        "spe": stats.get("spe", 50),
        "pokedex_entry": pokedex_entry,
        "abilities": abilities,
        "sprite": f"{slug}.png",
        "sprite_shiny": f"{slug}_shiny.png",
        "sprite_url": sprite_url,
        "sprite_shiny_url": sprite_shiny_url,
        "learnset": learnset_data,
    }


def fetch_new_species_pokemon(slug, display_name, national_dex, version_group):
    """Fetch data for a new species (pre-evo, cross-gen, etc.) from PokeAPI."""
    print(f"  Fetching {display_name} ({slug})...", end="\r")
    try:
        poke_data = fetch_json(f"{POKEAPI_BASE}/pokemon/{slug}")
    except urllib.error.HTTPError:
        print(f"  \u26a0 {display_name} not found in PokeAPI!")
        return None

    types = sorted(poke_data["types"], key=lambda t: t["slot"])
    type1_id = int(types[0]["type"]["url"].rstrip("/").split("/")[-1])
    type2_id = int(types[1]["type"]["url"].rstrip("/").split("/")[-1]) if len(types) > 1 else None

    stats = {}
    for s in poke_data["stats"]:
        stat_name = s["stat"]["name"]
        stats[STAT_ABBREV.get(stat_name, stat_name)] = s["base_stat"]

    abilities = []
    for ab in poke_data["abilities"]:
        ab_id = int(ab["ability"]["url"].rstrip("/").split("/")[-1])
        slot = ab["slot"]
        hidden = ab.get("is_hidden", False)
        abilities.append((ab_id, slot, hidden))

    species_data = fetch_json(
        f"{POKEAPI_BASE}/pokemon-species/{poke_data['species']['name']}"
    )
    pokedex_entry = ""
    for entry in reversed(species_data.get("flavor_text_entries", [])):
        if entry.get("language", {}).get("name") == "en":
            pokedex_entry = escape_toml_string(entry.get("flavor_text", ""))
            break
    if not pokedex_entry:
        pokedex_entry = display_name

    sprite_url = (
        poke_data["sprites"].get("front_default")
        or poke_data["sprites"].get("other", {}).get("official-artwork", {}).get("front_default", "")
        or ""
    )
    sprite_shiny_url = (
        poke_data["sprites"].get("front_shiny")
        or poke_data["sprites"].get("other", {}).get("official-artwork", {}).get("front_shiny", "")
        or ""
    )

    learnset_data = fetch_learnset(slug, version_group)

    return {
        "id": national_dex,
        "dex_number": national_dex,
        "name": display_name,
        "type1_id": type1_id,
        "type2_id": type2_id,
        "hp": stats.get("hp", 50),
        "atk": stats.get("atk", 50),
        "def": stats.get("def", 50),
        "spa": stats.get("spa", 50),
        "spd": stats.get("spd", 50),
        "spe": stats.get("spe", 50),
        "pokedex_entry": pokedex_entry,
        "abilities": abilities,
        "sprite": f"{slug}.png",
        "sprite_shiny": f"{slug}_shiny.png",
        "sprite_url": sprite_url,
        "sprite_shiny_url": sprite_shiny_url,
        "learnset": learnset_data,
    }


# ── Learnset fetching ────────────────────────────────────────────────────────

# Version group priority: prefer the latest available for each category
VERSION_PRIORITIES = {
    "mega": [],
    "gmax": [],
    "alolan": ["ultra-sun-ultra-moon", "sun-moon"],
    "galarian": ["sword-shield"],
    "hisuian": ["legends-arceus", "sword-shield"],
    "paldean": ["scarlet-violet"],
}


def fetch_learnset(slug, version_group):
    """Fetch learnset for a specific version group, with fallback."""
    try:
        poke_data = fetch_json(f"{POKEAPI_BASE}/pokemon/{slug}")
    except urllib.error.HTTPError:
        return []

    moves = []
    for move_entry in poke_data["moves"]:
        move_id = int(move_entry["move"]["url"].rstrip("/").split("/")[-1])
        for vgd in move_entry["version_group_details"]:
            vg = vgd["version_group"]["name"]
            if vg == version_group:
                method_name = vgd["move_learn_method"]["name"]
                level = vgd["level_learned_at"]
                if method_name == "level-up":
                    method = "level_up"
                elif method_name == "machine":
                    method = "tm"
                elif method_name == "egg":
                    method = "egg"
                elif method_name == "tutor":
                    method = "tutor"
                else:
                    method = "tutor"
                moves.append((move_id, method, level if level > 0 else None))
                break
    return moves


def fetch_learnset_for_form(slug, category):
    """Fetch learnset for a regional form using appropriate version group."""
    priorities = VERSION_PRIORITIES.get(category, [])
    for vg in priorities:
        moves = fetch_learnset(slug, vg)
        if moves:
            return moves
    return []


# ── Phase 2: Fetch abilities ─────────────────────────────────────────────────

def fetch_abilities(ability_ids, existing_ability_ids):
    """Fetch ability details for new ability IDs not already in the database."""
    abilities = []
    new_ids = sorted(ability_ids - existing_ability_ids)
    total = len(new_ids)
    for i, ab_id in enumerate(new_ids):
        print(f"  [{i+1}/{total}] Fetching ability #{ab_id}...", end="\r")
        try:
            data = fetch_json(f"{POKEAPI_BASE}/ability/{ab_id}")
        except urllib.error.HTTPError:
            continue

        desc = ""
        for entry in data.get("effect_entries", []):
            if entry.get("language", {}).get("name") == "en":
                desc = escape_toml_string(entry.get("short_effect", ""))
                break
        if not desc:
            for entry in data.get("flavor_text_entries", []):
                if entry.get("language", {}).get("name") == "en":
                    desc = escape_toml_string(entry.get("flavor_text", ""))
                    break

        abilities.append({
            "id": ab_id,
            "name": api_name_to_display(data["name"]),
            "description": desc or api_name_to_display(data["name"]),
        })
    print(f"  \u2713 Fetched {len(abilities)} new abilities" + " " * 30)
    return abilities


# ── Phase 3: Fetch moves ─────────────────────────────────────────────────────

def fetch_moves(move_ids, existing_move_ids):
    """Fetch move details for new move IDs not already in the database."""
    moves = []
    new_ids = sorted(move_ids - existing_move_ids)
    total = len(new_ids)
    for i, move_id in enumerate(new_ids):
        print(f"  [{i+1}/{total}] Fetching move #{move_id}...", end="\r")
        try:
            data = fetch_json(f"{POKEAPI_BASE}/move/{move_id}")
        except urllib.error.HTTPError:
            continue

        type_id = int(data["type"]["url"].rstrip("/").split("/")[-1])
        category = data.get("damage_class", {}).get("name", "status")
        power = data.get("power")
        accuracy = data.get("accuracy")
        pp = data.get("pp") or 5

        desc = ""
        for entry in data.get("flavor_text_entries", []):
            lang = entry.get("language", {}).get("name", "")
            vg = entry.get("version_group", {}).get("name", "")
            if lang == "en":
                desc = escape_toml_string(entry.get("flavor_text", ""))
                break
        if not desc:
            for entry in reversed(data.get("effect_entries", [])):
                if entry.get("language", {}).get("name") == "en":
                    desc = escape_toml_string(entry.get("short_effect", ""))
                    break

        moves.append({
            "id": move_id,
            "name": api_name_to_display(data["name"]),
            "type_id": type_id,
            "category": category,
            "power": power,
            "accuracy": accuracy,
            "pp": pp,
            "description": desc or api_name_to_display(data["name"]),
        })
    print(f"  \u2713 Fetched {len(moves)} new moves" + " " * 30)
    return moves


# ── Phase 4: Gmax form data (copy from base Pokémon) ────────────────────────

def fetch_gmax_as_form(slug, display_name, form_id_val, base_dex):
    """Try to fetch Gmax data from PokeAPI; fall back to base Pokémon copy."""
    print(f"  Fetching {display_name} ({slug})...", end="\r")
    try:
        poke_data = fetch_json(f"{POKEAPI_BASE}/pokemon/{slug}")
        types = sorted(poke_data["types"], key=lambda t: t["slot"])
        type1_id = int(types[0]["type"]["url"].rstrip("/").split("/")[-1])
        type2_id = int(types[1]["type"]["url"].rstrip("/").split("/")[-1]) if len(types) > 1 else None
        stats = {}
        for s in poke_data["stats"]:
            stats[STAT_ABBREV.get(s["stat"]["name"], s["stat"]["name"])] = s["base_stat"]
        abilities = []
        for ab in poke_data["abilities"]:
            ab_id = int(ab["ability"]["url"].rstrip("/").split("/")[-1])
            abilities.append((ab_id, ab["slot"], ab.get("is_hidden", False)))
        sprite_url = (
            poke_data["sprites"].get("front_default")
            or poke_data["sprites"].get("other", {}).get("official-artwork", {}).get("front_default", "")
            or ""
        )
        sprite_shiny_url = (
            poke_data["sprites"].get("front_shiny")
            or poke_data["sprites"].get("other", {}).get("official-artwork", {}).get("front_shiny", "")
            or ""
        )
    except urllib.error.HTTPError:
        base_slug = slug.replace("-gmax", "")
        try:
            poke_data = fetch_json(f"{POKEAPI_BASE}/pokemon/{base_slug}")
        except urllib.error.HTTPError:
            print(f"  \u26a0 Both {slug} and {base_slug} failed!")
            return None
        types = sorted(poke_data["types"], key=lambda t: t["slot"])
        type1_id = int(types[0]["type"]["url"].rstrip("/").split("/")[-1])
        type2_id = int(types[1]["type"]["url"].rstrip("/").split("/")[-1]) if len(types) > 1 else None
        stats = {}
        for s in poke_data["stats"]:
            stats[STAT_ABBREV.get(s["stat"]["name"], s["stat"]["name"])] = s["base_stat"]
        abilities = []
        for ab in poke_data["abilities"]:
            ab_id = int(ab["ability"]["url"].rstrip("/").split("/")[-1])
            abilities.append((ab_id, ab["slot"], ab.get("is_hidden", False)))
        sprite_url = poke_data["sprites"].get("front_default", "")
        sprite_shiny_url = ""

    base_name = display_name.replace("Gigantamax ", "")
    pokedex_entry = f"Gigantamax form of {base_name}."

    return {
        "id": form_id_val,
        "dex_number": base_dex,
        "name": display_name,
        "type1_id": type1_id,
        "type2_id": type2_id,
        "hp": stats.get("hp", 50),
        "atk": stats.get("atk", 50),
        "def": stats.get("def", 50),
        "spa": stats.get("spa", 50),
        "spd": stats.get("spd", 50),
        "spe": stats.get("spe", 50),
        "pokedex_entry": pokedex_entry,
        "abilities": abilities,
        "sprite": f"{slug}.png",
        "sprite_shiny": f"{slug}_shiny.png",
        "sprite_url": sprite_url,
        "sprite_shiny_url": sprite_shiny_url,
        "learnset": [],
    }


# ── TOML Generation ──────────────────────────────────────────────────────────

def append_pokemon_toml(entries):
    """Append [[pokemon]] entries to pokemon.toml."""
    lines = []
    if Path(DATA_DIR / "pokemon.toml").exists():
        existing = Path(DATA_DIR / "pokemon.toml").read_text()
        if not existing.endswith("\n"):
            lines.append("")
    lines.append("")
    lines.append("# ── Extra Pokémon (generated by fetch_extras.py) ──")
    lines.append("")

    for p in entries:
        lines.append("[[pokemon]]")
        lines.append(f'id = {p["id"]}')
        lines.append(f'dex_number = {p["dex_number"]}')
        lines.append(f'name = "{escape_toml_string(str(p["name"]))}"')
        lines.append(f'sprite = "{p["sprite"]}"')
        lines.append(f'sprite_shiny = "{p["sprite_shiny"]}"')
        lines.append(f'type1_id = {p["type1_id"]}')
        if p["type2_id"] is not None:
            lines.append(f'type2_id = {p["type2_id"]}')
        lines.append(f'hp = {p["hp"]}')
        lines.append(f'atk = {p["atk"]}')
        lines.append(f'def = {p["def"]}')
        lines.append(f'spa = {p["spa"]}')
        lines.append(f'spd = {p["spd"]}')
        lines.append(f'spe = {p["spe"]}')
        lines.append(f'pokedex_entry = "{escape_toml_string(str(p["pokedex_entry"]))}"')
        lines.append("")

    with open(DATA_DIR / "pokemon.toml", "a") as f:
        f.write("\n".join(lines))
    print(f"  \u2713 pokemon.toml ({len(entries)} extra entries appended)")


def append_ability_toml(abilities):
    """Append [[ability]] entries to ability.toml."""
    if not abilities:
        return
    lines = ["", "# ── Extra abilities (generated by fetch_extras.py) ──", ""]
    for a in abilities:
        lines.append("[[ability]]")
        lines.append(f'id = {a["id"]}')
        lines.append(f'name = "{escape_toml_string(a["name"])}"')
        lines.append(f'description = "{escape_toml_string(a["description"])}"')
        lines.append("")

    with open(DATA_DIR / "ability.toml", "a") as f:
        f.write("\n".join(lines))
    print(f"  \u2713 ability.toml ({len(abilities)} extra entries appended)")


def append_move_toml(moves):
    """Append [[move]] entries to move.toml."""
    if not moves:
        return
    lines = ["", "# ── Extra moves (generated by fetch_extras.py) ──", ""]
    for m in moves:
        lines.append("[[move]]")
        lines.append(f'id = {m["id"]}')
        lines.append(f'name = "{escape_toml_string(m["name"])}"')
        lines.append(f'type_id = {m["type_id"]}')
        lines.append(f'category = "{m["category"]}"')
        if m["power"] is not None:
            lines.append(f'power = {m["power"]}')
        if m["accuracy"] is not None:
            lines.append(f'accuracy = {m["accuracy"]}')
        lines.append(f'pp = {m["pp"]}')
        lines.append(f'description = "{escape_toml_string(m["description"])}"')
        lines.append("")

    with open(DATA_DIR / "move.toml", "a") as f:
        f.write("\n".join(lines))
    print(f"  \u2713 move.toml ({len(moves)} extra entries appended)")


def append_evolution_toml(evolution_links):
    """Append [[evolution]] entries to evolution.toml."""
    if not evolution_links:
        return
    lines = ["", "# ── Extra evolution links (generated by fetch_extras.py) ──", ""]
    for evo in evolution_links:
        from_id, to_id, condition, condition_value = evo
        lines.append("[[evolution]]")
        lines.append(f"from_id = {from_id}")
        lines.append(f"to_id = {to_id}")
        lines.append(f'condition = "{condition}"')
        if condition_value:
            lines.append(f'condition_value = "{condition_value}"')
        lines.append("")

    with open(DATA_DIR / "evolution.toml", "a") as f:
        f.write("\n".join(lines))
    print(f"  \u2713 evolution.toml ({len(evolution_links)} extra entries appended)")


def append_pokemon_ability_toml(pokemon_abilities):
    """Append [[pokemon_ability]] entries to pokemon_ability.toml."""
    if not pokemon_abilities:
        return
    lines = ["", "# ── Extra Pokémon-ability links (generated by fetch_extras.py) ──", ""]
    for pa in pokemon_abilities:
        lines.append("[[pokemon_ability]]")
        lines.append(f"pokemon_id = {pa[0]}")
        lines.append(f"ability_id = {pa[1]}")
        lines.append(f"slot = {pa[2]}")
        lines.append("")

    with open(DATA_DIR / "pokemon_ability.toml", "a") as f:
        f.write("\n".join(lines))
    print(f"  \u2713 pokemon_ability.toml ({len(pokemon_abilities)} extra entries appended)")


def append_learnset_toml(learnsets):
    """Append [[learnset]] entries to learnset.toml."""
    if not learnsets:
        return
    lines = ["", "# ── Extra learnset entries (generated by fetch_extras.py) ──", ""]
    for pokemon_id, move_id, method, level in learnsets:
        lines.append("[[learnset]]")
        lines.append(f"pokemon_id = {pokemon_id}")
        lines.append(f"move_id = {move_id}")
        lines.append(f'method = "{method}"')
        if level is not None:
            lines.append(f"level = {level}")
        lines.append("")

    with open(DATA_DIR / "learnset.toml", "a") as f:
        f.write("\n".join(lines))
    print(f"  \u2713 learnset.toml ({len(learnsets)} extra entries appended)")


def write_pokemon_form_toml(form_entries):
    """Overwrite pokemon_form.toml with all form relationships."""
    lines = []
    for base_id, form_id_val, form_name in form_entries:
        lines.append("[[pokemon_form]]")
        lines.append(f"base_id = {base_id}")
        lines.append(f"form_id = {form_id_val}")
        lines.append(f'form_name = "{escape_toml_string(form_name)}"')
        lines.append("")

    (DATA_DIR / "pokemon_form.toml").write_text("\n".join(lines))
    print(f"  \u2713 pokemon_form.toml ({len(form_entries)} entries, overwritten)")


# ── Sprite downloading ────────────────────────────────────────────────────────

def download_sprites(entries):
    """Download sprites for all extra Pokémon."""
    downloaded = 0
    skipped = 0
    total = len(entries)

    for i, entry in enumerate(entries):
        slug = entry["sprite"].replace(".png", "")
        poke_api_id = entry.get("poke_api_id")

        print(f"  [{i+1}/{total}] Sprites for {entry['name']}...", end="\r")

        # Normal sprite
        if entry.get("sprite_url"):
            normal_dest = SPRITE_DIR / f"{slug}.png"
            if download_file(entry["sprite_url"], normal_dest):
                downloaded += 1
            else:
                skipped += 1
        elif poke_api_id:
            normal_url = f"{SPRITE_BASE}/{poke_api_id}.png"
            normal_dest = SPRITE_DIR / f"{slug}.png"
            if download_file(normal_url, normal_dest):
                downloaded += 1
            else:
                skipped += 1

        # Shiny sprite
        if entry.get("sprite_shiny_url"):
            shiny_dest = SHINY_DIR / f"{slug}_shiny.png"
            if download_file(entry["sprite_shiny_url"], shiny_dest):
                downloaded += 1
            else:
                skipped += 1

    print(f"  \u2713 Sprites: {downloaded} downloaded, {skipped} skipped" + " " * 20)


# ── Main ─────────────────────────────────────────────────────────────────────

def main():
    print("\U0001f310 Karta Yaxu \u2014 Extra Pokémon Data Fetcher")
    print(f"   Cache:  {CACHE_DIR}")
    print(f"   Output: {DATA_DIR}")
    print()

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    SPRITE_DIR.mkdir(parents=True, exist_ok=True)
    SHINY_DIR.mkdir(parents=True, exist_ok=True)

    # Read existing IDs to avoid duplicates
    existing_ability_ids = read_existing_ids(DATA_DIR / "ability.toml")
    existing_move_ids = read_existing_ids(DATA_DIR / "move.toml")
    print(f"  Existing abilities: {len(existing_ability_ids)}, moves: {len(existing_move_ids)}")

    all_pokemon = []
    all_pokemon_abilities = []
    all_evolution_links = []
    all_form_entries = []
    all_learnsets = []
    all_ability_ids = set()
    all_move_ids = set()

    # ── Mega Evolutions (Gen 6) ──
    print("\U0001f525 Phase 1A: Fetching Gen 6 Mega Evolutions...")
    for slug, display_name, base_dex, variant, mega_stone in MEGAS_GEN6:
        fid = form_id(OFFSET_MEGA, base_dex, variant)
        result = fetch_form_pokemon(slug, display_name, fid, base_dex, "mega")
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_evolution_links.append((base_dex, fid, "mega", mega_stone))
            all_form_entries.append((base_dex, fid, "Mega"))
    print()

    # ── Mega Evolutions (Z-A) ──
    print("\U0001f525 Phase 1B: Fetching Z-A Mega Evolutions...")
    for slug, display_name, base_dex, variant, mega_stone in MEGAS_ZA:
        fid = form_id(OFFSET_MEGA, base_dex, variant)
        result = fetch_form_pokemon(slug, display_name, fid, base_dex, "mega")
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_evolution_links.append((base_dex, fid, "mega", mega_stone))
            all_form_entries.append((base_dex, fid, "Mega"))
    print()

    # ── Gigantamax Forms ──
    print("\U0001f31f Phase 2: Fetching Gigantamax Forms...")
    for slug, display_name, base_dex, variant in GMAX_FORMS:
        fid = form_id(OFFSET_GMAX, base_dex, variant)
        result = fetch_gmax_as_form(slug, display_name, fid, base_dex)
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_evolution_links.append((base_dex, fid, "gmax", None))
            all_form_entries.append((base_dex, fid, "Gmax"))
    print()

    # ── Alolan Forms ──
    print("\U0001f30a Phase 3: Fetching Alolan Forms...")
    for slug, display_name, base_dex, variant in ALOLAN_FORMS:
        fid = form_id(OFFSET_ALOLAN, base_dex, variant)
        result = fetch_form_pokemon(slug, display_name, fid, base_dex, "alolan")
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_form_entries.append((base_dex, fid, "Alolan"))
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((fid, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Galarian Forms ──
    print("\U0001f3f4 Phase 4: Fetching Galarian Forms...")
    for slug, display_name, base_dex, variant in GALARIAN_FORMS:
        fid = form_id(OFFSET_GALARIAN, base_dex, variant)
        result = fetch_form_pokemon(slug, display_name, fid, base_dex, "galarian")
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_form_entries.append((base_dex, fid, "Galarian"))
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((fid, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Hisuian Forms ──
    print("\U0001f3ef Phase 5: Fetching Hisuian Forms...")
    for slug, display_name, base_dex, variant in HISUIAN_FORMS:
        fid = form_id(OFFSET_HISUIAN, base_dex, variant)
        result = fetch_form_pokemon(slug, display_name, fid, base_dex, "hisuian")
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_form_entries.append((base_dex, fid, "Hisuian"))
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((fid, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Paldean Forms ──
    print("\U0001f30d Phase 6: Fetching Paldean Forms...")
    for slug, display_name, base_dex, variant in PALDEAN_FORMS:
        fid = form_id(OFFSET_PALDEAN, base_dex, variant)
        result = fetch_form_pokemon(slug, display_name, fid, base_dex, "paldean")
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((fid, ab_id, slot))
                all_ability_ids.add(ab_id)
            all_form_entries.append((base_dex, fid, "Paldean"))
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((fid, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Pre-evolutions ──
    print("\U0001f476 Phase 7: Fetching Pre-evolutions...")
    for slug, display_name, national_dex, version_group in PRE_EVOLUTIONS:
        result = fetch_new_species_pokemon(slug, display_name, national_dex, version_group)
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((national_dex, ab_id, slot))
                all_ability_ids.add(ab_id)
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((national_dex, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Cross-gen Evolutions ──
    print("\U0001f504 Phase 8: Fetching Cross-gen Evolutions...")
    for slug, display_name, national_dex, version_group in CROSS_GEN_EVOLUTIONS:
        result = fetch_new_species_pokemon(slug, display_name, national_dex, version_group)
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((national_dex, ab_id, slot))
                all_ability_ids.add(ab_id)
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((national_dex, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Regional Evolutions ──
    print("\U0001f30d Phase 9: Fetching Regional Evolutions...")
    for slug, display_name, national_dex, version_group in REGIONAL_EVOLUTIONS:
        result = fetch_new_species_pokemon(slug, display_name, national_dex, version_group)
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((national_dex, ab_id, slot))
                all_ability_ids.add(ab_id)
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((national_dex, move_id, method, level))
                all_move_ids.add(move_id)
    # Galarian Slowking is also a form of Slowking
    # But it's a regional evolution, so it's listed as national_dex 991
    print()

    # ── Convergent Species ──
    print("\U0001f9a2 Phase 10: Fetching Convergent Species...")
    for slug, display_name, national_dex, version_group in CONVERGENT_SPECIES:
        result = fetch_new_species_pokemon(slug, display_name, national_dex, version_group)
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((national_dex, ab_id, slot))
                all_ability_ids.add(ab_id)
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((national_dex, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Paradox Forms ──
    print("\U0001f52e Phase 11: Fetching Paradox Forms...")
    for slug, display_name, national_dex, version_group in PARADOX_FORMS:
        result = fetch_new_species_pokemon(slug, display_name, national_dex, version_group)
        if result:
            all_pokemon.append(result)
            for ab_id, slot, hidden in result["abilities"]:
                all_pokemon_abilities.append((national_dex, ab_id, slot))
                all_ability_ids.add(ab_id)
            for move_id, method, level in result.get("learnset", []):
                all_learnsets.append((national_dex, move_id, method, level))
                all_move_ids.add(move_id)
    print()

    # ── Add pre-defined evolution links ──
    all_evolution_links.extend(EVOLUTION_LINKS)
    all_evolution_links.extend(REGIONAL_EVOLUTION_LINKS)

    # ── Palette Variant forms (Tauros 3 breeds share same base_dex) ──
    # Already handled by the variant index in PALDEAN_FORMS

    # ── Fetch new abilities ──
    print("\U0001f4a1 Phase 12: Fetching new abilities...")
    new_abilities = fetch_abilities(all_ability_ids, existing_ability_ids)

    # ── Fetch new moves ──
    print(f"\u2694\ufe0f  Phase 13: Fetching new moves...")
    new_moves = fetch_moves(all_move_ids, existing_move_ids)

    # ── Generate TOML ──
    print()
    print("\U0001f4dd Generating TOML seed files...")
    append_pokemon_toml(all_pokemon)
    append_ability_toml(new_abilities)
    append_move_toml(new_moves)
    append_evolution_toml(all_evolution_links)
    append_pokemon_ability_toml(all_pokemon_abilities)
    append_learnset_toml(all_learnsets)
    write_pokemon_form_toml(all_form_entries)

    # ── Download sprites ──
    print()
    print("\U0001f3a8 Phase 14: Downloading sprites...")
    sprite_entries = []
    for p in all_pokemon:
        sprite_entries.append({
            "name": p["name"],
            "sprite": p["sprite"],
            "sprite_url": p.get("sprite_url", ""),
            "sprite_shiny_url": p.get("sprite_shiny_url", ""),
        })
    download_sprites(sprite_entries)

    # ── Summary ──
    print()
    print("=" * 50)
    print("\u2705 Complete!")
    print(f"   Extra Pokémon:       {len(all_pokemon)}")
    print(f"   New abilities:       {len(new_abilities)}")
    print(f"   New moves:           {len(new_moves)}")
    print(f"   Evolution links:     {len(all_evolution_links)}")
    print(f"   Pokemon-ability:     {len(all_pokemon_abilities)}")
    print(f"   Learnset entries:    {len(all_learnsets)}")
    print(f"   Form relationships:  {len(all_form_entries)}")
    print()
    print("\U0001f4cc Next steps:")
    print("   1. Review generated files in data/template/")
    print("   2. Re-seed: cargo run --bin seed")
    print("   3. Verify:  sqlite3 data/rom.db 'SELECT COUNT(*) FROM pokemon'")


if __name__ == "__main__":
    main()