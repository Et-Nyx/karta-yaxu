import { i as importEvents } from './import-events-CiRDkSb8.js';
import { d as db, i as importLog, p as pokemonSpecies, a as pokemon, e as evolutionDetails, b as pokemonTypes, c as pokemonAbilities, f as pokemonStats, g as pokemonMoves, t as types, h as typeNames, j as typeEffectiveness, k as abilities, l as abilityNames, m as abilityFlavorTexts, n as abilityEffects, v as versionGroups, o as moves, q as moveNames, r as moveFlavorTexts, s as moveEffects, u as moveLearnMethods, w as moveLearnMethodNames, x as pokemonForms, y as languages, z as generations, A as generationNames, B as regions, C as regionNames, D as versions, E as versionNames, F as natures, G as eggGroups, H as encounterMethods, I as encounterMethodNames, J as evolutionChains, K as items, L as itemNames, M as itemFlavorTexts, N as itemEffects, O as pokemonFlavorTexts, P as pokemonGenera, Q as speciesEggGroups } from './db-IHYKL_-q.js';
import { desc, eq, count } from 'drizzle-orm';
import 'node:events';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';

//#region src/lib/pokeapi/client.ts
var BASE_URL = "https://pokeapi.co/api/v2";
var cache = /* @__PURE__ */ new Map();
var RATE_LIMIT = 80;
var RATE_WINDOW = 6e4;
var timestamps = [];
async function waitForRateLimit() {
	const now = Date.now();
	while (timestamps.length > 0 && timestamps[0] < now - RATE_WINDOW) timestamps.shift();
	if (timestamps.length >= RATE_LIMIT) await sleep(timestamps[0] + RATE_WINDOW - now + 100);
	timestamps.push(Date.now());
}
function sleep(ms) {
	return new Promise((resolve) => setTimeout(resolve, ms));
}
var MAX_RETRIES = 3;
var INITIAL_BACKOFF = 1e3;
async function fetchFromApi(path) {
	if (cache.has(path)) return cache.get(path);
	await waitForRateLimit();
	let lastError = null;
	for (let attempt = 0; attempt < MAX_RETRIES; attempt++) try {
		const response = await fetch(`${BASE_URL}${path}`);
		if (response.status === 404) throw new Error(`PokeAPI 404: ${path} not found`);
		if (!response.ok) throw new Error(`PokeAPI ${response.status}: ${response.statusText} for ${path}`);
		const data = await response.json();
		cache.set(path, data);
		return data;
	} catch (error) {
		lastError = error;
		if (error.message.includes("404")) throw error;
		const backoffMs = INITIAL_BACKOFF * Math.pow(2, attempt);
		console.warn(`PokeAPI fetch failed (attempt ${attempt + 1}/${MAX_RETRIES}): ${path} — retrying in ${backoffMs}ms`);
		await sleep(backoffMs);
	}
	throw lastError ?? /* @__PURE__ */ new Error(`Failed to fetch ${path} after ${MAX_RETRIES} attempts`);
}
/**
* Extracts the numeric ID from a PokeAPI URL.
* e.g. "https://pokeapi.co/api/v2/region/3/" → 3
*/
function extractIdFromUrl(url) {
	const match = url.match(/\/(\d+)\/?$/);
	if (!match) throw new Error(`Cannot extract ID from URL: ${url}`);
	return parseInt(match[1], 10);
}
var fetchLanguage = (id) => fetchFromApi(`/language/${id}`);
var fetchGeneration = (id) => fetchFromApi(`/generation/${id}`);
var fetchRegion = (id) => fetchFromApi(`/region/${id}`);
var fetchVersionGroup = (id) => fetchFromApi(`/version-group/${id}`);
var fetchVersion = (id) => fetchFromApi(`/version/${id}`);
var fetchType = (id) => fetchFromApi(`/type/${id}`);
var fetchAbility = (id) => fetchFromApi(`/ability/${id}`);
var fetchMove = (id) => fetchFromApi(`/move/${id}`);
var fetchItem = (id) => fetchFromApi(`/item/${id}`);
var fetchNature = (id) => fetchFromApi(`/nature/${id}`);
var fetchEncounterMethod = (id) => fetchFromApi(`/encounter-method/${id}`);
var fetchMoveLearnMethod = (id) => fetchFromApi(`/move-learn-method/${id}`);
var fetchEggGroup = (id) => fetchFromApi(`/egg-group/${id}`);
var fetchPokemon = (id) => fetchFromApi(`/pokemon/${id}`);
var fetchPokemonSpecies = (id) => fetchFromApi(`/pokemon-species/${id}`);
var fetchPokemonForm = (id) => fetchFromApi(`/pokemon-form/${id}`);
var fetchEvolutionChain = (id) => fetchFromApi(`/evolution-chain/${id}`);
async function fetchList(endpoint, limit = 1e4) {
	return fetchFromApi(`/${endpoint}?limit=${limit}`);
}
//#endregion
//#region src/lib/pokeapi/mappers.ts
function slugify(name) {
	return name.toLowerCase().replace(/[^a-z0-9-]/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, "");
}
function resolveLanguageId(langName, languageMap, languageNameToPokeapiId) {
	const pokeapiId = languageNameToPokeapiId.get(langName);
	if (pokeapiId === void 0) return null;
	return languageMap.get(pokeapiId) ?? null;
}
function mapNames(names, languageMap, langNameMap) {
	const result = [];
	for (const n of names) {
		const langId = resolveLanguageId(n.language.name, languageMap, langNameMap);
		if (langId !== null) result.push({
			languageId: langId,
			name: n.name
		});
	}
	return result;
}
function mapVerboseEffects(effects, languageMap, langNameMap) {
	const result = [];
	for (const e of effects) {
		const langId = resolveLanguageId(e.language.name, languageMap, langNameMap);
		if (langId !== null) result.push({
			languageId: langId,
			effect: e.effect,
			shortEffect: e.short_effect
		});
	}
	return result;
}
function mapLanguage(data) {
	return { main: {
		pokeapiId: data.id,
		source: "pokeapi",
		name: data.name,
		iso639: data.iso639,
		iso3166: data.iso3166,
		official: data.official
	} };
}
function mapRegion(data, languageMap, langNameMap) {
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			mainGenerationId: data.main_generation ? void 0 : void 0
		},
		names: mapNames(data.names, languageMap, langNameMap)
	};
}
function mapGeneration(data, regionDbId, languageMap, langNameMap) {
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			regionId: regionDbId,
			order: data.id
		},
		names: mapNames(data.names, languageMap, langNameMap)
	};
}
function mapVersionGroup(data, generationDbId) {
	return { main: {
		pokeapiId: data.id,
		source: "pokeapi",
		name: data.name,
		slug: slugify(data.name),
		generationId: generationDbId,
		order: data.order
	} };
}
function mapVersion(data, versionGroupDbId, languageMap, langNameMap) {
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			versionGroupId: versionGroupDbId
		},
		names: mapNames(data.names, languageMap, langNameMap)
	};
}
function mapNature(data) {
	return { main: {
		pokeapiId: data.id,
		source: "pokeapi",
		name: data.name,
		slug: slugify(data.name),
		increasedStat: data.increased_stat?.name ?? null,
		decreasedStat: data.decreased_stat?.name ?? null,
		likesFlavor: data.likes_flavor?.name ?? null,
		hatesFlavor: data.hates_flavor?.name ?? null
	} };
}
function mapEggGroup(data) {
	return { main: {
		pokeapiId: data.id,
		name: data.name,
		slug: slugify(data.name)
	} };
}
function mapEncounterMethod(data, languageMap, langNameMap) {
	return {
		main: {
			pokeapiId: data.id,
			name: data.name,
			slug: slugify(data.name),
			order: data.order
		},
		names: mapNames(data.names, languageMap, langNameMap)
	};
}
function mapMoveLearnMethod(data, languageMap, langNameMap) {
	return {
		main: {
			pokeapiId: data.id,
			name: data.name,
			slug: slugify(data.name),
			description: data.descriptions.find((d) => d.language.name === "en")?.description ?? null
		},
		names: mapNames(data.names, languageMap, langNameMap)
	};
}
function mapType(data, generationDbId, languageMap, langNameMap) {
	const effectiveness = [];
	for (const t of data.damage_relations.double_damage_to) effectiveness.push({
		defenderPokeapiId: extractIdFromUrl(t.url),
		multiplier: 2
	});
	for (const t of data.damage_relations.half_damage_to) effectiveness.push({
		defenderPokeapiId: extractIdFromUrl(t.url),
		multiplier: .5
	});
	for (const t of data.damage_relations.no_damage_to) effectiveness.push({
		defenderPokeapiId: extractIdFromUrl(t.url),
		multiplier: 0
	});
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			iconUrl: `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/types/generation-ix/scarlet-violet/small/${data.id}.png`,
			moveDamageClass: data.move_damage_class?.name ?? null,
			generationId: generationDbId
		},
		names: mapNames(data.names, languageMap, langNameMap),
		effectiveness
	};
}
function mapAbility(data, generationDbId, languageMap, langNameMap, versionGroupMap) {
	const flavorTexts = [];
	for (const ft of data.flavor_text_entries) {
		const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
		const vgPokeapiId = extractIdFromUrl(ft.version_group.url);
		const vgDbId = versionGroupMap.get(vgPokeapiId);
		if (langId !== null && vgDbId !== void 0) flavorTexts.push({
			languageId: langId,
			flavorText: ft.flavor_text,
			versionGroupId: vgDbId
		});
	}
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			isMainSeries: data.is_main_series,
			generationId: generationDbId
		},
		names: mapNames(data.names, languageMap, langNameMap),
		flavorTexts,
		effects: mapVerboseEffects(data.effect_entries, languageMap, langNameMap)
	};
}
function mapMove(data, typeDbId, generationDbId, languageMap, langNameMap, versionGroupMap) {
	const flavorTexts = [];
	for (const ft of data.flavor_text_entries) {
		const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
		const vgPokeapiId = extractIdFromUrl(ft.version_group.url);
		const vgDbId = versionGroupMap.get(vgPokeapiId);
		if (langId !== null && vgDbId !== void 0) flavorTexts.push({
			languageId: langId,
			flavorText: ft.flavor_text,
			versionGroupId: vgDbId
		});
	}
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			typeId: typeDbId,
			damageClass: data.damage_class.name,
			power: data.power,
			accuracy: data.accuracy,
			pp: data.pp,
			priority: data.priority,
			target: data.target.name,
			generationId: generationDbId
		},
		names: mapNames(data.names, languageMap, langNameMap),
		flavorTexts,
		effects: mapVerboseEffects(data.effect_entries, languageMap, langNameMap)
	};
}
function mapItem(data, languageMap, langNameMap, versionGroupMap) {
	const flavorTexts = [];
	for (const ft of data.flavor_text_entries) {
		const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
		if (langId !== null && "version_group" in ft) {
			const vgPokeapiId = extractIdFromUrl(ft.version_group.url);
			const vgDbId = versionGroupMap.get(vgPokeapiId);
			if (vgDbId !== void 0) flavorTexts.push({
				languageId: langId,
				flavorText: ft.text,
				versionGroupId: vgDbId
			});
		}
	}
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			cost: data.cost,
			flingPower: data.fling_power,
			category: data.category.name,
			spriteUrl: data.sprites.default
		},
		names: mapNames(data.names, languageMap, langNameMap),
		flavorTexts,
		effects: mapVerboseEffects(data.effect_entries, languageMap, langNameMap)
	};
}
function mapPokemonSpecies(data, generationDbId, evolutionChainDbId, evolvesFromDbId, languageMap, langNameMap, versionToVersionGroupMap) {
	const flavorTexts = [];
	for (const ft of data.flavor_text_entries) {
		const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
		if (langId !== null) {
			const versionPokeapiId = extractIdFromUrl(ft.version.url);
			const versionGroupId = versionToVersionGroupMap.get(versionPokeapiId);
			if (versionGroupId !== void 0) flavorTexts.push({
				languageId: langId,
				flavorText: ft.flavor_text,
				versionGroupId
			});
		}
	}
	const genera = [];
	for (const g of data.genera) {
		const langId = resolveLanguageId(g.language.name, languageMap, langNameMap);
		if (langId !== null) genera.push({
			languageId: langId,
			genus: g.genus
		});
	}
	const enGenus = data.genera.find((g) => g.language.name === "en")?.genus ?? null;
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			name: data.name,
			slug: slugify(data.name),
			genus: enGenus,
			dexNumber: data.id,
			evolutionChainId: evolutionChainDbId,
			evolvesFromSpeciesId: evolvesFromDbId,
			isBaby: data.is_baby,
			isLegendary: data.is_legendary,
			isMythical: data.is_mythical,
			habitat: data.habitat?.name ?? null,
			color: data.color.name,
			shape: data.shape?.name ?? null,
			growthRate: data.growth_rate.name,
			baseHappiness: data.base_happiness,
			captureRate: data.capture_rate,
			genderRate: data.gender_rate,
			hatchCounter: data.hatch_counter,
			generationId: generationDbId,
			order: data.order
		},
		genera,
		flavorTexts,
		eggGroupPokeapiIds: data.egg_groups.map((eg) => extractIdFromUrl(eg.url))
	};
}
function mapPokemon(data, speciesDbId) {
	const stats = data.stats.reduce((acc, s) => {
		const statName = s.stat.name;
		if (statName === "hp") acc.hp = s.base_stat;
		else if (statName === "attack") acc.atk = s.base_stat;
		else if (statName === "defense") acc.def = s.base_stat;
		else if (statName === "special-attack") acc.spa = s.base_stat;
		else if (statName === "special-defense") acc.spd = s.base_stat;
		else if (statName === "speed") acc.spe = s.base_stat;
		return acc;
	}, {
		hp: 0,
		atk: 0,
		def: 0,
		spa: 0,
		spd: 0,
		spe: 0
	});
	const artwork = data.sprites.other?.["official-artwork"]?.front_default ?? null;
	return {
		main: {
			pokeapiId: data.id,
			source: "pokeapi",
			speciesId: speciesDbId,
			name: data.name,
			slug: slugify(data.name),
			height: data.height,
			weight: data.weight,
			...stats,
			spriteDefault: data.sprites.front_default,
			spriteShiny: data.sprites.front_shiny,
			spriteArtwork: artwork,
			isDefault: data.is_default,
			order: data.order
		},
		types: data.types.map((t) => ({
			typePokeapiId: extractIdFromUrl(t.type.url),
			slot: t.slot
		})),
		abilities: data.abilities.map((a) => ({
			abilityPokeapiId: extractIdFromUrl(a.ability.url),
			slot: a.slot,
			isHidden: a.is_hidden
		})),
		stats: data.stats.map((s) => ({
			statName: s.stat.name,
			baseStat: s.base_stat,
			effort: s.effort
		})),
		moves: data.moves.flatMap((m) => {
			const movePokeapiId = extractIdFromUrl(m.move.url);
			return m.version_group_details.map((vgd) => ({
				movePokeapiId,
				learnMethodPokeapiId: extractIdFromUrl(vgd.move_learn_method.url),
				level: vgd.level_learned_at,
				versionGroupPokeapiId: extractIdFromUrl(vgd.version_group.url)
			}));
		})
	};
}
function mapPokemonForm(data, pokemonDbId, speciesDbId) {
	return { main: {
		pokeapiId: data.id,
		source: "pokeapi",
		pokemonId: pokemonDbId,
		speciesId: speciesDbId,
		formName: data.form_name || null,
		isDefault: data.is_default,
		isBattleOnly: data.is_battle_only,
		isMega: data.is_mega,
		order: data.order,
		spriteDefault: data.sprites.front_default,
		spriteShiny: data.sprites.front_shiny,
		spriteArtwork: null
	} };
}
function flattenEvolutionChain(chain, parentSpeciesPokeapiId = null) {
	const details = [];
	const currentSpeciesId = extractIdFromUrl(chain.species.url);
	if (parentSpeciesPokeapiId !== null) for (const ed of chain.evolution_details) details.push({
		fromSpeciesPokeapiId: parentSpeciesPokeapiId,
		toSpeciesPokeapiId: currentSpeciesId,
		trigger: ed.trigger.name,
		minLevel: ed.min_level,
		minHappiness: ed.min_happiness,
		minBeauty: ed.min_beauty,
		minAffection: ed.min_affection,
		itemPokeapiId: ed.item ? extractIdFromUrl(ed.item.url) : null,
		heldItemPokeapiId: ed.held_item ? extractIdFromUrl(ed.held_item.url) : null,
		knownMovePokeapiId: ed.known_move ? extractIdFromUrl(ed.known_move.url) : null,
		knownMoveTypePokeapiId: ed.known_move_type ? extractIdFromUrl(ed.known_move_type.url) : null,
		location: ed.location?.name ?? null,
		timeOfDay: ed.time_of_day || null,
		gender: ed.gender,
		relativePhysicalStats: ed.relative_physical_stats,
		needsOverworldRain: ed.needs_overworld_rain,
		needsMultiplayer: false,
		turnUpsideDown: ed.turn_upside_down,
		tradeSpeciesPokeapiId: ed.trade_species ? extractIdFromUrl(ed.trade_species.url) : null,
		partySpeciesPokeapiId: ed.party_species ? extractIdFromUrl(ed.party_species.url) : null,
		partyTypePokeapiId: ed.party_type ? extractIdFromUrl(ed.party_type.url) : null
	});
	for (const child of chain.evolves_to) details.push(...flattenEvolutionChain(child, currentSpeciesId));
	return details;
}
/** Extract all species pokeapiIds from a chain (for dependency resolution) */
function extractSpeciesFromChain(chain) {
	const ids = [extractIdFromUrl(chain.species.url)];
	for (const child of chain.evolves_to) ids.push(...extractSpeciesFromChain(child));
	return ids;
}
//#endregion
//#region src/lib/pokeapi/import-service.ts
function emptyResult$1(entity) {
	return {
		entity,
		total: 0,
		inserted: 0,
		updated: 0,
		skipped: 0,
		errors: []
	};
}
async function batchProcess(items, fn, concurrency = 5) {
	for (let i = 0; i < items.length; i += concurrency) {
		const batch = items.slice(i, i + concurrency);
		await Promise.all(batch.map(fn));
	}
}
async function logImport(entityType, pokeapiId, action, reason) {
	const timestamp = (/* @__PURE__ */ new Date()).toISOString();
	importEvents.emit("log", {
		entityType,
		pokeapiId,
		action,
		reason,
		timestamp
	});
	await db.insert(importLog).values({
		entityType,
		pokeapiId,
		action,
		reason: null
	});
}
var _langNameToPokeapiId = null;
async function getLangNameMap() {
	if (_langNameToPokeapiId) return _langNameToPokeapiId;
	const langs = await db.select({
		name: languages.name,
		pokeapiId: languages.pokeapiId
	}).from(languages);
	_langNameToPokeapiId = /* @__PURE__ */ new Map();
	for (const l of langs) if (l.pokeapiId !== null) _langNameToPokeapiId.set(l.name, l.pokeapiId);
	return _langNameToPokeapiId;
}
function clearLangNameCache() {
	_langNameToPokeapiId = null;
}
async function buildIdMap(table) {
	const rows = await db.select({
		id: table.id,
		pokeapiId: table.pokeapiId
	}).from(table);
	const map = /* @__PURE__ */ new Map();
	for (const r of rows) if (r.pokeapiId !== null) map.set(r.pokeapiId, r.id);
	return map;
}
async function protectedUpsert(table, data, pokeapiId) {
	const existing = await db.select().from(table).where(eq(table.pokeapiId, pokeapiId)).get();
	if (existing) {
		if (existing.source && existing.source !== "pokeapi") return {
			action: "skipped",
			id: existing.id
		};
		await db.update(table).set(data).where(eq(table.id, existing.id));
		return {
			action: "updated",
			id: existing.id
		};
	}
	return {
		action: "inserted",
		id: (await db.insert(table).values(data).returning({ id: table.id }))[0].id
	};
}
async function ensureLanguage(pokeapiId) {
	const existing = await db.select().from(languages).where(eq(languages.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const { id } = await protectedUpsert(languages, mapLanguage(await fetchLanguage(pokeapiId)).main, pokeapiId);
	await logImport("language", pokeapiId, "inserted");
	clearLangNameCache();
	return id;
}
async function ensureRegion(pokeapiId) {
	const existing = await db.select().from(regions).where(eq(regions.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const languageMap = await buildIdMap(languages);
	const langNameMap = await getLangNameMap();
	const mapped = mapRegion(await fetchRegion(pokeapiId), languageMap, langNameMap);
	const { id } = await protectedUpsert(regions, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(regionNames).values({
		regionId: id,
		...n
	}).onConflictDoNothing();
	await logImport("region", pokeapiId, "inserted");
	return id;
}
async function ensureGeneration(pokeapiId) {
	const existing = await db.select().from(generations).where(eq(generations.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchGeneration(pokeapiId);
	const mapped = mapGeneration(data, await ensureRegion(extractIdFromUrl(data.main_region.url)), await buildIdMap(languages), await getLangNameMap());
	const { id } = await protectedUpsert(generations, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(generationNames).values({
		generationId: id,
		...n
	}).onConflictDoNothing();
	await logImport("generation", pokeapiId, "inserted");
	return id;
}
async function ensureVersionGroup(pokeapiId) {
	const existing = await db.select().from(versionGroups).where(eq(versionGroups.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchVersionGroup(pokeapiId);
	const { id } = await protectedUpsert(versionGroups, mapVersionGroup(data, await ensureGeneration(extractIdFromUrl(data.generation.url))).main, pokeapiId);
	await logImport("version_group", pokeapiId, "inserted");
	return id;
}
async function ensureVersion(pokeapiId) {
	const existing = await db.select().from(versions).where(eq(versions.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchVersion(pokeapiId);
	const mapped = mapVersion(data, await ensureVersionGroup(extractIdFromUrl(data.version_group.url)), await buildIdMap(languages), await getLangNameMap());
	const { id } = await protectedUpsert(versions, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(versionNames).values({
		versionId: id,
		...n
	}).onConflictDoNothing();
	await logImport("version", pokeapiId, "inserted");
	return id;
}
async function ensureNature(pokeapiId) {
	const existing = await db.select().from(natures).where(eq(natures.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const { id } = await protectedUpsert(natures, mapNature(await fetchNature(pokeapiId)).main, pokeapiId);
	await logImport("nature", pokeapiId, "inserted");
	return id;
}
async function ensureEggGroup(pokeapiId) {
	const existing = await db.select().from(eggGroups).where(eq(eggGroups.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const mapped = mapEggGroup(await fetchEggGroup(pokeapiId));
	const result = await db.insert(eggGroups).values(mapped.main).returning({ id: eggGroups.id });
	await logImport("egg_group", pokeapiId, "inserted");
	return result[0].id;
}
async function ensureMoveLearnMethod(pokeapiId) {
	const existing = await db.select().from(moveLearnMethods).where(eq(moveLearnMethods.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const mapped = mapMoveLearnMethod(await fetchMoveLearnMethod(pokeapiId), await buildIdMap(languages), await getLangNameMap());
	const id = (await db.insert(moveLearnMethods).values(mapped.main).returning({ id: moveLearnMethods.id }))[0].id;
	for (const n of mapped.names) await db.insert(moveLearnMethodNames).values({
		moveLearnMethodId: id,
		...n
	}).onConflictDoNothing();
	await logImport("move_learn_method", pokeapiId, "inserted");
	return id;
}
async function ensureEncounterMethod(pokeapiId) {
	const existing = await db.select().from(encounterMethods).where(eq(encounterMethods.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const mapped = mapEncounterMethod(await fetchEncounterMethod(pokeapiId), await buildIdMap(languages), await getLangNameMap());
	const id = (await db.insert(encounterMethods).values(mapped.main).returning({ id: encounterMethods.id }))[0].id;
	for (const n of mapped.names) await db.insert(encounterMethodNames).values({
		encounterMethodId: id,
		...n
	}).onConflictDoNothing();
	await logImport("encounter_method", pokeapiId, "inserted");
	return id;
}
async function importAllLanguages() {
	const result = emptyResult$1("languages");
	const list = await fetchList("language");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureLanguage(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	clearLangNameCache();
	return result;
}
async function importAllRegions() {
	const result = emptyResult$1("regions");
	const list = await fetchList("region");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureRegion(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllGenerations() {
	const result = emptyResult$1("generations");
	const list = await fetchList("generation");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureGeneration(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllVersionGroups() {
	const result = emptyResult$1("version_groups");
	const list = await fetchList("version-group");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureVersionGroup(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllVersions() {
	const result = emptyResult$1("versions");
	const list = await fetchList("version");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureVersion(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllNatures() {
	const result = emptyResult$1("natures");
	const list = await fetchList("nature");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureNature(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllEggGroups() {
	const result = emptyResult$1("egg_groups");
	const list = await fetchList("egg-group");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureEggGroup(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllMoveLearnMethods() {
	const result = emptyResult$1("move_learn_methods");
	const list = await fetchList("move-learn-method");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureMoveLearnMethod(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllEncounterMethods() {
	const result = emptyResult$1("encounter_methods");
	const list = await fetchList("encounter-method");
	result.total = list.results.length;
	for (const item of list.results) try {
		await ensureEncounterMethod(extractIdFromUrl(item.url));
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
/** Import all game metadata in dependency order */
async function importAllGameData() {
	const results = [];
	console.log("📦 Importing all game data...");
	console.log("  → Languages...");
	results.push(await importAllLanguages());
	console.log("  → Regions...");
	results.push(await importAllRegions());
	console.log("  → Generations...");
	results.push(await importAllGenerations());
	console.log("  → Version Groups...");
	results.push(await importAllVersionGroups());
	console.log("  → Versions...");
	results.push(await importAllVersions());
	console.log("  → Natures...");
	results.push(await importAllNatures());
	console.log("  → Egg Groups...");
	results.push(await importAllEggGroups());
	console.log("  → Move Learn Methods...");
	results.push(await importAllMoveLearnMethods());
	console.log("  → Encounter Methods...");
	results.push(await importAllEncounterMethods());
	console.log("✅ Game data import complete.");
	return results;
}
//#endregion
//#region src/lib/pokeapi/import-pokemon.ts
function emptyResult(entity) {
	return {
		entity,
		total: 0,
		inserted: 0,
		updated: 0,
		skipped: 0,
		errors: []
	};
}
async function ensureType(pokeapiId) {
	const existing = await db.select().from(types).where(eq(types.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchType(pokeapiId);
	const mapped = mapType(data, await ensureGeneration(extractIdFromUrl(data.generation.url)), await buildIdMap(languages), await getLangNameMap());
	const { id } = await protectedUpsert(types, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(typeNames).values({
		typeId: id,
		...n
	}).onConflictDoNothing();
	for (const eff of mapped.effectiveness) {
		const defenderDbId = await ensureType(eff.defenderPokeapiId);
		await db.insert(typeEffectiveness).values({
			attackerId: id,
			defenderId: defenderDbId,
			multiplier: eff.multiplier
		}).onConflictDoNothing();
	}
	await logImport("type", pokeapiId, "inserted");
	return id;
}
async function ensureAbility(pokeapiId) {
	const existing = await db.select().from(abilities).where(eq(abilities.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchAbility(pokeapiId);
	const mapped = mapAbility(data, await ensureGeneration(extractIdFromUrl(data.generation.url)), await buildIdMap(languages), await getLangNameMap(), await buildIdMap(versionGroups));
	const { id } = await protectedUpsert(abilities, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(abilityNames).values({
		abilityId: id,
		...n
	}).onConflictDoNothing();
	for (const ft of mapped.flavorTexts) await db.insert(abilityFlavorTexts).values({
		abilityId: id,
		...ft
	}).onConflictDoNothing();
	for (const e of mapped.effects) await db.insert(abilityEffects).values({
		abilityId: id,
		...e
	}).onConflictDoNothing();
	await logImport("ability", pokeapiId, "inserted");
	return id;
}
async function ensureMove(pokeapiId) {
	const existing = await db.select().from(moves).where(eq(moves.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchMove(pokeapiId);
	const mapped = mapMove(data, await ensureType(extractIdFromUrl(data.type.url)), await ensureGeneration(extractIdFromUrl(data.generation.url)), await buildIdMap(languages), await getLangNameMap(), await buildIdMap(versionGroups));
	const { id } = await protectedUpsert(moves, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(moveNames).values({
		moveId: id,
		...n
	}).onConflictDoNothing();
	for (const ft of mapped.flavorTexts) await db.insert(moveFlavorTexts).values({
		moveId: id,
		...ft
	}).onConflictDoNothing();
	for (const e of mapped.effects) await db.insert(moveEffects).values({
		moveId: id,
		...e
	}).onConflictDoNothing();
	await logImport("move", pokeapiId, "inserted");
	return id;
}
async function ensureItem(pokeapiId) {
	const existing = await db.select().from(items).where(eq(items.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const mapped = mapItem(await fetchItem(pokeapiId), await buildIdMap(languages), await getLangNameMap(), await buildIdMap(versionGroups));
	const { id } = await protectedUpsert(items, mapped.main, pokeapiId);
	for (const n of mapped.names) await db.insert(itemNames).values({
		itemId: id,
		...n
	}).onConflictDoNothing();
	for (const ft of mapped.flavorTexts) await db.insert(itemFlavorTexts).values({
		itemId: id,
		...ft
	}).onConflictDoNothing();
	for (const e of mapped.effects) await db.insert(itemEffects).values({
		itemId: id,
		...e
	}).onConflictDoNothing();
	await logImport("item", pokeapiId, "inserted");
	return id;
}
async function ensureEvolutionChain(pokeapiId) {
	const existing = await db.select().from(evolutionChains).where(eq(evolutionChains.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchEvolutionChain(pokeapiId);
	let babyItemDbId = null;
	if (data.baby_trigger_item) babyItemDbId = await ensureItem(extractIdFromUrl(data.baby_trigger_item.url));
	const chainDbId = (await db.insert(evolutionChains).values({
		pokeapiId,
		source: "pokeapi",
		babyTriggerItemId: babyItemDbId
	}).returning({ id: evolutionChains.id }))[0].id;
	const speciesIds = extractSpeciesFromChain(data.chain);
	const speciesDbMap = /* @__PURE__ */ new Map();
	for (const spId of speciesIds) {
		const dbId = await ensurePokemonSpeciesBase(spId, chainDbId);
		speciesDbMap.set(spId, dbId);
	}
	const details = flattenEvolutionChain(data.chain);
	for (const detail of details) {
		const fromDbId = speciesDbMap.get(detail.fromSpeciesPokeapiId);
		const toDbId = speciesDbMap.get(detail.toSpeciesPokeapiId);
		if (!fromDbId || !toDbId) continue;
		let itemDbId = null;
		if (detail.itemPokeapiId) itemDbId = await ensureItem(detail.itemPokeapiId);
		let heldItemDbId = null;
		if (detail.heldItemPokeapiId) heldItemDbId = await ensureItem(detail.heldItemPokeapiId);
		let knownMoveDbId = null;
		if (detail.knownMovePokeapiId) knownMoveDbId = await ensureMove(detail.knownMovePokeapiId);
		let knownMoveTypeDbId = null;
		if (detail.knownMoveTypePokeapiId) knownMoveTypeDbId = await ensureType(detail.knownMoveTypePokeapiId);
		await db.insert(evolutionDetails).values({
			chainId: chainDbId,
			fromSpeciesId: fromDbId,
			toSpeciesId: toDbId,
			trigger: detail.trigger,
			minLevel: detail.minLevel,
			minHappiness: detail.minHappiness,
			minBeauty: detail.minBeauty,
			minAffection: detail.minAffection,
			itemId: itemDbId,
			heldItemId: heldItemDbId,
			knownMoveId: knownMoveDbId,
			knownMoveTypeId: knownMoveTypeDbId,
			location: detail.location,
			timeOfDay: detail.timeOfDay,
			gender: detail.gender,
			relativePhysicalStats: detail.relativePhysicalStats,
			needsOverworldRain: detail.needsOverworldRain,
			turnUpsideDown: detail.turnUpsideDown
		}).onConflictDoNothing();
	}
	await logImport("evolution_chain", pokeapiId, "inserted");
	return chainDbId;
}
/**
* Ensures a PokemonSpecies exists. This is the "base" version that inserts the
* species record without requiring the full evolution chain to be resolved yet.
* Used internally by ensureEvolutionChain to break circular deps.
*/
async function ensurePokemonSpeciesBase(pokeapiId, evolutionChainDbId) {
	const existing = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.pokeapiId, pokeapiId)).get();
	const data = await fetchPokemonSpecies(pokeapiId);
	const genDbId = await ensureGeneration(extractIdFromUrl(data.generation.url));
	let evolvesFromDbId = null;
	if (data.evolves_from_species) evolvesFromDbId = await ensurePokemonSpeciesBase(extractIdFromUrl(data.evolves_from_species.url), evolutionChainDbId);
	for (const eg of data.egg_groups) await ensureEggGroup(extractIdFromUrl(eg.url));
	const languageMap = await buildIdMap(languages);
	const langNameMap = await getLangNameMap();
	const versions$1 = await db.select({
		pokeapiId: versions.pokeapiId,
		versionGroupId: versions.versionGroupId
	}).from(versions);
	const versionToVgMap = /* @__PURE__ */ new Map();
	for (const v of versions$1) if (v.pokeapiId !== null) versionToVgMap.set(v.pokeapiId, v.versionGroupId);
	const mapped = mapPokemonSpecies(data, genDbId, evolutionChainDbId, evolvesFromDbId, languageMap, langNameMap, versionToVgMap);
	if (existing) {
		for (const ft of mapped.flavorTexts) await db.insert(pokemonFlavorTexts).values({
			speciesId: existing.id,
			versionGroupId: ft.versionGroupId,
			languageId: ft.languageId,
			flavorText: ft.flavorText
		}).onConflictDoNothing();
		return existing.id;
	}
	const { id } = await protectedUpsert(pokemonSpecies, mapped.main, pokeapiId);
	for (const g of mapped.genera) await db.insert(pokemonGenera).values({
		speciesId: id,
		...g
	}).onConflictDoNothing();
	for (const egPokeapiId of mapped.eggGroupPokeapiIds) {
		const egDbId = await ensureEggGroup(egPokeapiId);
		await db.insert(speciesEggGroups).values({
			speciesId: id,
			eggGroupId: egDbId
		}).onConflictDoNothing();
	}
	for (const ft of mapped.flavorTexts) await db.insert(pokemonFlavorTexts).values({
		speciesId: id,
		versionGroupId: ft.versionGroupId,
		languageId: ft.languageId,
		flavorText: ft.flavorText
	}).onConflictDoNothing();
	await logImport("pokemon_species", pokeapiId, "inserted");
	return id;
}
/**
* Public entry point: ensures a PokemonSpecies and its full evolution chain.
*/
async function ensurePokemonSpecies(pokeapiId) {
	const existing = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	await ensureEvolutionChain(extractIdFromUrl((await fetchPokemonSpecies(pokeapiId)).evolution_chain.url));
	const species = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.pokeapiId, pokeapiId)).get();
	if (!species) throw new Error(`Failed to import species ${pokeapiId} via evolution chain`);
	return species.id;
}
async function ensurePokemon(pokeapiId) {
	const existing = await db.select().from(pokemon).where(eq(pokemon.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const data = await fetchPokemon(pokeapiId);
	const speciesDbId = await ensurePokemonSpecies(extractIdFromUrl(data.species.url));
	const mapped = mapPokemon(data, speciesDbId);
	const { id: pokemonDbId } = await protectedUpsert(pokemon, mapped.main, pokeapiId);
	for (const t of mapped.types) {
		const typeDbId = await ensureType(t.typePokeapiId);
		await db.insert(pokemonTypes).values({
			pokemonId: pokemonDbId,
			typeId: typeDbId,
			slot: t.slot
		}).onConflictDoNothing();
	}
	for (const a of mapped.abilities) {
		const abilityDbId = await ensureAbility(a.abilityPokeapiId);
		await db.insert(pokemonAbilities).values({
			pokemonId: pokemonDbId,
			abilityId: abilityDbId,
			slot: a.slot,
			isHidden: a.isHidden
		}).onConflictDoNothing();
	}
	for (const s of mapped.stats) await db.insert(pokemonStats).values({
		pokemonId: pokemonDbId,
		...s
	}).onConflictDoNothing();
	for (const m of mapped.moves) try {
		const vgDbId = await ensureVersionGroup(m.versionGroupPokeapiId);
		const moveDbId = await ensureMove(m.movePokeapiId);
		const learnMethodDbId = await ensureMoveLearnMethod(m.learnMethodPokeapiId);
		await db.insert(pokemonMoves).values({
			pokemonId: pokemonDbId,
			moveId: moveDbId,
			learnMethodId: learnMethodDbId,
			level: m.level,
			versionGroupId: vgDbId
		}).onConflictDoNothing();
	} catch {}
	for (const formRef of data.forms) await ensurePokemonForm(extractIdFromUrl(formRef.url), pokemonDbId, speciesDbId);
	await logImport("pokemon", pokeapiId, "inserted");
	return pokemonDbId;
}
async function ensurePokemonForm(pokeapiId, pokemonDbId, speciesDbId) {
	const existing = await db.select().from(pokemonForms).where(eq(pokemonForms.pokeapiId, pokeapiId)).get();
	if (existing) return existing.id;
	const mapped = mapPokemonForm(await fetchPokemonForm(pokeapiId), pokemonDbId, speciesDbId);
	const result = await db.insert(pokemonForms).values(mapped.main).returning({ id: pokemonForms.id });
	await logImport("pokemon_form", pokeapiId, "inserted");
	return result[0].id;
}
async function importAllTypes() {
	const result = emptyResult("types");
	const list = await fetchList("type");
	result.total = list.results.length;
	for (const item of list.results) try {
		const pid = extractIdFromUrl(item.url);
		if (pid > 1e4) continue;
		await ensureType(pid);
		result.inserted++;
	} catch (e) {
		result.errors.push({
			id: extractIdFromUrl(item.url),
			error: e.message
		});
	}
	return result;
}
async function importAllAbilities() {
	const result = emptyResult("abilities");
	const list = await fetchList("ability");
	result.total = list.results.length;
	await batchProcess(list.results, async (item) => {
		try {
			await ensureAbility(extractIdFromUrl(item.url));
			result.inserted++;
		} catch (e) {
			result.errors.push({
				id: extractIdFromUrl(item.url),
				error: e.message
			});
		}
	}, 5);
	return result;
}
async function importAllMoves() {
	const result = emptyResult("moves");
	const filtered = (await fetchList("move")).results.filter((item) => extractIdFromUrl(item.url) <= 1e4);
	result.total = filtered.length;
	await batchProcess(filtered, async (item) => {
		try {
			await ensureMove(extractIdFromUrl(item.url));
			result.inserted++;
		} catch (e) {
			result.errors.push({
				id: extractIdFromUrl(item.url),
				error: e.message
			});
		}
	}, 5);
	return result;
}
async function importAllItems() {
	const result = emptyResult("items");
	const list = await fetchList("item");
	result.total = list.results.length;
	await batchProcess(list.results, async (item) => {
		try {
			await ensureItem(extractIdFromUrl(item.url));
			result.inserted++;
		} catch (e) {
			result.errors.push({
				id: extractIdFromUrl(item.url),
				error: e.message
			});
		}
	}, 5);
	return result;
}
/** Import a single Pokémon by PokeAPI ID with full dependency resolution */
async function importPokemonById(pokeapiId) {
	console.log(`🔴 Importing Pokémon #${pokeapiId}...`);
	const dbId = await ensurePokemon(pokeapiId);
	const speciesRow = await db.select().from(pokemonSpecies).innerJoin(pokemon, eq(pokemon.speciesId, pokemonSpecies.id)).where(eq(pokemon.id, dbId)).get();
	if (speciesRow && speciesRow.pokemon_species.evolutionChainId) {
		const chainEdges = await db.select().from(evolutionDetails).where(eq(evolutionDetails.chainId, speciesRow.pokemon_species.evolutionChainId));
		const speciesIds = /* @__PURE__ */ new Set();
		speciesIds.add(speciesRow.pokemon_species.id);
		for (const edge of chainEdges) {
			speciesIds.add(edge.fromSpeciesId);
			speciesIds.add(edge.toSpeciesId);
		}
		for (const spId of speciesIds) if (spId !== speciesRow.pokemon_species.id) {
			const spRecord = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.id, spId)).get();
			if (spRecord) {
				const defaultVariety = (await fetchPokemonSpecies(spRecord.pokeapiId)).varieties.find((v) => v.is_default);
				if (defaultVariety) {
					const defaultPokeapiId = extractIdFromUrl(defaultVariety.pokemon.url);
					if (defaultPokeapiId !== pokeapiId) {
						console.log(`  ↪ Also importing family member #${defaultPokeapiId}...`);
						await ensurePokemon(defaultPokeapiId);
					}
				}
			}
		}
	}
	console.log(`✅ Pokémon #${pokeapiId} (and family) imported (db id: ${dbId})`);
	return dbId;
}
/** Import a range of Pokémon (e.g. 1-151 for Gen 1) */
async function importPokemonByRange(from, to) {
	const result = emptyResult("pokemon");
	result.total = to - from + 1;
	for (let id = from; id <= to; id++) try {
		await ensurePokemon(id);
		result.inserted++;
		console.log(`  ✅ #${id} done (${result.inserted}/${result.total})`);
	} catch (e) {
		result.errors.push({
			id,
			error: e.message
		});
		console.error(`  ❌ #${id} failed: ${e.message}`);
	}
	return result;
}
//#endregion
//#region src/routes/import/+page.server.ts
async function getTableCounts(table) {
	const result = await db.select({
		source: table.source,
		c: count()
	}).from(table).groupBy(table.source);
	const counts = {
		total: 0,
		pokeapi: 0,
		edited: 0,
		custom: 0
	};
	for (const row of result) {
		if (row.source === "pokeapi") counts.pokeapi = row.c;
		else if (row.source === "edited") counts.edited = row.c;
		else if (row.source === "custom") counts.custom = row.c;
		counts.total += row.c;
	}
	return counts;
}
var load = async () => {
	return {
		status: {
			languages: await getTableCounts(languages),
			regions: await getTableCounts(regions),
			generations: await getTableCounts(generations),
			types: await getTableCounts(types),
			abilities: await getTableCounts(abilities),
			moves: await getTableCounts(moves),
			items: await getTableCounts(items),
			pokemon: await getTableCounts(pokemon)
		},
		logs: await db.select().from(importLog).orderBy(desc(importLog.timestamp)).limit(50)
	};
};
var actions = {
	importGameData: async () => {
		try {
			const res = await importAllGameData();
			return {
				success: true,
				message: `Imported Game Data (Languages: ${res.languages}, Generations: ${res.generations})`
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	},
	importTypes: async () => {
		try {
			await importAllTypes();
			return {
				success: true,
				message: "Imported Types"
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	},
	importAbilities: async () => {
		try {
			await importAllAbilities();
			return {
				success: true,
				message: "Imported Abilities"
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	},
	importMoves: async () => {
		try {
			await importAllMoves();
			return {
				success: true,
				message: "Imported Moves"
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	},
	importItems: async () => {
		try {
			await importAllItems();
			return {
				success: true,
				message: "Imported Items"
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	},
	importSinglePokemon: async ({ request }) => {
		const data = await request.formData();
		const id = parseInt(data.get("pokeapiId"));
		if (isNaN(id) || id <= 0) return {
			success: false,
			error: "Invalid ID"
		};
		try {
			return {
				success: true,
				message: `Imported Pokemon DB ID: ${await importPokemonById(id)}`
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	},
	importRangePokemon: async ({ request }) => {
		const data = await request.formData();
		const fromId = parseInt(data.get("fromId"));
		const toId = parseInt(data.get("toId"));
		if (isNaN(fromId) || isNaN(toId) || fromId > toId || fromId <= 0) return {
			success: false,
			error: "Invalid Range"
		};
		try {
			const res = await importPokemonByRange(fromId, toId);
			return {
				success: true,
				message: `Imported ${res.imported} Pokemon (${res.skipped} skipped)`
			};
		} catch (e) {
			return {
				success: false,
				error: e.message
			};
		}
	}
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	actions: actions,
	load: load
});

const index = 3;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-CQXUOrl3.js')).default;
const server_id = "src/routes/import/+page.server.ts";
const imports = ["_app/immutable/nodes/3.CandOcun.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/DsitodRo.js","_app/immutable/chunks/CP97kCR3.js"];
const stylesheets = ["_app/immutable/assets/3.DEntuJaD.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=3-Bgaf2t78.js.map
