import { t as __exportAll } from "./chunk.js";
import { drizzle } from "drizzle-orm/better-sqlite3";
import Database from "better-sqlite3";
import { integer, primaryKey, real, sqliteTable, text } from "drizzle-orm/sqlite-core";
//#region src/lib/db/schema/game-metadata.ts
var languages = sqliteTable("languages", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	iso639: text("iso639"),
	iso3166: text("iso3166"),
	official: integer("is_official", { mode: "boolean" }).notNull().default(true)
});
var regions = sqliteTable("regions", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	mainGenerationId: integer("main_generation_id")
});
var regionNames = sqliteTable("region_names", {
	regionId: integer("region_id").notNull().references(() => regions.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.regionId, t.languageId] }) }));
var generations = sqliteTable("generations", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	regionId: integer("region_id").references(() => regions.id),
	order: integer("order")
});
var generationNames = sqliteTable("generation_names", {
	generationId: integer("generation_id").notNull().references(() => generations.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.generationId, t.languageId] }) }));
var versionGroups = sqliteTable("version_groups", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	generationId: integer("generation_id").notNull().references(() => generations.id),
	order: integer("order")
});
var versionGroupNames = sqliteTable("version_group_names", {
	versionGroupId: integer("version_group_id").notNull().references(() => versionGroups.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.versionGroupId, t.languageId] }) }));
var versions = sqliteTable("versions", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	versionGroupId: integer("version_group_id").notNull().references(() => versionGroups.id)
});
var versionNames = sqliteTable("version_names", {
	versionId: integer("version_id").notNull().references(() => versions.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.versionId, t.languageId] }) }));
var pokedexes = sqliteTable("pokedexes", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	regionId: integer("region_id").references(() => regions.id),
	isMainSeries: integer("is_main_series", { mode: "boolean" }).notNull().default(true)
});
var pokedexNames = sqliteTable("pokedex_names", {
	pokedexId: integer("pokedex_id").notNull().references(() => pokedexes.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull(),
	description: text("description")
}, (t) => ({ pk: primaryKey({ columns: [t.pokedexId, t.languageId] }) }));
//#endregion
//#region src/lib/db/schema/locations.ts
var locations = sqliteTable("locations", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	regionId: integer("region_id").references(() => regions.id)
});
var locationNames = sqliteTable("location_names", {
	locationId: integer("location_id").notNull().references(() => locations.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.locationId, t.languageId] }) }));
var locationAreas = sqliteTable("location_areas", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	locationId: integer("location_id").notNull().references(() => locations.id),
	gameIndex: integer("game_index")
});
var locationAreaNames = sqliteTable("location_area_names", {
	locationAreaId: integer("location_area_id").notNull().references(() => locationAreas.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.locationAreaId, t.languageId] }) }));
var encounterMethods = sqliteTable("encounter_methods", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	order: integer("order")
});
var encounterMethodNames = sqliteTable("encounter_method_names", {
	encounterMethodId: integer("encounter_method_id").notNull().references(() => encounterMethods.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull(),
	description: text("description")
}, (t) => ({ pk: primaryKey({ columns: [t.encounterMethodId, t.languageId] }) }));
var encounterConditions = sqliteTable("encounter_conditions", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique()
});
var encounterConditionValues = sqliteTable("encounter_condition_values", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	conditionId: integer("condition_id").notNull().references(() => encounterConditions.id),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique()
});
//#endregion
//#region src/lib/db/schema/core-data.ts
var types = sqliteTable("types", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	hexColor: text("hexcolor"),
	iconUrl: text("icon_url"),
	moveDamageClass: text("move_damage_class", { enum: [
		"physical",
		"special",
		"status"
	] }),
	generationId: integer("generation_id").references(() => generations.id)
});
var typeEffectiveness = sqliteTable("type_effectiveness", {
	attackerId: integer("attacker_id").notNull().references(() => types.id),
	defenderId: integer("defender_id").notNull().references(() => types.id),
	multiplier: real("multiplier").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.attackerId, t.defenderId] }) }));
var typeNames = sqliteTable("type_names", {
	typeId: integer("type_id").notNull().references(() => types.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.typeId, t.languageId] }) }));
var abilities = sqliteTable("abilities", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	isMainSeries: integer("is_main_series", { mode: "boolean" }).notNull().default(true),
	generationId: integer("generation_id").references(() => generations.id)
});
var abilityNames = sqliteTable("ability_names", {
	abilityId: integer("ability_id").notNull().references(() => abilities.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.abilityId, t.languageId] }) }));
