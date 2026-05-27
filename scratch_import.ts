import { importPokemonById } from './src/lib/pokeapi/import-pokemon.ts';

async function run() {
  try {
    console.log("Attempting to import Meltan #808...");
    await importPokemonById(808);
    console.log("Meltan import call finished!");
  } catch (e) {
    console.error("Meltan import failed with error:", e);
  }

  try {
    console.log("\nAttempting to import Melmetal #809...");
    await importPokemonById(809);
    console.log("Melmetal import call finished!");
  } catch (e) {
    console.error("Melmetal import failed with error:", e);
  }
}

run();
