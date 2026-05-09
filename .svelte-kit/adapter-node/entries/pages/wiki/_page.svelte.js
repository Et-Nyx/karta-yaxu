import { o as head } from "../../../chunks/dev.js";
//#region src/routes/wiki/+page.svelte
function _page($$renderer) {
	head("15drtgh", $$renderer, ($$renderer) => {
		$$renderer.title(($$renderer) => {
			$$renderer.push(`<title>Wiki | Karta Yaxu</title>`);
		});
	});
	$$renderer.push(`<div class="wiki-home svelte-15drtgh"><div class="header-section svelte-15drtgh"><h2 class="svelte-15drtgh">WIKI / DATABASE</h2> <p class="svelte-15drtgh">Explore the Karta Yaxu knowledge base. Data is strictly read-only on this section.</p></div> <div class="menu-grid svelte-15drtgh"><a href="/wiki/pokemon" class="menu-card svelte-15drtgh"><div class="icon svelte-15drtgh">🐾</div> <h3 class="svelte-15drtgh">Pokédex</h3> <p class="svelte-15drtgh">Browse all Pokémon, their stats, types, and evolutions.</p></a> <a href="/wiki/moves" class="menu-card svelte-15drtgh"><div class="icon svelte-15drtgh">⚔️</div> <h3 class="svelte-15drtgh">Moves</h3> <p class="svelte-15drtgh">Explore attacks, status moves, and their detailed mechanics.</p></a> <a href="/wiki/abilities" class="menu-card svelte-15drtgh"><div class="icon svelte-15drtgh">✨</div> <h3 class="svelte-15drtgh">Abilities</h3> <p class="svelte-15drtgh">List of all passive abilities and their effects in battle.</p></a> <a href="/wiki/types" class="menu-card svelte-15drtgh"><div class="icon svelte-15drtgh">🔥</div> <h3 class="svelte-15drtgh">Types</h3> <p class="svelte-15drtgh">Type match-ups, weaknesses, and resistances.</p></a></div></div>`);
}
//#endregion
export { _page as default };
