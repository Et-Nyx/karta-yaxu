import { db } from '$lib/db';
import { asc, desc, count, eq, isNotNull } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import {
  importAllGameData,
  importAllLanguages,
  importAllRegions,
  importAllGenerations,
  importAllVersionGroups,
  importAllVersions,
  importAllNatures,
  importAllEggGroups,
  importAllMoveLearnMethods,
  importAllEncounterMethods,
  importAllPokedexes,
  ensurePokedex,
  importPokedexesByVersionGroup,
} from '$lib/pokeapi/import-service';
import {
  importAllTypes,
  importAllAbilities,
  importAllItems,
  importAllMoves,
  importPokemonById,
  importPokemonByRange,
  ensureType,
  ensureAbility,
  ensureMove,
  ensureItem,
  importTypesByRegion,
  importAbilitiesByRegion,
  importMovesByRegion,
  importPokemonByRegion,
} from '$lib/pokeapi/import-pokemon';
import {
  importAllLocations,
  importLocationsByRegion,
} from '$lib/pokeapi/import-locations';
import type { Actions, PageServerLoad } from './$types';
import { requireEditor } from '$lib/auth/guards';

type CountRow = { source: string; n: number };

async function countsBySource(table: any): Promise<{ total: number; pokeapi: number; edited: number; custom: number }> {
  try {
    if (table.source) {
      const rows = await db.select({ source: table.source, n: count() })
        .from(table)
        .groupBy(table.source) as CountRow[];

      const out = { total: 0, pokeapi: 0, edited: 0, custom: 0 };
      for (const r of rows) {
        out.total += r.n;
        if (r.source === 'pokeapi') out.pokeapi = r.n;
        else if (r.source === 'edited') out.edited = r.n;
        else if (r.source === 'custom') out.custom = r.n;
      }
      return out;
    } else {
      // Table does not have a "source" column (e.g. eggGroups, moveLearnMethods, encounterMethods)
      // Since they are strictly metadata, all records are implicitly 'pokeapi' source
      const [res] = await db.select({ n: count() }).from(table);
      const total = res?.n || 0;
      return { total, pokeapi: total, edited: 0, custom: 0 };
    }
  } catch (e: any) {
    console.error("countsBySource error:", e);
    return { total: 0, pokeapi: 0, edited: 0, custom: 0 };
  }
}

function parseId(raw: FormDataEntryValue | null): number | null {
  const n = parseInt(raw as string);
  return isNaN(n) || n <= 0 ? null : n;
}

async function checkPokedexPreconditions(): Promise<string | null> {
  const [{ c }] = await db.select({ c: count() }).from(schema.pokemonSpecies);
  if (c === 0) return 'No Pokémon imported yet — import Pokémon before Pokédexes.';
  return null;
}

