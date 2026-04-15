#!/usr/bin/env python3
"""
fetch_pokeapi.py — Fetch Pokémon Fire Red data from PokeAPI.

Generates all TOML seed files in data/template/ and downloads
classic pixel sprites to static/sprites/.

Usage:
    python3 scripts/fetch_pokeapi.py
"""

import json
import os
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path

# ── Configuration ────────────────────────────────────────────────────────────

POKEAPI_BASE = "https://pokeapi.co/api/v2"
SPRITE_BASE = "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon"
VERSION_GROUP = "firered-leafgreen"
POKEMON_COUNT = 151

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CACHE_DIR = PROJECT_ROOT / "scripts" / ".cache"
DATA_DIR = PROJECT_ROOT / "data" / "template"
SPRITE_DIR = PROJECT_ROOT / "static" / "sprites" / "pokemon"
SHINY_DIR = PROJECT_ROOT / "static" / "sprites" / "shiny"

RATE_LIMIT_S = 0.1  # 100ms between API requests
_last_request = 0.0

# ── Special name handling ────────────────────────────────────────────────────

SPECIAL_NAMES = {
    "nidoran-f": "Nidoran♀",
    "nidoran-m": "Nidoran♂",
    "mr-mime": "Mr. Mime",
    "farfetchd": "Farfetch'd",
}

STAT_ABBREV = {
    "hp": "hp",
    "attack": "atk",
    "defense": "def",
    "special-attack": "spa",
    "special-defense": "spd",
    "speed": "spe",
}

# ── Utility ──────────────────────────────────────────────────────────────────

def api_name_to_display(name: str) -> str:
    """Convert PokeAPI name like 'vine-whip' to 'Vine Whip'."""
    if name in SPECIAL_NAMES:
        return SPECIAL_NAMES[name]
    return name.replace("-", " ").title()


def api_name_to_slug(name: str) -> str:
    """Convert PokeAPI name to filename slug: 'mr-mime' stays 'mr-mime'."""
    return name.lower()


