// ============================================================================
// PokeAPI → Drizzle Mappers
// Pure functions: JSON → insert-ready objects. No I/O.
// ============================================================================

import { extractIdFromUrl } from './client';
import type {
  PokeApiLanguage, PokeApiGeneration, PokeApiRegion,
  PokeApiVersionGroup, PokeApiVersion, PokeApiPokedex,
  PokeApiType, PokeApiAbility, PokeApiMove, PokeApiItem, PokeApiNature,
  PokeApiPokemon, PokeApiPokemonSpecies, PokeApiPokemonForm,
  PokeApiEvolutionChain, ChainLink,
  PokeApiLocation, PokeApiLocationArea,
  PokeApiEncounterMethod, PokeApiEncounterCondition, PokeApiEncounterConditionValue,
  PokeApiMoveLearnMethod, PokeApiEggGroup,
  Name, VerboseEffect,
} from './types';

// ── Lookup Map Types ───────────────────────────────────────────────────────

/** Maps pokeapiId → internal db id */
export type IdMap = Map<number, number>;

// ── Utilities ──────────────────────────────────────────────────────────────

export function slugify(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9-]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '');
}

function resolveLanguageId(langName: string, languageMap: IdMap, languageNameToPokeapiId: Map<string, number>): number | null {
  const pokeapiId = languageNameToPokeapiId.get(langName);
  if (pokeapiId === undefined) return null;
  return languageMap.get(pokeapiId) ?? null;
}

function mapNames(names: Name[], languageMap: IdMap, langNameMap: Map<string, number>): Array<{ languageId: number; name: string }> {
  const result: Array<{ languageId: number; name: string }> = [];
  for (const n of names) {
    const langId = resolveLanguageId(n.language.name, languageMap, langNameMap);
    if (langId !== null) {
      result.push({ languageId: langId, name: n.name });
    }
  }
  return result;
}

function mapVerboseEffects(effects: VerboseEffect[], languageMap: IdMap, langNameMap: Map<string, number>): Array<{ languageId: number; effect: string; shortEffect: string }> {
  const result: Array<{ languageId: number; effect: string; shortEffect: string }> = [];
  for (const e of effects) {
    const langId = resolveLanguageId(e.language.name, languageMap, langNameMap);
    if (langId !== null) {
      result.push({ languageId: langId, effect: e.effect, shortEffect: e.short_effect });
    }
  }
  return result;
}

// ── Game Metadata Mappers ──────────────────────────────────────────────────

export function mapLanguage(data: PokeApiLanguage) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      iso639: data.iso639,
      iso3166: data.iso3166,
      official: data.official,
    },
  };
}

export function mapRegion(data: PokeApiRegion, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      mainGenerationId: data.main_generation ? undefined : undefined, // resolved by caller
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}

export function mapGeneration(data: PokeApiGeneration, regionDbId: number | null, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      regionId: regionDbId,
      order: data.id,
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}

export function mapVersionGroup(data: PokeApiVersionGroup, generationDbId: number) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      generationId: generationDbId,
      order: data.order,
    },
  };
}

export function mapVersion(data: PokeApiVersion, versionGroupDbId: number, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      versionGroupId: versionGroupDbId,
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}

export function mapPokedex(data: PokeApiPokedex, regionDbId: number | null, languageMap: IdMap, langNameMap: Map<string, number>) {
  const nameEntries = mapNames(data.names, languageMap, langNameMap);
  const descEntries: Array<{ languageId: number; name: string; description: string | null }> = [];
  for (const ne of nameEntries) {
    const desc = data.descriptions.find(d => {
      const pid = langNameMap.get(d.language.name);
      return pid !== undefined && languageMap.get(pid) === ne.languageId;
    });
    descEntries.push({ ...ne, description: desc?.description ?? null });
  }
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      regionId: regionDbId,
      isMainSeries: data.is_main_series,
    },
    names: descEntries,
  };
}

