import { B as attr, V as escape_html, a as ensure_array_like, i as derived, n as attr_class, o as head } from "../../../../../chunks/dev.js";
import { t as PokemonCard } from "../../../../../chunks/PokemonCard.js";
//#region src/routes/wiki/abilities/[slug]/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		const a = data.ability;
		let searchQuery = "";
		let filteredPokemon = derived(() => data.pokemonList.filter((p) => p.name.toLowerCase().includes(searchQuery.toLowerCase()) || p.pokeapiId.toString() === searchQuery));
		head("ao07z8", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>${escape_html(a.name.toUpperCase().replace("-", " "))} | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="ability-detail-page svelte-ao07z8"><header class="detail-header svelte-ao07z8"><h1 class="svelte-ao07z8">${escape_html(a.name.toUpperCase().replace("-", " "))}</h1></header> <section class="pokemon-section svelte-ao07z8"><div class="section-header svelte-ao07z8"><h2 class="svelte-ao07z8">POKÉMON WITH THIS ABILITY</h2> <div class="search-bar svelte-ao07z8"><input type="text"${attr("value", searchQuery)} placeholder="Filter Pokémon..." class="search-input svelte-ao07z8"/> <div class="results-count svelte-ao07z8">${escape_html(filteredPokemon().length)} / ${escape_html(data.pokemonList.length)}</div></div></div> <div class="pokemon-grid svelte-ao07z8"><!--[-->`);
		const each_array = ensure_array_like(filteredPokemon());
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let p = each_array[$$index];
			$$renderer.push(`<div${attr_class("card-wrapper svelte-ao07z8", void 0, { "hidden-ability": p.isHidden })}>`);
			if (p.isHidden) {
				$$renderer.push("<!--[0-->");
				$$renderer.push(`<div class="hidden-badge svelte-ao07z8">HIDDEN</div>`);
			} else $$renderer.push("<!--[-1-->");
			$$renderer.push(`<!--]--> `);
			PokemonCard($$renderer, { pokemon: {
				pokeapiId: p.pokeapiId || 0,
				name: p.name,
				slug: p.slug,
				spriteUrl: p.spriteDefault,
				types: p.types
			} });
			$$renderer.push(`<!----></div>`);
		}
		$$renderer.push(`<!--]--></div> `);
		if (filteredPokemon().length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="empty-state svelte-ao07z8"><p>No Pokémon found.</p></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></section></div>`);
	});
}
//#endregion
export { _page as default };