def fetch_json(url: str, cache_subdir: str = "raw") -> dict:
    """Fetch JSON with file caching and rate limiting."""
    global _last_request

    # Cache key from URL
    cache_key = url.replace(POKEAPI_BASE + "/", "").rstrip("/").replace("/", "_")
    cache_path = CACHE_DIR / cache_subdir / f"{cache_key}.json"
    cache_path.parent.mkdir(parents=True, exist_ok=True)

    if cache_path.exists():
        with open(cache_path) as f:
            return json.load(f)

    # Rate limit
    elapsed = time.time() - _last_request
    if elapsed < RATE_LIMIT_S:
        time.sleep(RATE_LIMIT_S - elapsed)

    try:
        req = urllib.request.Request(url, headers={"User-Agent": "KartaYaxu/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  ⚠ HTTP {e.code} for {url}")
        raise
    finally:
        _last_request = time.time()

    with open(cache_path, "w") as f:
        json.dump(data, f)

    return data


def download_file(url: str, dest: Path) -> bool:
    """Download a binary file. Returns True if downloaded, False if skipped."""
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
        print(f"  ⚠ Failed to download {url}: {e}")
        _last_request = time.time()
        return False


def escape_toml_string(s: str) -> str:
    """Escape a string for TOML double-quoted value."""
    s = s.replace("\\", "\\\\")
    s = s.replace('"', '\\"')
    s = s.replace("\n", " ")
    s = s.replace("\r", "")
    s = s.replace("\f", " ")
    # Collapse multiple spaces
    while "  " in s:
        s = s.replace("  ", " ")
    return s.strip()


def get_english_text(entries: list, key: str = "flavor_text", version_group: str = None) -> str:
    """Get English text from flavor_text_entries or effect_entries."""
    # Try version-group-specific first
    if version_group:
        for entry in entries:
            lang = entry.get("language", {}).get("name", "")
            vg = entry.get("version_group", {})
            if vg and isinstance(vg, dict):
                vg_name = vg.get("name", "")
            else:
                vg_name = ""
            if lang == "en" and vg_name == version_group:
                return escape_toml_string(entry.get(key, ""))

    # Fallback: latest English entry
    for entry in reversed(entries):
        if entry.get("language", {}).get("name") == "en":
            return escape_toml_string(entry.get(key, entry.get("short_effect", "")))

    return ""


# ── Phase 1: Fetch Pokémon ───────────────────────────────────────────────────

def fetch_all_pokemon():
    """Fetch all 151 Pokemon + species data."""
    pokemon_list = []
    all_ability_ids = set()
    all_move_ids = set()
    pokemon_abilities = []  # (pokemon_id, ability_id, slot)
    pokemon_learnsets = []  # (pokemon_id, move_id, method, level)

    for poke_id in range(1, POKEMON_COUNT + 1):
        print(f"  [{poke_id}/{POKEMON_COUNT}] Fetching Pokémon #{poke_id}...", end="\r")

        poke_data = fetch_json(f"{POKEAPI_BASE}/pokemon/{poke_id}")
        species_data = fetch_json(f"{POKEAPI_BASE}/pokemon-species/{poke_id}")

        # Name
        api_name = poke_data["name"]
        display_name = api_name_to_display(api_name)
        slug = api_name_to_slug(api_name)

        # Types
        types = sorted(poke_data["types"], key=lambda t: t["slot"])
        type1_id = types[0]["type"]["url"].rstrip("/").split("/")[-1]
        type2_id = types[1]["type"]["url"].rstrip("/").split("/")[-1] if len(types) > 1 else None

        # Stats
        stats = {}
        for s in poke_data["stats"]:
            stat_name = s["stat"]["name"]
            stats[STAT_ABBREV.get(stat_name, stat_name)] = s["base_stat"]

        # Pokedex entry (English, Fire Red preferred)
        pokedex_entry = ""
        for entry in species_data.get("flavor_text_entries", []):
            if entry.get("language", {}).get("name") == "en":
                ver = entry.get("version", {}).get("name", "")
                text = escape_toml_string(entry.get("flavor_text", ""))
                if ver == "firered":
                    pokedex_entry = text
                    break
                if not pokedex_entry:
                    pokedex_entry = text

        # Sprites
        sprite_file = f"{slug}.png"
        shiny_file = f"{slug}_shiny.png"

        pokemon_list.append({
            "id": poke_id,
            "dex_number": poke_id,
            "name": display_name,
            "sprite": sprite_file,
            "sprite_shiny": shiny_file,
            "type1_id": int(type1_id),
            "type2_id": int(type2_id) if type2_id else None,
            "hp": stats.get("hp", 0),
            "atk": stats.get("atk", 0),
            "def": stats.get("def", 0),
            "spa": stats.get("spa", 0),
            "spd": stats.get("spd", 0),
            "spe": stats.get("spe", 0),
            "pokedex_entry": pokedex_entry,
            "_api_name": api_name,
        })

        # Abilities
        for ab in poke_data["abilities"]:
            ab_id = int(ab["ability"]["url"].rstrip("/").split("/")[-1])
            slot = ab["slot"]
            all_ability_ids.add(ab_id)
            pokemon_abilities.append((poke_id, ab_id, slot))

        # Moves (filtered to Fire Red)
        for move_entry in poke_data["moves"]:
            move_id = int(move_entry["move"]["url"].rstrip("/").split("/")[-1])
            for vgd in move_entry["version_group_details"]:
                if vgd["version_group"]["name"] == VERSION_GROUP:
                    method_name = vgd["move_learn_method"]["name"]
                    level = vgd["level_learned_at"]

                    # Map method names
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

                    all_move_ids.add(move_id)
                    pokemon_learnsets.append((poke_id, move_id, method, level if level > 0 else None))

        # Evolution chain ID (for later)
        evo_chain_url = species_data.get("evolution_chain", {}).get("url")
        if evo_chain_url:
            pokemon_list[-1]["_evo_chain_url"] = evo_chain_url

    print(f"  ✓ Fetched {POKEMON_COUNT} Pokémon" + " " * 30)

    return pokemon_list, all_ability_ids, all_move_ids, pokemon_abilities, pokemon_learnsets


# ── Phase 2: Fetch Abilities ─────────────────────────────────────────────────

def fetch_abilities(ability_ids: set):
    """Fetch ability details."""
    abilities = []
    sorted_ids = sorted(ability_ids)
    total = len(sorted_ids)

    for i, ab_id in enumerate(sorted_ids):
        print(f"  [{i+1}/{total}] Fetching ability #{ab_id}...", end="\r")
        data = fetch_json(f"{POKEAPI_BASE}/ability/{ab_id}")

        # Get English short effect
        desc = ""
        for entry in data.get("effect_entries", []):
            if entry.get("language", {}).get("name") == "en":
                desc = escape_toml_string(entry.get("short_effect", entry.get("effect", "")))
                break

        if not desc:
            for entry in data.get("flavor_text_entries", []):
                if entry.get("language", {}).get("name") == "en":
                    desc = escape_toml_string(entry.get("flavor_text", ""))
                    break

        abilities.append({
            "id": ab_id,
            "name": api_name_to_display(data["name"]),
            "description": desc,
        })

    print(f"  ✓ Fetched {total} abilities" + " " * 30)
    return abilities


# ── Phase 3: Fetch Moves ─────────────────────────────────────────────────────

def fetch_moves(move_ids: set):
    """Fetch move details."""
    moves = []
    sorted_ids = sorted(move_ids)
    total = len(sorted_ids)

    for i, move_id in enumerate(sorted_ids):
        print(f"  [{i+1}/{total}] Fetching move #{move_id}...", end="\r")
        data = fetch_json(f"{POKEAPI_BASE}/move/{move_id}")

        type_id = int(data["type"]["url"].rstrip("/").split("/")[-1])
        category = data.get("damage_class", {}).get("name", "status")
        power = data.get("power")
        accuracy = data.get("accuracy")
        pp = data.get("pp", 0)

        # Get description — try flavor text for FR, fallback to effect
        desc = ""
        for entry in data.get("flavor_text_entries", []):
            lang = entry.get("language", {}).get("name", "")
            vg = entry.get("version_group", {}).get("name", "")
            if lang == "en" and vg == VERSION_GROUP:
                desc = escape_toml_string(entry.get("flavor_text", ""))
                break

        if not desc:
            for entry in reversed(data.get("flavor_text_entries", [])):
                if entry.get("language", {}).get("name") == "en":
                    desc = escape_toml_string(entry.get("flavor_text", ""))
                    break

        if not desc:
            for entry in data.get("effect_entries", []):
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
            "pp": pp or 5,
            "description": desc,
        })

    print(f"  ✓ Fetched {total} moves" + " " * 30)
    return moves