export const load: PageServerLoad = async ({ locals }) => {
  requireEditor(locals);

  const [
    languagesCounts,
    regionsCounts,
    generationsCounts,
    versionGroupsCounts,
    versionsCounts,
    naturesCounts,
    eggGroupsCounts,
    moveLearnMethodsCounts,
    encounterMethodsCounts,
    typesCounts,
    abilitiesCounts,
    movesCounts,
    itemsCounts,
    pokemonCounts,
    speciesCounts,
    pokedexesCounts,
    locationsCounts,
  ] = await Promise.all([
    countsBySource(schema.languages),
    countsBySource(schema.regions),
    countsBySource(schema.generations),
    countsBySource(schema.versionGroups),
    countsBySource(schema.versions),
    countsBySource(schema.natures),
    countsBySource(schema.eggGroups),
    countsBySource(schema.moveLearnMethods),
    countsBySource(schema.encounterMethods),
    countsBySource(schema.types),
    countsBySource(schema.abilities),
    countsBySource(schema.moves),
    countsBySource(schema.items),
    countsBySource(schema.pokemon),
    countsBySource(schema.pokemonSpecies),
    countsBySource(schema.pokedexes),
    countsBySource(schema.locations),
  ]);

  const recentEdits = await db.select({
    id:        schema.edits.id,
    tableName: schema.edits.tableName,
    recordId:  schema.edits.recordId,
    editedAt:  schema.edits.editedAt,
    username:  schema.trainers.username,
  })
    .from(schema.edits)
    .leftJoin(schema.trainers, eq(schema.edits.trainerId, schema.trainers.id))
    .orderBy(desc(schema.edits.editedAt))
    .limit(15);

  const logs = await db.select()
    .from(schema.importLog)
    .orderBy(desc(schema.importLog.timestamp))
    .limit(50);

  const regions = await db
    .select({
      id: schema.regions.id,
      pokeapiId: schema.regions.pokeapiId,
      name: schema.regions.name,
      slug: schema.regions.slug,
    })
    .from(schema.regions)
    .orderBy(asc(schema.regions.name));

  const versionGroups = await db
    .select({
      id: schema.versionGroups.id,
      pokeapiId: schema.versionGroups.pokeapiId,
      name: schema.versionGroups.name,
      order: schema.versionGroups.order,
    })
    .from(schema.versionGroups)
    .where(isNotNull(schema.versionGroups.pokeapiId))
    .orderBy(asc(schema.versionGroups.id)) as Array<{ id: number; pokeapiId: number; name: string; order: number | null }>;

  return {
    entities: [
      { key: 'languages',         label: 'Languages',          counts: languagesCounts,         ready: false },
      { key: 'regions',           label: 'Regions',            counts: regionsCounts,           ready: false },
      { key: 'generations',       label: 'Generations',        counts: generationsCounts,       ready: false },
      { key: 'versionGroups',     label: 'Version Groups',     counts: versionGroupsCounts,     ready: false },
      { key: 'versions',          label: 'Versions',           counts: versionsCounts,          ready: false },
      { key: 'eggGroups',         label: 'Egg Groups',         counts: eggGroupsCounts,         ready: false },
      { key: 'moveLearnMethods',  label: 'Move Learn Methods',  counts: moveLearnMethodsCounts,  ready: false },
      { key: 'encounterMethods', label: 'Encounter Methods',  counts: encounterMethodsCounts, ready: false },
      
      { key: 'types',             label: 'Types',              href: '/editor/types',     counts: typesCounts,     ready: true },
      { key: 'natures',           label: 'Natures',            href: '/editor/natures',   counts: naturesCounts,   ready: true },
      { key: 'abilities',         label: 'Abilities',          href: '/editor/abilities', counts: abilitiesCounts, ready: true },
      { key: 'items',             label: 'Items',              href: '/editor/items',     counts: itemsCounts,     ready: true },
      { key: 'pokedexes',         label: 'Pokédexes',          href: '/editor/pokedex',   counts: pokedexesCounts, ready: true },
      { key: 'moves',             label: 'Moves',              href: '/editor/moves',     counts: movesCounts,     ready: true },
      { key: 'pokemon',           label: 'Pokémon',            href: '/editor/pokemon',   counts: pokemonCounts,   ready: true },
      { key: 'species',           label: 'Species',            href: '/editor/species',   counts: speciesCounts,   ready: true },
      { key: 'locations',         label: 'Locations',          href: '/editor/locations', counts: locationsCounts, ready: true },
    ],
    recentEdits,
    logs,
    regions,
    versionGroups
  };
};

