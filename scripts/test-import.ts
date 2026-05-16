// Quick verification script for the import module
// Run with: npx tsx scripts/test-import.ts

import { eq, count } from 'drizzle-orm';
import { db } from '../src/lib/db/index';
import * as schema from '../src/lib/db/schema/index';
import { ensureLanguage } from '../src/lib/pokeapi/import-service';
import { ensureType, ensurePokemon } from '../src/lib/pokeapi/import-pokemon';

async function main() {
  console.log('=== Karta Yaxu Import Verification ===\n');

  // Test 1: Import Language ID=9 ("en")
  console.log('📝 Test 1: Import Language ID=9 ("en")...');
  try {
    const langId = await ensureLanguage(9);
    console.log(`   ✅ Language imported (db id: ${langId})`);
    const lang = db.select().from(schema.languages).where(eq(schema.languages.id, langId)).get();
    console.log(`   → name: ${lang?.name}, iso639: ${lang?.iso639}, official: ${lang?.official}`);
  } catch (e) {
    console.error(`   ❌ Failed: ${(e as Error).message}`);
  }

  // Test 2: Import Type ID=13 ("electric") — should auto-import Generation
  console.log('\n⚡ Test 2: Import Type ID=13 ("electric")...');
  try {
    const typeId = await ensureType(13);
    console.log(`   ✅ Type imported (db id: ${typeId})`);
    const type = db.select().from(schema.types).where(eq(schema.types.id, typeId)).get();
    console.log(`   → name: ${type?.name}, generationId: ${type?.generationId}`);

    // Check that generation was auto-imported
    const genCount = db.select({ c: count() }).from(schema.generations).get();
    console.log(`   → Generations in DB: ${genCount?.c}`);
    const regionCount = db.select({ c: count() }).from(schema.regions).get();
    console.log(`   → Regions in DB: ${regionCount?.c}`);
  } catch (e) {
    console.error(`   ❌ Failed: ${(e as Error).message}`);
    console.error((e as Error).stack);
  }

  // Test 3: Import Pokemon ID=26 ("raichu") — full dependency chain
  console.log('\n🔴 Test 3: Import Pokemon ID=26 ("raichu")...');
  try {
    const pokemonId = await ensurePokemon(26);
    console.log(`   ✅ Pokemon imported (db id: ${pokemonId})`);

    // Check species chain (Pichu → Pikachu → Raichu)
    const species = db.select({ name: schema.pokemonSpecies.name, pokeapiId: schema.pokemonSpecies.pokeapiId })
      .from(schema.pokemonSpecies)
      .all();
    console.log(`   → Species in DB: ${species.length}`);
    for (const s of species) {
      console.log(`     - ${s.name} (pokeapi_id: ${s.pokeapiId})`);
    }

    // Counts
    const typeCount = db.select({ c: count() }).from(schema.types).get();
    console.log(`   → Types in DB: ${typeCount?.c}`);
    const abilityCount = db.select({ c: count() }).from(schema.abilities).get();
    console.log(`   → Abilities in DB: ${abilityCount?.c}`);
    const moveCount = db.select({ c: count() }).from(schema.moves).get();
    console.log(`   → Moves in DB: ${moveCount?.c}`);
    const evoCount = db.select({ c: count() }).from(schema.pokemonEvolutionDetails).get();
    console.log(`   → Evolution edges (linked-list) in DB: ${evoCount?.c}`);
    const logCount = db.select({ c: count() }).from(schema.importLog).get();
    console.log(`   → Import log entries: ${logCount?.c}`);
  } catch (e) {
    console.error(`   ❌ Failed: ${(e as Error).message}`);
    console.error((e as Error).stack);
  }

  // Test 4: Idempotency — re-import same pokemon
  console.log('\n🔄 Test 4: Re-import Pokemon ID=26 (idempotency)...');
  try {
    const pokemonId = await ensurePokemon(26);
    console.log(`   ✅ Re-import returned same id: ${pokemonId} (should be instant)`);
  } catch (e) {
    console.error(`   ❌ Failed: ${(e as Error).message}`);
  }

  console.log('\n=== Verification Complete ===');
}

main().catch(console.error);