# ── Phase 4: Fetch Natures ───────────────────────────────────────────────────

def fetch_natures():
    """Fetch all 25 natures."""
    natures = []
    for nat_id in range(1, 26):
        print(f"  [{nat_id}/25] Fetching nature #{nat_id}...", end="\r")
        data = fetch_json(f"{POKEAPI_BASE}/nature/{nat_id}")

        inc = data.get("increased_stat")
        dec = data.get("decreased_stat")
        inc_name = STAT_ABBREV.get(inc["name"]) if inc else None
        dec_name = STAT_ABBREV.get(dec["name"]) if dec else None

        natures.append({
            "id": nat_id,
            "name": api_name_to_display(data["name"]),
            "increased_stat": inc_name,
            "decreased_stat": dec_name,
        })

    print(f"  ✓ Fetched 25 natures" + " " * 30)
    return natures


# ── Phase 5: Fetch Evolution Chains ──────────────────────────────────────────

def fetch_evolutions(pokemon_list: list):
    """Parse evolution chains for the 151 Pokémon."""
    evolutions = []
    seen_chains = set()

    # Build name → id map
    name_to_id = {}
    for p in pokemon_list:
        name_to_id[p["_api_name"]] = p["id"]

    for p in pokemon_list:
        chain_url = p.get("_evo_chain_url")
        if not chain_url or chain_url in seen_chains:
            continue
        seen_chains.add(chain_url)

        chain_id = chain_url.rstrip("/").split("/")[-1]
        print(f"  Fetching evolution chain #{chain_id}...", end="\r")

        try:
            data = fetch_json(chain_url)
        except Exception:
            continue

        _walk_chain(data["chain"], name_to_id, evolutions)

    print(f"  ✓ Found {len(evolutions)} evolution links" + " " * 30)
    return evolutions


