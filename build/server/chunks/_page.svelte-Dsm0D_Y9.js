import { h as head, a as attr, c as escape_html, e as ensure_array_like, f as derived } from './dev-Br_spHyP.js';
import { P as PokemonCard } from './PokemonCard-DHzEhfG1.js';
import './TypeBadge-B5SmRQYc.js';

//#region src/routes/wiki/pokemon/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		let searchQuery = "";
		let filteredPokemon = derived(() => data.pokemon.filter((p) => p.name.toLowerCase().includes(searchQuery.toLowerCase()) || p.pokeapiId.toString() === searchQuery));
		head("1it32e3", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>Pokédex | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="pokedex-page svelte-1it32e3"><div class="page-header svelte-1it32e3"><h2 class="svelte-1it32e3">POKÉDEX</h2> <div class="search-bar svelte-1it32e3"><input type="text"${attr("value", searchQuery)} placeholder="Search by name or ID... (/)" class="search-input svelte-1it32e3"/> <div class="results-count svelte-1it32e3">${escape_html(filteredPokemon().length)} RESULTS</div></div></div> <div class="pokemon-grid svelte-1it32e3"><!--[-->`);
		const each_array = ensure_array_like(filteredPokemon());
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let p = each_array[$$index];
			PokemonCard($$renderer, { pokemon: {
				pokeapiId: p.pokeapiId || 0,
				name: p.name,
				slug: p.slug,
				spriteUrl: p.spriteDefault,
				types: p.types
			} });
		}
		$$renderer.push(`<!--]--></div> `);
		if (filteredPokemon().length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="empty-state svelte-1it32e3"><p>No Pokémon found matching "${escape_html(searchQuery)}".</p> <p>Make sure you have imported data in the Admin panel.</p></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div>`);
	});
}

export { _page as default };
//# sourceMappingURL=_page.svelte-Dsm0D_Y9.js.map