var abilityFlavorTexts = sqliteTable("ability_flavor_texts", {
	abilityId: integer("ability_id").notNull().references(() => abilities.id),
	flavorText: text("flavor_text").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id),
	versionGroupId: integer("version_group_id").notNull().references(() => versionGroups.id)
}, (t) => ({ pk: primaryKey({ columns: [
	t.abilityId,
	t.languageId,
	t.versionGroupId
] }) }));
var abilityEffects = sqliteTable("ability_effects", {
	abilityId: integer("ability_id").notNull().references(() => abilities.id),
	effect: text("effect").notNull(),
	shortEffect: text("short_effect").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id)
}, (t) => ({ pk: primaryKey({ columns: [t.abilityId, t.languageId] }) }));
var moves = sqliteTable("moves", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	typeId: integer("type_id").notNull().references(() => types.id),
	damageClass: text("damage_class", { enum: [
		"physical",
		"special",
		"status"
	] }).notNull(),
	power: integer("power"),
	accuracy: integer("accuracy"),
	pp: integer("pp").notNull(),
	priority: integer("priority").notNull().default(0),
	target: text("target"),
	generationId: integer("generation_id").references(() => generations.id)
});
var moveNames = sqliteTable("move_names", {
	moveId: integer("move_id").notNull().references(() => moves.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.moveId, t.languageId] }) }));
var moveFlavorTexts = sqliteTable("move_flavor_texts", {
	moveId: integer("move_id").notNull().references(() => moves.id),
	flavorText: text("flavor_text").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id),
	versionGroupId: integer("version_group_id").notNull().references(() => versionGroups.id)
}, (t) => ({ pk: primaryKey({ columns: [
	t.moveId,
	t.languageId,
	t.versionGroupId
] }) }));
var moveEffects = sqliteTable("move_effects", {
	moveId: integer("move_id").notNull().references(() => moves.id),
	effect: text("effect").notNull(),
	shortEffect: text("short_effect").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id)
}, (t) => ({ pk: primaryKey({ columns: [t.moveId, t.languageId] }) }));
var moveLearnMethods = sqliteTable("move_learn_methods", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	description: text("description")
});
var moveLearnMethodNames = sqliteTable("move_learn_method_names", {
	moveLearnMethodId: integer("move_learn_method_id").notNull().references(() => moveLearnMethods.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull(),
	description: text("description")
}, (t) => ({ pk: primaryKey({ columns: [t.moveLearnMethodId, t.languageId] }) }));
//#endregion
//#region src/lib/db/schema/items.ts
var items = sqliteTable("items", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	cost: integer("cost"),
	flingPower: integer("fling_power"),
	category: text("category"),
	spriteUrl: text("sprite_url")
});
var itemNames = sqliteTable("item_names", {
	itemId: integer("item_id").notNull().references(() => items.id),
	languageId: integer("language_id").notNull().references(() => languages.id),
	name: text("name").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.itemId, t.languageId] }) }));
