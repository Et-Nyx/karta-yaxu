import { h as head, c as escape_html, a as attr, e as ensure_array_like, f as derived } from './dev-Br_spHyP.js';
import { T as TypeBadge } from './TypeBadge-B5SmRQYc.js';
import { P as PokemonCard } from './PokemonCard-DHzEhfG1.js';
import { M as MoveCategoryIcon } from './MoveCategoryIcon-0og4pQTo.js';

//#region src/routes/wiki/moves/[slug]/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		const m = data.move;
		let searchQuery = "";
		let filteredLearners = derived(() => data.learners.filter((p) => p.name.toLowerCase().includes(searchQuery.toLowerCase()) || p.pokeapiId.toString() === searchQuery));
		head("kpqjk4", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>${escape_html(m.name.toUpperCase())} | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="move-detail-page svelte-kpqjk4"><header class="detail-header svelte-kpqjk4"><div class="title-group svelte-kpqjk4"><h1 class="svelte-kpqjk4">${escape_html(m.name.toUpperCase())}</h1> `);
		MoveCategoryIcon($$renderer, {
			category: m.damageClass || "physical",
			size: "lg"
		});
		$$renderer.push(`<!----> `);
		if (m.typeName) {
			$$renderer.push("<!--[0-->");
			TypeBadge($$renderer, {
				name: m.typeName,
				size: "lg"
			});
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div> <div class="stats-row svelte-kpqjk4"><div class="stat svelte-kpqjk4"><span class="label svelte-kpqjk4">POWER</span> <span class="value svelte-kpqjk4">${escape_html(m.power !== null ? m.power : "-")}</span></div> <div class="stat svelte-kpqjk4"><span class="label svelte-kpqjk4">ACCURACY</span> <span class="value svelte-kpqjk4">${escape_html(m.accuracy !== null ? m.accuracy + "%" : "-")}</span></div> <div class="stat svelte-kpqjk4"><span class="label svelte-kpqjk4">PP</span> <span class="value svelte-kpqjk4">${escape_html(m.pp !== null ? m.pp : "-")}</span></div> <div class="stat svelte-kpqjk4"><span class="label svelte-kpqjk4">PRIORITY</span> <span class="value svelte-kpqjk4">${escape_html(m.priority)}</span></div> <div class="stat svelte-kpqjk4"><span class="label svelte-kpqjk4">TARGET</span> <span class="value svelte-kpqjk4">${escape_html(m.target ? m.target.toUpperCase().replace("-", " ") : "-")}</span></div></div></header> <section class="learners-section svelte-kpqjk4"><div class="section-header svelte-kpqjk4"><h2 class="svelte-kpqjk4">LEARNED BY</h2> <div class="search-bar svelte-kpqjk4"><input type="text"${attr("value", searchQuery)} placeholder="Filter Pokémon..." class="search-input svelte-kpqjk4"/> <div class="results-count svelte-kpqjk4">${escape_html(filteredLearners().length)} / ${escape_html(data.learners.length)}</div></div></div> <div class="pokemon-grid svelte-kpqjk4"><!--[-->`);
		const each_array = ensure_array_like(filteredLearners());
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
		if (filteredLearners().length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="empty-state svelte-kpqjk4"><p>No Pokémon found.</p></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></section></div>`);
	});
}

export { _page as default };
//# sourceMappingURL=_page.svelte-CtqbJaZz.js.map