export function mapNature(data: PokeApiNature) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      increasedStat: data.increased_stat?.name ?? null,
      decreasedStat: data.decreased_stat?.name ?? null,
      likesFlavor: data.likes_flavor?.name ?? null,
      hatesFlavor: data.hates_flavor?.name ?? null,
    },
  };
}

export function mapEggGroup(data: PokeApiEggGroup) {
  return {
    main: {
      pokeapiId: data.id,
      name: data.name,
      slug: slugify(data.name),
    },
  };
}

export function mapEncounterMethod(data: PokeApiEncounterMethod, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      name: data.name,
      slug: slugify(data.name),
      order: data.order,
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}

export function mapEncounterCondition(data: PokeApiEncounterCondition) {
  return {
    main: {
      pokeapiId: data.id,
      name: data.name,
      slug: slugify(data.name),
    },
  };
}

export function mapEncounterConditionValue(data: PokeApiEncounterConditionValue, conditionDbId: number) {
  return {
    main: {
      pokeapiId: data.id,
      conditionId: conditionDbId,
      name: data.name,
      slug: slugify(data.name),
    },
  };
}

export function mapMoveLearnMethod(data: PokeApiMoveLearnMethod, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      name: data.name,
      slug: slugify(data.name),
      description: data.descriptions.find(d => d.language.name === 'en')?.description ?? null,
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}

// ── Core Data Mappers ──────────────────────────────────────────────────────

export function mapType(data: PokeApiType, generationDbId: number | null, languageMap: IdMap, langNameMap: Map<string, number>) {
  // Map damage relations to effectiveness entries
  const effectiveness: Array<{ defenderPokeapiId: number; multiplier: number }> = [];
  for (const t of data.damage_relations.double_damage_to) {
    effectiveness.push({ defenderPokeapiId: extractIdFromUrl(t.url), multiplier: 2 });
  }
  for (const t of data.damage_relations.half_damage_to) {
    effectiveness.push({ defenderPokeapiId: extractIdFromUrl(t.url), multiplier: 0.5 });
  }
  for (const t of data.damage_relations.no_damage_to) {
    effectiveness.push({ defenderPokeapiId: extractIdFromUrl(t.url), multiplier: 0 });
  }

  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      iconUrl: `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/types/generation-ix/scarlet-violet/small/${data.id}.png`,
      moveDamageClass: (data.move_damage_class?.name as 'physical' | 'special' | 'status') ?? null,
      generationId: generationDbId,
    },
    names: mapNames(data.names, languageMap, langNameMap),
    effectiveness,
  };
}

export function mapAbility(
  data: PokeApiAbility,
  generationDbId: number | null,
  languageMap: IdMap,
  langNameMap: Map<string, number>,
  versionGroupMap: IdMap,
) {
  const flavorTexts: Array<{ languageId: number; flavorText: string; versionGroupId: number }> = [];
  for (const ft of data.flavor_text_entries) {
    const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
    const vgPokeapiId = extractIdFromUrl(ft.version_group.url);
    const vgDbId = versionGroupMap.get(vgPokeapiId);
    if (langId !== null && vgDbId !== undefined) {
      flavorTexts.push({ languageId: langId, flavorText: ft.flavor_text, versionGroupId: vgDbId });
    }
  }

  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      isMainSeries: data.is_main_series,
      generationId: generationDbId,
    },
    names: mapNames(data.names, languageMap, langNameMap),
    flavorTexts,
    effects: mapVerboseEffects(data.effect_entries, languageMap, langNameMap),
  };
}

export function mapMove(
  data: PokeApiMove,
  typeDbId: number,
  generationDbId: number | null,
  languageMap: IdMap,
  langNameMap: Map<string, number>,
  versionGroupMap: IdMap,
) {
  const flavorTexts: Array<{ languageId: number; flavorText: string; versionGroupId: number }> = [];
  for (const ft of data.flavor_text_entries) {
    const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
    const vgPokeapiId = extractIdFromUrl(ft.version_group.url);
    const vgDbId = versionGroupMap.get(vgPokeapiId);
    if (langId !== null && vgDbId !== undefined) {
      flavorTexts.push({ languageId: langId, flavorText: ft.flavor_text, versionGroupId: vgDbId });
    }
  }

  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      typeId: typeDbId,
      damageClass: data.damage_class.name as 'physical' | 'special' | 'status',
      power: data.power,
      accuracy: data.accuracy,
      pp: data.pp,
      priority: data.priority,
      target: data.target.name,
      generationId: generationDbId,
    },
    names: mapNames(data.names, languageMap, langNameMap),
    flavorTexts,
    effects: mapVerboseEffects(data.effect_entries, languageMap, langNameMap),
  };
}