var itemFlavorTexts = sqliteTable("item_flavor_texts", {
	itemId: integer("item_id").notNull().references(() => items.id),
	flavorText: text("flavor_text").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id),
	versionGroupId: integer("version_group_id").notNull().references(() => versionGroups.id)
}, (t) => ({ pk: primaryKey({ columns: [
	t.itemId,
	t.languageId,
	t.versionGroupId
] }) }));
var itemEffects = sqliteTable("item_effects", {
	itemId: integer("item_id").notNull().references(() => items.id),
	effect: text("effect").notNull(),
	shortEffect: text("short_effect").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id)
}, (t) => ({ pk: primaryKey({ columns: [t.itemId, t.languageId] }) }));
var natures = sqliteTable("natures", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	increasedStat: text("increased_stat"),
	decreasedStat: text("decreased_stat"),
	likesFlavor: text("likes_flavor"),
	hatesFlavor: text("hates_flavor")
});
//#endregion
//#region src/lib/db/schema/pokemon.ts
var eggGroups = sqliteTable("egg_groups", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique()
});
var evolutionChains = sqliteTable("evolution_chains", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	babyTriggerItemId: integer("baby_trigger_item_id").references(() => items.id)
});
var pokemonSpecies = sqliteTable("pokemon_species", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	genus: text("genus"),
	dexNumber: integer("dex_number").notNull(),
	evolutionChainId: integer("evolution_chain_id").references(() => evolutionChains.id),
	evolvesFromSpeciesId: integer("evolves_from_species_id"),
	isBaby: integer("is_baby", { mode: "boolean" }).notNull().default(false),
	isLegendary: integer("is_legendary", { mode: "boolean" }).notNull().default(false),
	isMythical: integer("is_mythical", { mode: "boolean" }).notNull().default(false),
	habitat: text("habitat"),
	color: text("color"),
	shape: text("shape"),
	growthRate: text("growth_rate"),
	baseHappiness: integer("base_happiness"),
	captureRate: integer("capture_rate"),
	genderRate: integer("gender_rate"),
	hatchCounter: integer("hatch_counter"),
	generationId: integer("generation_id").references(() => generations.id),
	order: integer("order")
});
var pokemonFlavorTexts = sqliteTable("pokemon_flavor_texts", {
	speciesId: integer("species_id").notNull().references(() => pokemonSpecies.id),
	flavorText: text("flavor_text").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id),
	versionGroupId: integer("version_group_id").notNull().references(() => versionGroups.id)
}, (t) => ({ pk: primaryKey({ columns: [
	t.speciesId,
	t.languageId,
	t.versionGroupId
] }) }));
var pokemonGenera = sqliteTable("pokemon_genera", {
	speciesId: integer("species_id").notNull().references(() => pokemonSpecies.id),
	genus: text("genus").notNull(),
	languageId: integer("language_id").notNull().references(() => languages.id)
}, (t) => ({ pk: primaryKey({ columns: [t.speciesId, t.languageId] }) }));
var speciesEggGroups = sqliteTable("species_egg_groups", {
	speciesId: integer("species_id").notNull().references(() => pokemonSpecies.id),
	eggGroupId: integer("egg_group_id").notNull().references(() => eggGroups.id)
}, (t) => ({ pk: primaryKey({ columns: [t.speciesId, t.eggGroupId] }) }));
var pokemon = sqliteTable("pokemon", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	speciesId: integer("species_id").notNull().references(() => pokemonSpecies.id),
	name: text("name").notNull().unique(),
	slug: text("slug").notNull().unique(),
	height: integer("height").notNull(),
	weight: integer("weight").notNull(),
	hp: integer("hp").notNull(),
	atk: integer("atk").notNull(),
	def: integer("def").notNull(),
	spa: integer("spa").notNull(),
	spd: integer("spd").notNull(),
	spe: integer("spe").notNull(),
	spriteDefault: text("sprite_default"),
	spriteShiny: text("sprite_shiny"),
	spriteArtwork: text("sprite_artwork"),
	isDefault: integer("is_default", { mode: "boolean" }).notNull().default(true),
	order: integer("order")
});
var pokemonTypes = sqliteTable("pokemon_types", {
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	typeId: integer("type_id").notNull().references(() => types.id),
	slot: integer("slot").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.pokemonId, t.slot] }) }));