export const actions: Actions = {
  // ── Game Data ──────────────────────────────────────────────────────────
  importGameData: async ({ locals }) => {
    requireEditor(locals);
    try {
      await importAllGameData();
      return { success: true, message: 'Game data imported (Languages, Regions, Generations, Versions)' };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importLanguages: async () => {
    try {
      const res = await importAllLanguages();
      return { success: true, message: `Imported ${res.inserted} languages` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importRegions: async () => {
    try {
      const res = await importAllRegions();
      return { success: true, message: `Imported ${res.inserted} regions` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importGenerations: async () => {
    try {
      const res = await importAllGenerations();
      return { success: true, message: `Imported ${res.inserted} generations` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importVersionGroups: async () => {
    try {
      const res = await importAllVersionGroups();
      return { success: true, message: `Imported ${res.inserted} version groups` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importVersions: async () => {
    try {
      const res = await importAllVersions();
      return { success: true, message: `Imported ${res.inserted} versions` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importNatures: async () => {
    try {
      const res = await importAllNatures();
      return { success: true, message: `Imported ${res.inserted} natures` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importEggGroups: async () => {
    try {
      const res = await importAllEggGroups();
      return { success: true, message: `Imported ${res.inserted} egg groups` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importMoveLearnMethods: async () => {
    try {
      const res = await importAllMoveLearnMethods();
      return { success: true, message: `Imported ${res.inserted} move learn methods` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },
  importEncounterMethods: async () => {
    try {
      const res = await importAllEncounterMethods();
      return { success: true, message: `Imported ${res.inserted} encounter methods` };
    } catch (e: any) { return { success: false, error: e.message }; }
  },

  // ── Types ──────────────────────────────────────────────────────────────
  importTypes: async () => {
    try {
      const res = await importAllTypes();
      return { success: true, message: `Imported ${res.inserted} types` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSingleType: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('pokeapiId'));
    if (!id) return { success: false, error: 'Invalid ID' };
    try {
      await ensureType(id);
      return { success: true, message: `Type #${id} imported` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importRangeTypes: async ({ request }) => {
    const fd = await request.formData();
    const from = parseId(fd.get('fromId'));
    const to = parseId(fd.get('toId'));
    if (!from || !to || from > to) return { success: false, error: 'Invalid range' };
    let n = 0, errors = 0;
    for (let i = from; i <= to; i++) {
      try { await ensureType(i); n++; } catch { errors++; }
    }
    return { success: true, message: `Imported ${n} types (#${from}–${to})${errors ? `, ${errors} errors` : ''}` };
  },
  importTypesByRegion: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('regionPokeapiId'));
    if (!id) return { success: false, error: 'Invalid region' };
    try {
      const res = await importTypesByRegion(id);
      return { success: true, message: `Imported ${res.inserted}/${res.total} types for region` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },

  // ── Abilities ──────────────────────────────────────────────────────────
  importAbilities: async () => {
    try {
      const res = await importAllAbilities();
      return { success: true, message: `Imported ${res.inserted} abilities` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSingleAbility: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('pokeapiId'));
    if (!id) return { success: false, error: 'Invalid ID' };
    try {
      await ensureAbility(id);
      return { success: true, message: `Ability #${id} imported` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importRangeAbilities: async ({ request }) => {
    const fd = await request.formData();
    const from = parseId(fd.get('fromId'));
    const to = parseId(fd.get('toId'));
    if (!from || !to || from > to) return { success: false, error: 'Invalid range' };
    let n = 0, errors = 0;
    for (let i = from; i <= to; i++) {
      try { await ensureAbility(i); n++; } catch { errors++; }
    }
    return { success: true, message: `Imported ${n} abilities (#${from}–${to})${errors ? `, ${errors} errors` : ''}` };
  },
  importAbilitiesByRegion: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('regionPokeapiId'));
    if (!id) return { success: false, error: 'Invalid region' };
    try {
      const res = await importAbilitiesByRegion(id);
      return { success: true, message: `Imported ${res.inserted}/${res.total} abilities for region` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },

  // ── Moves ──────────────────────────────────────────────────────────────
  importMoves: async () => {
    try {
      const res = await importAllMoves();
      return { success: true, message: `Imported ${res.inserted} moves` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSingleMove: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('pokeapiId'));
    if (!id) return { success: false, error: 'Invalid ID' };
    try {
      await ensureMove(id);
      return { success: true, message: `Move #${id} imported` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importRangeMoves: async ({ request }) => {
    const fd = await request.formData();
    const from = parseId(fd.get('fromId'));
    const to = parseId(fd.get('toId'));
    if (!from || !to || from > to) return { success: false, error: 'Invalid range' };
    let n = 0, errors = 0;
    for (let i = from; i <= to; i++) {
      try { await ensureMove(i); n++; } catch { errors++; }
    }
    return { success: true, message: `Imported ${n} moves (#${from}–${to})${errors ? `, ${errors} errors` : ''}` };
  },
  importMovesByRegion: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('regionPokeapiId'));
    if (!id) return { success: false, error: 'Invalid region' };
    try {
      const res = await importMovesByRegion(id);
      return { success: true, message: `Imported ${res.inserted}/${res.total} moves for region` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },

  // ── Items ──────────────────────────────────────────────────────────────
  importItems: async () => {
    try {
      const res = await importAllItems();
      return { success: true, message: `Imported ${res.inserted} items` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSingleItem: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('pokeapiId'));
    if (!id) return { success: false, error: 'Invalid ID' };
    try {
      await ensureItem(id);
      return { success: true, message: `Item #${id} imported` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importRangeItems: async ({ request }) => {
    const fd = await request.formData();
    const from = parseId(fd.get('fromId'));
    const to = parseId(fd.get('toId'));
    if (!from || !to || from > to) return { success: false, error: 'Invalid range' };
    let n = 0, errors = 0;
    for (let i = from; i <= to; i++) {
      try { await ensureItem(i); n++; } catch { errors++; }
    }
    return { success: true, message: `Imported ${n} items (#${from}–${to})${errors ? `, ${errors} errors` : ''}` };
  },

  // ── Pokémon ────────────────────────────────────────────────────────────
  importPokemonByRegion: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('regionPokeapiId'));
    const overwriteEdited = fd.get('overwriteEdited') === 'true';
    if (!id) return { success: false, error: 'Invalid region' };
    try {
      const res = await importPokemonByRegion(id, { forceRefresh: true, overwriteEdited });
      return { success: true, message: `Imported ${res.inserted}/${res.total} Pokémon for region` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSinglePokemon: async ({ request }) => {
    const fd = await request.formData();
    const id = parseId(fd.get('pokeapiId'));
    const overwriteEdited = fd.get('overwriteEdited') === 'true';
    if (!id) return { success: false, error: 'Invalid ID' };
    try {
      await importPokemonById(id, { forceRefresh: true, overwriteEdited });
      return { success: true, message: `Pokémon #${id} imported` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importRangePokemon: async ({ request }) => {
    const fd = await request.formData();
    const from = parseId(fd.get('fromId'));
    const to = parseId(fd.get('toId'));
    const overwriteEdited = fd.get('overwriteEdited') === 'true';
    if (!from || !to || from > to) return { success: false, error: 'Invalid range' };
    try {
      const res = await importPokemonByRange(from, to, { forceRefresh: true, overwriteEdited });
      return { success: true, message: `Imported ${res.inserted} Pokémon (${res.errors.length} errors)` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },

  // ── Pokédexes ──────────────────────────────────────────────────────────
  importPokedexes: async ({ locals }) => {
    requireEditor(locals);
    const err = await checkPokedexPreconditions();
    if (err) return { success: false, error: err };
    try {
      const res = await importAllPokedexes();
      return { success: true, message: `Imported ${res.inserted}/${res.total} Pokédexes (${res.errors.length} errors)` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSinglePokedex: async ({ request, locals }) => {
    requireEditor(locals);
    const err = await checkPokedexPreconditions();
    if (err) return { success: false, error: err };
    const fd = await request.formData();
    const id = parseId(fd.get('pokeapiId'));
    if (!id) return { success: false, error: 'Invalid ID' };
    try {
      await ensurePokedex(id);
      return { success: true, message: `Pokédex #${id} imported` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importPokedexesByVersionGroup: async ({ request, locals }) => {
    requireEditor(locals);
    const err = await checkPokedexPreconditions();
    if (err) return { success: false, error: err };
    const fd = await request.formData();
    const vgPokeapiId = parseId(fd.get('vgPokeapiId'));
    if (!vgPokeapiId) return { success: false, error: 'Invalid version group' };
    const [vg] = await db.select({ id: schema.versionGroups.id })
      .from(schema.versionGroups)
      .where(eq(schema.versionGroups.pokeapiId, vgPokeapiId))
      .limit(1);
    if (!vg) return { success: false, error: 'Version group not found in database — import Game Data first.' };
    try {
      const res = await importPokedexesByVersionGroup(vgPokeapiId);
      return { success: true, message: `Imported ${res.inserted}/${res.total} Pokédexes for this game (${res.errors.length} errors)` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },

  // ── Locations ──────────────────────────────────────────────────────────
  importLocations: async () => {
    try {
      const res = await importAllLocations();
      return {
        success: true,
        message: `Imported ${res.inserted} location areas from ${res.total} locations (${res.errors.length} errors)`,
      };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importLocationsByRegion: async ({ request }) => {
    const fd = await request.formData();
    const regionPokeapiId = parseId(fd.get('regionPokeapiId'));
    if (!regionPokeapiId) return { success: false, error: 'Invalid region' };
    try {
      const res = await importLocationsByRegion(regionPokeapiId);
      return {
        success: true,
        message: `Imported ${res.inserted} location areas from ${res.total} locations (${res.errors.length} errors)`,
      };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
};