export function mapItem(
  data: PokeApiItem,
  languageMap: IdMap,
  langNameMap: Map<string, number>,
  versionGroupMap: IdMap,
) {
  const flavorTexts: Array<{ languageId: number; flavorText: string; versionGroupId: number }> = [];
  for (const ft of data.flavor_text_entries) {
    const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
    // Item flavor texts use `version` not `version_group` in PokeAPI — but our schema expects version_group
    // We'll skip items that don't map cleanly for now
    if (langId !== null && 'version_group' in ft) {
      const vgPokeapiId = extractIdFromUrl((ft as any).version_group.url);
      const vgDbId = versionGroupMap.get(vgPokeapiId);
      if (vgDbId !== undefined) {
        flavorTexts.push({ languageId: langId, flavorText: (ft as any).text, versionGroupId: vgDbId });
      }
    }
  }

  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      cost: data.cost,
      flingPower: data.fling_power,
      category: data.category.name,
      spriteUrl: data.sprites.default,
    },
    names: mapNames(data.names, languageMap, langNameMap),
    flavorTexts,
    effects: mapVerboseEffects(data.effect_entries, languageMap, langNameMap),
  };
}

// ── Pokémon Mappers ────────────────────────────────────────────────────────

export function mapPokemonSpecies(
  data: PokeApiPokemonSpecies,
  generationDbId: number | null,
  evolutionChainDbId: number | null,
  evolvesFromDbId: number | null,
  languageMap: IdMap,
  langNameMap: Map<string, number>,
  versionToVersionGroupMap: IdMap,
) {
  const flavorTexts: Array<{ languageId: number; flavorText: string; versionGroupId: number }> = [];
  for (const ft of data.flavor_text_entries) {
    const langId = resolveLanguageId(ft.language.name, languageMap, langNameMap);
    if (langId !== null) {
      const versionPokeapiId = extractIdFromUrl(ft.version.url);
      const versionGroupId = versionToVersionGroupMap.get(versionPokeapiId);
      if (versionGroupId !== undefined) {
        flavorTexts.push({ languageId: langId, flavorText: ft.flavor_text, versionGroupId });
      }
    }
  }

  const genera: Array<{ languageId: number; genus: string }> = [];
  for (const g of data.genera) {
    const langId = resolveLanguageId(g.language.name, languageMap, langNameMap);
    if (langId !== null) {
      genera.push({ languageId: langId, genus: g.genus });
    }
  }

  const enGenus = data.genera.find(g => g.language.name === 'en')?.genus ?? null;

  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      genus: enGenus,
      dexNumber: data.id, // national dex number = species id
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
      order: data.order,
    },
    genera,
    flavorTexts,
    eggGroupPokeapiIds: data.egg_groups.map(eg => extractIdFromUrl(eg.url)),
  };
}

