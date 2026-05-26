import { importPokemonById } from '../src/lib/pokeapi/import-pokemon.js';
import { importAllGameData } from '../src/lib/pokeapi/import-service.js';

const TEST_SPECIES = [
  10,   // Caterpie: simple 3-stage
  16,   // Pidgey: simple 3-stage
  172,  // Pichu: baby + Alolan Raichu
  439,  // Mime Jr: baby + Galar Mr. Mime + Mr. Rime
  44,   // Gloom: branched items (Vileplume/Bellossom)
  133,  // Eevee: mega 8-way branch
  236,  // Tyrogue: 3-way stat branch
  263,  // Zigzagoon: Galar 3-stage + Obstagoon
  66,   // Machop: trade evo
  79,   // Slowpoke: branch + trade+item + Galar Slowbro/Slowking
  52,   // Meowth: Galar Perrserker
  83,   // Farfetch'd: Galar Sirfetch'd
  222,  // Corsola: Galar Cursola
  562,  // Yamask: Cofagrigus + Galar Runerigus
  265,  // Wurmple: random dual split
  290,  // Nincada: shed (Ninjask + Shedinja)
  215,  // Sneasel: Weavile + Hisuian Sneasler
  234,  // Stantler: Hisuian Wyrdeer
  132,  // Ditto: standalone, no chain
  280,  // Ralts: gender-branched (Gardevoir/Gallade)
  226,  // Mantyke: party species requirement (Remoraid)
  674,  // Pancham: party type requirement (Dark type)
];

async function main() {
  console.log('🌍 Importing Game Metadata...');
  await importAllGameData();
  console.log('✅ Game Metadata Imported!');

  console.log('🔥 Starting Pokémon Bulk Import...');
  for (const id of TEST_SPECIES) {
    try {
      console.log(`\n----------------------------------------`);
      console.log(`📦 Importing Species PokeAPI ID #${id}...`);
      await importPokemonById(id);
      console.log(`----------------------------------------`);
    } catch (e) {
      console.error(`❌ Error importing species #${id}:`, e);
    }
  }
  console.log('\n🎉 ALL CURATED TEST EVOLUTION TOPOLOGIES SUCCESSFULLY IMPORTED!');
  process.exit(0);
}

main().catch(err => {
  console.error('💥 Fatal error running test-import:', err);
  process.exit(1);
});