var pokemonAbilities = sqliteTable("pokemon_abilities", {
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	abilityId: integer("ability_id").notNull().references(() => abilities.id),
	slot: integer("slot").notNull(),
	isHidden: integer("is_hidden", { mode: "boolean" }).notNull().default(false)
}, (t) => ({ pk: primaryKey({ columns: [t.pokemonId, t.abilityId] }) }));
var pokemonStats = sqliteTable("pokemon_stats", {
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	statName: text("stat_name").notNull(),
	baseStat: integer("base_stat").notNull(),
	effort: integer("effort").notNull().default(0)
}, (t) => ({ pk: primaryKey({ columns: [t.pokemonId, t.statName] }) }));
var pokemonMoves = sqliteTable("pokemon_moves", {
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	moveId: integer("move_id").notNull().references(() => moves.id),
	learnMethodId: integer("learn_method_id").notNull().references(() => moveLearnMethods.id),
	level: integer("level"),
	versionGroupId: integer("version_group_id").references(() => versionGroups.id)
}, (t) => ({ pk: primaryKey({ columns: [
	t.pokemonId,
	t.moveId,
	t.learnMethodId,
	t.level
] }) }));
var pokemonForms = sqliteTable("pokemon_forms", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	pokeapiId: integer("pokeapi_id"),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	speciesId: integer("species_id").notNull().references(() => pokemonSpecies.id),
	formName: text("form_name"),
	isDefault: integer("is_default", { mode: "boolean" }).notNull().default(false),
	isBattleOnly: integer("is_battle_only", { mode: "boolean" }).notNull().default(false),
	isMega: integer("is_mega", { mode: "boolean" }).notNull().default(false),
	order: integer("order"),
	spriteDefault: text("sprite_default"),
	spriteShiny: text("sprite_shiny"),
	spriteArtwork: text("sprite_artwork")
});
var evolutionDetails = sqliteTable("evolution_details", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	chainId: integer("chain_id").notNull().references(() => evolutionChains.id),
	fromSpeciesId: integer("from_species_id").notNull().references(() => pokemonSpecies.id),
	toSpeciesId: integer("to_species_id").notNull().references(() => pokemonSpecies.id),
	trigger: text("trigger").notNull(),
	minLevel: integer("min_level"),
	minHappiness: integer("min_happiness"),
	minBeauty: integer("min_beauty"),
	minAffection: integer("min_affection"),
	itemId: integer("item_id").references(() => items.id),
	heldItemId: integer("held_item_id").references(() => items.id),
	knownMoveId: integer("known_move_id").references(() => moves.id),
	knownMoveTypeId: integer("known_move_type_id").references(() => types.id),
	location: text("location"),
	timeOfDay: text("time_of_day"),
	gender: integer("gender"),
	relativePhysicalStats: integer("relative_physical_stats"),
	needsOverworldRain: integer("needs_overworld_rain", { mode: "boolean" }).default(false),
	needsMultiplayer: integer("needs_multiplayer", { mode: "boolean" }).default(false),
	turnUpsideDown: integer("turn_upside_down", { mode: "boolean" }).default(false),
	tradeSpeciesId: integer("trade_species_id").references(() => pokemonSpecies.id),
	partySpeciesId: integer("party_species_id").references(() => pokemonSpecies.id),
	partyTypeId: integer("party_type_id").references(() => types.id),
	region: text("region"),
	baseFormId: integer("base_form_id").references(() => pokemonForms.id),
	usedMoveId: integer("used_move_id").references(() => moves.id),
	minMoveCount: integer("min_move_count"),
	minSteps: integer("min_steps"),
	minDamageTaken: integer("min_damage_taken")
});
var pokedexEntries = sqliteTable("pokedex_entries", {
	pokedexId: integer("pokedex_id").notNull().references(() => pokedexes.id),
	speciesId: integer("species_id").notNull().references(() => pokemonSpecies.id),
	entryNumber: integer("entry_number").notNull()
}, (t) => ({ pk: primaryKey({ columns: [t.pokedexId, t.speciesId] }) }));
var encounters = sqliteTable("encounters", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	source: text("source", { enum: [
		"pokeapi",
		"edited",
		"custom"
	] }).notNull().default("pokeapi"),
	locationAreaId: integer("location_area_id").notNull().references(() => locationAreas.id),
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	versionId: integer("version_id").notNull().references(() => versions.id),
	encounterMethodId: integer("encounter_method_id").notNull().references(() => encounterMethods.id),
	conditionValueId: integer("condition_value_id").references(() => encounterConditionValues.id),
	minLevel: integer("min_level").notNull(),
	maxLevel: integer("max_level").notNull(),
	chance: integer("chance").notNull(),
	slot: integer("slot")
});
//#endregion
//#region src/lib/db/schema/trainers.ts
var trainers = sqliteTable("trainers", {
	id: text("id").primaryKey(),
	username: text("username").notNull().unique(),
	passwordHash: text("password_hash").notNull(),
	role: text("role", { enum: ["viewer", "editor"] }).notNull().default("viewer"),
	isNpc: integer("is_npc", { mode: "boolean" }).notNull().default(false),
	createdAt: integer("created_at", { mode: "timestamp" }).$defaultFn(() => /* @__PURE__ */ new Date())
});
var teams = sqliteTable("teams", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	trainerId: text("trainer_id").notNull().references(() => trainers.id),
	name: text("name").notNull(),
	createdAt: integer("created_at", { mode: "timestamp" }).$defaultFn(() => /* @__PURE__ */ new Date())
});
var teamMembers = sqliteTable("team_members", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	teamId: integer("team_id").notNull().references(() => teams.id),
	slot: integer("slot").notNull(),
	pokemonId: integer("pokemon_id").notNull().references(() => pokemon.id),
	abilityId: integer("ability_id").notNull().references(() => abilities.id),
	natureId: integer("nature_id").notNull().references(() => natures.id),
	heldItemId: integer("held_item_id").references(() => items.id),
	move1Id: integer("move1_id").references(() => moves.id),
	move2Id: integer("move2_id").references(() => moves.id),
	move3Id: integer("move3_id").references(() => moves.id),
	move4Id: integer("move4_id").references(() => moves.id)
});
var battles = sqliteTable("battles", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	trainerId1: text("trainer_id_1").notNull().references(() => trainers.id),
	trainerId2: text("trainer_id_2").notNull().references(() => trainers.id),
	teamId1: integer("team_id_1").notNull().references(() => teams.id),
	teamId2: integer("team_id_2").notNull().references(() => teams.id),
	winnerId: text("winner_id").references(() => trainers.id),
	log: text("log").notNull().default(""),
	playedAt: integer("played_at", { mode: "timestamp" }).$defaultFn(() => /* @__PURE__ */ new Date())
});
var edits = sqliteTable("edits", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	trainerId: text("trainer_id").notNull().references(() => trainers.id),
	tableName: text("table_name").notNull(),
	recordId: integer("record_id").notNull(),
	previousData: text("previous_data"),
	newData: text("new_data"),
	editedAt: integer("edited_at", { mode: "timestamp" }).$defaultFn(() => /* @__PURE__ */ new Date())
});
var importLog = sqliteTable("import_log", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	entityType: text("entity_type").notNull(),
	pokeapiId: integer("pokeapi_id").notNull(),
	action: text("action", { enum: [
		"inserted",
		"updated",
		"skipped"
	] }).notNull(),
	reason: text("reason"),
	timestamp: integer("timestamp", { mode: "timestamp" }).$defaultFn(() => /* @__PURE__ */ new Date())
});
//#endregion
//#region src/lib/db/schema/index.ts
var schema_exports = /* @__PURE__ */ __exportAll({
	abilities: () => abilities,
	abilityEffects: () => abilityEffects,
	abilityFlavorTexts: () => abilityFlavorTexts,
	abilityNames: () => abilityNames,
	battles: () => battles,
	edits: () => edits,
	eggGroups: () => eggGroups,
	encounterConditionValues: () => encounterConditionValues,
	encounterConditions: () => encounterConditions,
	encounterMethodNames: () => encounterMethodNames,
	encounterMethods: () => encounterMethods,
	encounters: () => encounters,
	evolutionChains: () => evolutionChains,
	evolutionDetails: () => evolutionDetails,
	generationNames: () => generationNames,
	generations: () => generations,
	importLog: () => importLog,
	itemEffects: () => itemEffects,
	itemFlavorTexts: () => itemFlavorTexts,
	itemNames: () => itemNames,
	items: () => items,
	languages: () => languages,
	locationAreaNames: () => locationAreaNames,
	locationAreas: () => locationAreas,
	locationNames: () => locationNames,
	locations: () => locations,
	moveEffects: () => moveEffects,
	moveFlavorTexts: () => moveFlavorTexts,
	moveLearnMethodNames: () => moveLearnMethodNames,
	moveLearnMethods: () => moveLearnMethods,
	moveNames: () => moveNames,
	moves: () => moves,
	natures: () => natures,
	pokedexEntries: () => pokedexEntries,
	pokedexNames: () => pokedexNames,
	pokedexes: () => pokedexes,
	pokemon: () => pokemon,
	pokemonAbilities: () => pokemonAbilities,
	pokemonFlavorTexts: () => pokemonFlavorTexts,
	pokemonForms: () => pokemonForms,
	pokemonGenera: () => pokemonGenera,
	pokemonMoves: () => pokemonMoves,
	pokemonSpecies: () => pokemonSpecies,
	pokemonStats: () => pokemonStats,
	pokemonTypes: () => pokemonTypes,
	regionNames: () => regionNames,
	regions: () => regions,
	speciesEggGroups: () => speciesEggGroups,
	teamMembers: () => teamMembers,
	teams: () => teams,
	trainers: () => trainers,
	typeEffectiveness: () => typeEffectiveness,
	typeNames: () => typeNames,
	types: () => types,
	versionGroupNames: () => versionGroupNames,
	versionGroups: () => versionGroups,
	versionNames: () => versionNames,
	versions: () => versions
});
//#endregion
//#region src/lib/db/index.ts
var client = new Database(process.env.DATABASE_URL || "./data/rom.db");
client.pragma("journal_mode = WAL");
var db = drizzle(client, { schema: schema_exports });
//#endregion
export { moves as A, regions as B, abilityFlavorTexts as C, moveLearnMethodNames as D, moveFlavorTexts as E, encounterMethods as F, versionNames as H, generationNames as I, generations as L, typeNames as M, types as N, moveLearnMethods as O, encounterMethodNames as P, languages as R, abilityEffects as S, moveEffects as T, versions as U, versionGroups as V, itemFlavorTexts as _, evolutionDetails as a, natures as b, pokemonFlavorTexts as c, pokemonMoves as d, pokemonSpecies as f, itemEffects as g, speciesEggGroups as h, evolutionChains as i, typeEffectiveness as j, moveNames as k, pokemonForms as l, pokemonTypes as m, importLog as n, pokemon as o, pokemonStats as p, eggGroups as r, pokemonAbilities as s, db as t, pokemonGenera as u, itemNames as v, abilityNames as w, abilities as x, items as y, regionNames as z };