export function mapPokemon(
  data: PokeApiPokemon,
  speciesDbId: number,
) {
  const stats = data.stats.reduce((acc, s) => {
    const statName = s.stat.name;
    if (statName === 'hp') acc.hp = s.base_stat;
    else if (statName === 'attack') acc.atk = s.base_stat;
    else if (statName === 'defense') acc.def = s.base_stat;
    else if (statName === 'special-attack') acc.spa = s.base_stat;
    else if (statName === 'special-defense') acc.spd = s.base_stat;
    else if (statName === 'speed') acc.spe = s.base_stat;
    return acc;
  }, { hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 });

  const artwork = data.sprites.other?.['official-artwork']?.front_default ?? null;

  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
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
      order: data.order,
    },
    types: data.types.map(t => ({
      typePokeapiId: extractIdFromUrl(t.type.url),
      slot: t.slot,
    })),
    abilities: data.abilities.map(a => ({
      abilityPokeapiId: extractIdFromUrl(a.ability.url),
      slot: a.slot,
      isHidden: a.is_hidden,
    })),
    stats: data.stats.map(s => ({
      statName: s.stat.name,
      baseStat: s.base_stat,
      effort: s.effort,
    })),
    moves: data.moves.flatMap(m => {
      const movePokeapiId = extractIdFromUrl(m.move.url);
      return m.version_group_details.map(vgd => ({
        movePokeapiId,
        learnMethodPokeapiId: extractIdFromUrl(vgd.move_learn_method.url),
        level: vgd.level_learned_at,
        versionGroupPokeapiId: extractIdFromUrl(vgd.version_group.url),
      }));
    }),
  };
}

export function mapPokemonForm(data: PokeApiPokemonForm, pokemonDbId: number, speciesDbId: number) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      pokemonId: pokemonDbId,
      speciesId: speciesDbId,
      formName: data.form_name || null,
      isDefault: data.is_default,
      isBattleOnly: data.is_battle_only,
      isMega: data.is_mega,
      order: data.order,
      spriteDefault: data.sprites.front_default,
      spriteShiny: data.sprites.front_shiny,
      spriteArtwork: null,
    },
  };
}

// ── Evolution Chain Mapper ─────────────────────────────────────────────────

export interface FlattenedEvolutionDetail {
  fromSpeciesPokeapiId: number;
  toSpeciesPokeapiId: number;
  trigger: string;
  minLevel: number | null;
  minHappiness: number | null;
  minBeauty: number | null;
  minAffection: number | null;
  itemPokeapiId: number | null;
  heldItemPokeapiId: number | null;
  knownMovePokeapiId: number | null;
  knownMoveTypePokeapiId: number | null;
  location: string | null;
  timeOfDay: string | null;
  gender: number | null;
  relativePhysicalStats: number | null;
  needsOverworldRain: boolean;
  needsMultiplayer: boolean;
  turnUpsideDown: boolean;
  tradeSpeciesPokeapiId: number | null;
  partySpeciesPokeapiId: number | null;
  partyTypePokeapiId: number | null;
}

export function flattenEvolutionChain(chain: ChainLink, parentSpeciesPokeapiId: number | null = null): FlattenedEvolutionDetail[] {
  const details: FlattenedEvolutionDetail[] = [];
  const currentSpeciesId = extractIdFromUrl(chain.species.url);

  // Process evolution details (how this species was reached)
  if (parentSpeciesPokeapiId !== null) {
    for (const ed of chain.evolution_details) {
      details.push({
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
        partyTypePokeapiId: ed.party_type ? extractIdFromUrl(ed.party_type.url) : null,
      });
    }
  }

  // Recurse into evolves_to
  for (const child of chain.evolves_to) {
    details.push(...flattenEvolutionChain(child, currentSpeciesId));
  }

  return details;
}

/** Extract all species pokeapiIds from a chain (for dependency resolution) */
export function extractSpeciesFromChain(chain: ChainLink): number[] {
  const ids: number[] = [extractIdFromUrl(chain.species.url)];
  for (const child of chain.evolves_to) {
    ids.push(...extractSpeciesFromChain(child));
  }
  return ids;
}

// ── Location Mappers ───────────────────────────────────────────────────────

export function mapLocation(data: PokeApiLocation, regionDbId: number | null, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      regionId: regionDbId,
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}

export function mapLocationArea(data: PokeApiLocationArea, locationDbId: number, languageMap: IdMap, langNameMap: Map<string, number>) {
  return {
    main: {
      pokeapiId: data.id,
      source: 'pokeapi' as const,
      name: data.name,
      slug: slugify(data.name),
      locationId: locationDbId,
      gameIndex: data.game_index,
    },
    names: mapNames(data.names, languageMap, langNameMap),
  };
}