def _walk_chain(node: dict, name_to_id: dict, results: list):
    """Recursively walk an evolution chain node."""
    from_name = node["species"]["name"]
    from_id = name_to_id.get(from_name)

    for evo in node.get("evolves_to", []):
        to_name = evo["species"]["name"]
        to_id = name_to_id.get(to_name)

        if from_id is None or to_id is None:
            # One of the Pokemon is outside our 151 range
            _walk_chain(evo, name_to_id, results)
            continue

        # Parse evolution details
        details = evo.get("evolution_details", [{}])
        detail = details[0] if details else {}

        trigger = detail.get("trigger", {}).get("name", "level-up")
        condition = "level"
        condition_value = None

        if trigger == "level-up":
            min_level = detail.get("min_level")
            min_happiness = detail.get("min_happiness")
            if min_level:
                condition = "level"
                condition_value = str(min_level)
            elif min_happiness:
                condition = "happiness"
                condition_value = str(min_happiness)
            else:
                condition = "level"
                condition_value = None
        elif trigger == "trade":
            held_item = detail.get("held_item")
            if held_item:
                condition = "trade"
                condition_value = api_name_to_display(held_item["name"])
            else:
                condition = "trade"
        elif trigger == "use-item":
            item = detail.get("item", {})
            condition = "item"
            condition_value = api_name_to_display(item.get("name", "unknown"))
        else:
            condition = trigger

        results.append({
            "from_id": from_id,
            "to_id": to_id,
            "condition": condition,
            "condition_value": condition_value,
        })

        _walk_chain(evo, name_to_id, results)


# ── Phase 6: Download Sprites ────────────────────────────────────────────────

def download_sprites(pokemon_list: list):
    """Download classic pixel sprites for all Pokémon."""
    downloaded = 0
    skipped = 0
    total = len(pokemon_list) * 2

    for i, p in enumerate(pokemon_list):
        poke_id = p["id"]
        slug = p["_api_name"]

        # Normal sprite
        normal_url = f"{SPRITE_BASE}/{poke_id}.png"
        normal_dest = SPRITE_DIR / f"{slug}.png"
        print(f"  [{i*2+1}/{total}] Sprites for {p['name']}...", end="\r")

        if download_file(normal_url, normal_dest):
            downloaded += 1
        else:
            skipped += 1

        # Shiny sprite
        shiny_url = f"{SPRITE_BASE}/shiny/{poke_id}.png"
        shiny_dest = SHINY_DIR / f"{slug}_shiny.png"

        if download_file(shiny_url, shiny_dest):
            downloaded += 1
        else:
            skipped += 1

    print(f"  ✓ Sprites: {downloaded} downloaded, {skipped} skipped (cached)" + " " * 20)


# ── TOML Generation ─────────────────────────────────────────────────────────

def write_pokemon_toml(pokemon_list):
    """Generate pokemon.toml for Gen 1 Pokémon."""
    lines = []
    for p in pokemon_list:
        lines.append("[[pokemon]]")
        lines.append(f'id = {p["id"]}')
        lines.append(f'dex_number = {p["dex_number"]}')
        lines.append(f'name = "{p["name"]}"')
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
        lines.append(f'pokedex_entry = "{p["pokedex_entry"]}"')
        lines.append("")
    (DATA_DIR / "pokemon.toml").write_text("\n".join(lines))
    print(f"  pokemon.toml ({len(pokemon_list)} entries)")


