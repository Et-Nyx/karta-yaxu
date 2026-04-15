#!/usr/bin/env python3
"""
fetch_types.py — Fetch type and type effectiveness data from PokeAPI.

Fetches all 19 standard types from PokeAPI and updates:
  - data/template/type.toml       (adds Stellar type, id=19)
  - data/template/type_effectiveness.toml  (adds non-1.0 interactions only)

Existing entries are preserved exactly; only new non-1.0 interactions are merged.
If Stellar has no non-1.0 interactions (which is the case per PokeAPI), no new
type_effectiveness entries are added.

Usage:
  python3 scripts/fetch_types.py
"""

import json
import re
import time
from pathlib import Path

POKEAPI_BASE = "https://pokeapi.co/api/v2"
PROJECT_ROOT = Path(__file__).resolve().parent.parent
CACHE_DIR = PROJECT_ROOT / "scripts" / ".cache"
DATA_DIR = PROJECT_ROOT / "data" / "template"

RATE_LIMIT_S = 0.1
_last_request = 0.0

STELLAR_HEXCOLOR = "#83cfc5"
STELLAR_ICON = "stellar.png"


def fetch_json(url: str, cache_subdir: str = "raw") -> dict:
    global _last_request
    cache_key = url.replace(POKEAPI_BASE + "/", "").rstrip("/").replace("/", "_")
    cache_path = CACHE_DIR / cache_subdir / f"{cache_key}.json"
    cache_path.parent.mkdir(parents=True, exist_ok=True)

    if cache_path.exists():
        with open(cache_path) as f:
            return json.load(f)

    now = time.time()
    wait = RATE_LIMIT_S - (now - _last_request)
    if wait > 0:
        time.sleep(wait)
    _last_request = time.time()

    try:
        req = urllib.request.Request(url, headers={"User-Agent": "KartaYaxu/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  ⚠ HTTP {e.code} for {url}")
        raise

    with open(cache_path, "w") as f:
        json.dump(data, f)
    return data


import urllib.request
import urllib.error


def fetch_type_list() -> list[dict]:
    data = fetch_json(f"{POKEAPI_BASE}/type?limit=100")
    types = []
    for t in data["results"]:
        m = re.search(r"/(\d+)/$", t["url"])
        if m:
            type_id = int(m.group(1))
            if type_id <= 19:
                types.append({"name": t["name"], "id": type_id})
    types.sort(key=lambda x: x["id"])
    return types


def fetch_type_data(type_id: int) -> dict:
    return fetch_json(f"{POKEAPI_BASE}/type/{type_id}/")


def extract_type_id_from_url(url: str) -> int:
    m = re.search(r"/(\d+)/$", url)
    return int(m.group(1)) if m else 0


def build_effectiveness_map(type_ids: list[int]) -> dict[tuple, float]:
    eff = {}
    for type_id in type_ids:
        data = fetch_type_data(type_id)
        dr = data.get("damage_relations", {})

        for target in dr.get("double_damage_to", []):
            tid = extract_type_id_from_url(target["url"])
            if tid:
                eff[(type_id, tid)] = 2.0

        for target in dr.get("half_damage_to", []):
            tid = extract_type_id_from_url(target["url"])
            if tid:
                eff[(type_id, tid)] = 0.5

        for target in dr.get("no_damage_to", []):
            tid = extract_type_id_from_url(target["url"])
            if tid:
                eff[(type_id, tid)] = 0.0

    return eff


def parse_type_toml(path: Path) -> list[dict]:
    text = path.read_text()
    entries = []
    for block in text.split("[[type]]")[1:]:
        entry = {}
        for line in block.strip().split("\n"):
            line = line.strip()
            if line.startswith("id ="):
                entry["id"] = int(line.split("=")[1].strip())
            elif line.startswith("name ="):
                entry["name"] = line.split("=", 1)[1].strip().strip('"')
            elif line.startswith("hexcolor ="):
                entry["hexcolor"] = line.split("=", 1)[1].strip().strip('"')
            elif line.startswith("icon ="):
                entry["icon"] = line.split("=", 1)[1].strip().strip('"')
        if entry:
            entries.append(entry)
    return entries


def write_type_toml(path: Path, entries: list[dict]):
    lines = []
    for e in entries:
        lines.append("[[type]]")
        lines.append(f'id = {e["id"]}')
        lines.append(f'name = "{e["name"]}"')
        lines.append(f'hexcolor = "{e["hexcolor"]}"')
        lines.append(f'icon = "{e["icon"]}"')
        lines.append("")
    path.write_text("\n".join(lines).rstrip() + "\n")


def parse_effectiveness_toml(path: Path) -> set[tuple]:
    text = path.read_text()
    entries = set()
    for block in text.split("[[type_effectiveness]]")[1:]:
        attacker_id = 0
        defender_id = 0
        multiplier = 1.0
        for line in block.strip().split("\n"):
            line = line.strip()
            if line.startswith("attacker_id ="):
                attacker_id = int(line.split("=")[1].strip())
            elif line.startswith("defender_id ="):
                defender_id = int(line.split("=")[1].strip())
            elif line.startswith("multiplier ="):
                multiplier = float(line.split("=")[1].strip())
        if attacker_id and defender_id:
            entries.add((attacker_id, defender_id, multiplier))
    return entries


def write_effectiveness_toml(path: Path, entries: list[tuple]):
    type_names = {
        1: "Normal", 2: "Fighting", 3: "Flying", 4: "Poison", 5: "Ground",
        6: "Rock", 7: "Bug", 8: "Ghost", 9: "Steel", 10: "Fire",
        11: "Water", 12: "Grass", 13: "Electric", 14: "Psychic", 15: "Ice",
        16: "Dragon", 17: "Dark", 18: "Fairy", 19: "Stellar",
    }
    lines = []
    for attacker_id, defender_id, multiplier in sorted(entries):
        aname = type_names.get(attacker_id, str(attacker_id))
        dname = type_names.get(defender_id, str(defender_id))
        lines.append(f"# {aname} -> {dname}")
        lines.append("[[type_effectiveness]]")
        lines.append(f"attacker_id = {attacker_id}")
        lines.append(f"defender_id = {defender_id}")
        lines.append(f"multiplier = {multiplier}")
        lines.append("")
    path.write_text("\n".join(lines).rstrip() + "\n")


def main():
    print("Fetching type list from PokeAPI...")
    types = fetch_type_list()
    type_ids = [t["id"] for t in types]
    print(f"Found {len(types)} types: {[t['name'] for t in types]}")

    print("Loading existing type.toml...")
    type_path = DATA_DIR / "type.toml"
    existing_types = parse_type_toml(type_path)
    print(f"  Existing entries: {len(existing_types)}")

    new_type_ids = {t["id"] for t in existing_types}
    for t in types:
        if t["id"] not in new_type_ids:
            existing_types.append({
                "id": t["id"],
                "name": t["name"].title(),
                "hexcolor": STELLAR_HEXCOLOR,
                "icon": STELLAR_ICON,
            })
            print(f"  Added type id={t['id']} name='{t['name']}'")

    existing_types.sort(key=lambda x: x["id"])
    write_type_toml(type_path, existing_types)
    print(f"Wrote {len(existing_types)} types to type.toml")

    print("Fetching type effectiveness from PokeAPI...")
    eff_map = build_effectiveness_map(type_ids)
    print(f"  Non-1.0 interactions found: {len(eff_map)}")

    print("Loading existing type_effectiveness.toml...")
    eff_path = DATA_DIR / "type_effectiveness.toml"
    existing_eff = parse_effectiveness_toml(eff_path)
    print(f"  Existing entries: {len(existing_eff)}")

    new_entries = set()
    for (a, d), m in eff_map.items():
        new_entries.add((a, d, m))

    merged = existing_eff | new_entries
    added = new_entries - existing_eff
    print(f"  New entries to add: {len(added)}")
    if added:
        for a, d, m in sorted(added):
            print(f"    Adding: {a} -> {d} = {m}")

    eff_list = sorted(merged, key=lambda x: (x[0], x[1]))
    write_effectiveness_toml(eff_path, eff_list)
    print(f"Wrote {len(eff_list)} entries to type_effectiveness.toml")

    print("\nDone! Run fetch_pokeapi.py next, then fetch_extras.py.")


if __name__ == "__main__":
    main()