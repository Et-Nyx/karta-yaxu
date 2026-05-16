import { json } from '@sveltejs/kit';
import { and, asc, eq } from 'drizzle-orm';
import { db } from '$lib/db';
import { pokemon, pokemonAbilities, pokemonTypes } from '$lib/db/schema/pokemon';
import { requireEditor } from '$lib/auth/guards';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params, locals }) => {
  requireEditor(locals);

  const speciesId = Number(params.speciesId);
  if (!Number.isInteger(speciesId) || speciesId <= 0) {
    return json({ found: false, error: 'invalid speciesId' });
  }

  const [base] = await db.select({
    id:            pokemon.id,
    name:          pokemon.name,
    height:        pokemon.height,
    weight:        pokemon.weight,
    hp:            pokemon.hp,
    atk:           pokemon.atk,
    def:           pokemon.def,
    spa:           pokemon.spa,
    spd:           pokemon.spd,
    spe:           pokemon.spe,
    spriteDefault: pokemon.spriteDefault,
    spriteShiny:   pokemon.spriteShiny,
    spriteArtwork: pokemon.spriteArtwork,
  })
    .from(pokemon)
    .where(and(eq(pokemon.speciesId, speciesId), eq(pokemon.isDefault, true)))
    .limit(1);

  if (!base) return json({ found: false });

  const typeRows = await db.select({ slot: pokemonTypes.slot, typeId: pokemonTypes.typeId })
    .from(pokemonTypes)
    .where(eq(pokemonTypes.pokemonId, base.id))
    .orderBy(asc(pokemonTypes.slot));

  const abilityRows = await db.select({
    abilityId: pokemonAbilities.abilityId,
    slot:      pokemonAbilities.slot,
    isHidden:  pokemonAbilities.isHidden,
  })
    .from(pokemonAbilities)
    .where(eq(pokemonAbilities.pokemonId, base.id))
    .orderBy(asc(pokemonAbilities.slot));

  return json({
    found: true,
    pokemon: {
      name:          base.name,
      height:        base.height,
      weight:        base.weight,
      hp:            base.hp,
      atk:           base.atk,
      def:           base.def,
      spa:           base.spa,
      spd:           base.spd,
      spe:           base.spe,
      spriteDefault: base.spriteDefault,
      spriteShiny:   base.spriteShiny,
      spriteArtwork: base.spriteArtwork,
    },
    types:     typeRows,
    abilities: abilityRows,
  });
};