def write_ability_toml(abilities):
    lines = []
    for a in abilities:
        lines.append("[[ability]]")
        lines.append(f'id = {a["id"]}')
        lines.append(f'name = "{a["name"]}"')
        lines.append(f'description = "{a["description"]}"')
        lines.append("")
    (DATA_DIR / "ability.toml").write_text("\n".join(lines))
    print(f"  ability.toml ({len(abilities)} entries)")


def write_move_toml(moves):
    lines = []
    for m in moves:
        lines.append("[[move]]")
        lines.append(f'id = {m["id"]}')
        lines.append(f'name = "{m["name"]}"')
        lines.append(f'type_id = {m["type_id"]}')
        lines.append(f'category = "{m["category"]}"')
        if m["power"] is not None:
            lines.append(f'power = {m["power"]}')
        if m["accuracy"] is not None:
            lines.append(f'accuracy = {m["accuracy"]}')
        lines.append(f'pp = {m["pp"]}')
        lines.append(f'description = "{m["description"]}"')
        lines.append("")
    (DATA_DIR / "move.toml").write_text("\n".join(lines))
    print(f"  move.toml ({len(moves)} entries)")


def write_nature_toml(natures):
    lines = []
    for n in natures:
        lines.append("[[nature]]")
        lines.append(f'id = {n["id"]}')
        lines.append(f'name = "{n["name"]}"')
        if n["increased_stat"]:
            lines.append(f'increased_stat = "{n["increased_stat"]}"')
        if n["decreased_stat"]:
            lines.append(f'decreased_stat = "{n["decreased_stat"]}"')
        lines.append("")
    (DATA_DIR / "nature.toml").write_text("\n".join(lines))
    print(f"  nature.toml ({len(natures)} entries)")


def write_evolution_toml(evolutions):
    lines = []
    for e in evolutions:
        lines.append("[[evolution]]")
        lines.append(f'from_id = {e["from_id"]}')
        lines.append(f'to_id = {e["to_id"]}')
        lines.append(f'condition = "{e["condition"]}"')
        if e["condition_value"]:
            lines.append(f'condition_value = "{e["condition_value"]}"')
        lines.append("")
    (DATA_DIR / "evolution.toml").write_text("\n".join(lines))
    print(f"  evolution.toml ({len(evolutions)} entries)")


def write_learnset_toml(learnsets):
    lines = []
    for pokemon_id, move_id, method, level in learnsets:
        lines.append("[[learnset]]")
        lines.append(f"pokemon_id = {pokemon_id}")
        lines.append(f"move_id = {move_id}")
        lines.append(f'method = "{method}"')
        if level is not None:
            lines.append(f"level = {level}")
        lines.append("")
    (DATA_DIR / "learnset.toml").write_text("\n".join(lines))
    print(f"  learnset.toml ({len(learnsets)} entries)")


def write_pokemon_ability_toml(pokemon_abilities):
    lines = []
    for pokemon_id, ability_id, slot in pokemon_abilities:
        lines.append("[[pokemon_ability]]")
        lines.append(f"pokemon_id = {pokemon_id}")
        lines.append(f"ability_id = {ability_id}")
        lines.append(f"slot = {slot}")
        lines.append("")
    (DATA_DIR / "pokemon_ability.toml").write_text("\n".join(lines))
    print(f"  pokemon_ability.toml ({len(pokemon_abilities)} entries)")


def write_pokemon_form_toml():
    """Write empty pokemon_form.toml — populated by fetch_extras.py."""
    (DATA_DIR / "pokemon_form.toml").write_text("pokemon_form = []")
    print("  pokemon_form.toml (0 entries — populated by fetch_extras.py)")


def write_status_condition_toml():
    """Write the 6 standard non-volatile status conditions."""
    status_content = (
        "[[status_condition]]\n"
        'id = 1\n'
        'name = "Burn"\n'
        'description = "Deals 1/8 max HP damage each turn and halves physical attack damage."\n'
        'icon = "brn.png"\n'
        "is_volatile = false\n"
        "\n"
        "[[status_condition]]\n"
        "id = 2\n"
        'name = "Freeze"\n'
        'description = "The Pokémon is frozen solid and cannot move. There is a 20% chance of thawing each turn."\n'
        'icon = "frz.png"\n'
        "is_volatile = false\n"
        "\n"
        "[[status_condition]]\n"
        "id = 3\n"
        'name = "Paralysis"\n'
        'description = "Speed is reduced by 50% and each turn there is a 25% chance the Pokémon cannot move."\n'
        'icon = "par.png"\n'
        "is_volatile = false\n"
        "\n"
        "[[status_condition]]\n"
        "id = 4\n"
        'name = "Poison"\n'
        'description = "Deals 1/8 max HP damage at the end of each turn."\n'
        'icon = "psn.png"\n'
        "is_volatile = false\n"
        "\n"
        "[[status_condition]]\n"
        "id = 5\n"
        'name = "Bad Poison"\n'
        'description = "Damage increases each turn: 1/16, 2/16, 3/16... of max HP."\n'
        'icon = "tox.png"\n'
        "is_volatile = false\n"
        "\n"
        "[[status_condition]]\n"
        "id = 6\n"
        'name = "Sleep"\n'
        'description = "The Pokémon cannot move for 1-3 turns."\n'
        'icon = "slp.png"\n'
        "is_volatile = false\n"
    )
    (DATA_DIR / "status_condition.toml").write_text(status_content)
    print("  status_condition.toml (6 entries)")

def main():
    print("🌐 Karta Yaxu — PokeAPI Data Fetcher")
    print(f"   Target: Pokémon Fire Red (Gen III, {POKEMON_COUNT} Pokémon)")
    print(f"   Cache:  {CACHE_DIR}")
    print(f"   Output: {DATA_DIR}")
    print()

    # Ensure directories exist
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    SPRITE_DIR.mkdir(parents=True, exist_ok=True)
    SHINY_DIR.mkdir(parents=True, exist_ok=True)

    # Phase 1: Pokémon
    print("📦 Phase 1: Fetching Pokémon data...")
    pokemon_list, ability_ids, move_ids, poke_abilities, learnsets = fetch_all_pokemon()

    # Phase 2: Abilities
    print(f"💡 Phase 2: Fetching {len(ability_ids)} abilities...")
    abilities = fetch_abilities(ability_ids)

    # Phase 3: Moves
    print(f"⚔️  Phase 3: Fetching {len(move_ids)} moves...")
    moves = fetch_moves(move_ids)

    # Phase 4: Natures
    print("🌿 Phase 4: Fetching natures...")
    natures = fetch_natures()

    # Phase 5: Evolutions
    print("🔗 Phase 5: Fetching evolution chains...")
    evolutions = fetch_evolutions(pokemon_list)

    # Phase 6: Sprites
    print("🎨 Phase 6: Downloading sprites...")
    download_sprites(pokemon_list)

    # Phase 7: Generate TOML
    print()
    print("📝 Generating TOML seed files...")
    write_pokemon_toml(pokemon_list)
    write_ability_toml(abilities)
    write_move_toml(moves)
    write_nature_toml(natures)
    write_evolution_toml(evolutions)
    write_learnset_toml(learnsets)
    write_pokemon_ability_toml(poke_abilities)
    write_pokemon_form_toml()
    write_status_condition_toml()

    # Summary
    print()
    print("═" * 50)
    print("✅ Complete!")
    print(f"   Pokémon:     {len(pokemon_list)}")
    print(f"   Abilities:   {len(abilities)}")
    print(f"   Moves:       {len(moves)}")
    print(f"   Natures:     {len(natures)}")
    print(f"   Evolutions:  {len(evolutions)}")
    print(f"   Learnsets:   {len(learnsets)}")
    print()
    print("📌 Next steps:")
    print("   1. Review generated files in data/template/")
    print("   2. Re-seed: cargo run --bin seed")
    print("   3. Verify:  sqlite3 data/rom.db 'SELECT COUNT(*) FROM pokemon'")


if __name__ == "__main__":
    main()
