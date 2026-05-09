import { h as head, c as escape_html, i as attr_style, j as stringify, e as ensure_array_like, a as attr, f as derived } from './dev-Br_spHyP.js';
import { T as TypeBadge } from './TypeBadge-B5SmRQYc.js';
import { P as PokemonCard } from './PokemonCard-DHzEhfG1.js';
import { T as TypeIcon } from './TypeIcon-B6kcdt8A.js';

//#region src/routes/wiki/types/[slug]/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		const t = data.typeData;
		const doubleDamageTo = data.attackingList.filter((x) => x.multiplier === 2);
		const halfDamageTo = data.attackingList.filter((x) => x.multiplier === .5);
		const noDamageTo = data.attackingList.filter((x) => x.multiplier === 0);
		const doubleDamageFrom = data.defendingList.filter((x) => x.multiplier === 2);
		const halfDamageFrom = data.defendingList.filter((x) => x.multiplier === .5);
		const noDamageFrom = data.defendingList.filter((x) => x.multiplier === 0);
		let searchQuery = "";
		let filteredPokemon = derived(() => data.pokemonList.filter((p) => p.name.toLowerCase().includes(searchQuery.toLowerCase()) || p.pokeapiId.toString() === searchQuery));
		head("qr246h", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>${escape_html(t.name.toUpperCase())} Type | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="type-detail-page svelte-qr246h"><header class="detail-header svelte-qr246h"><div class="header-content svelte-qr246h">`);
		TypeIcon($$renderer, {
			typeName: t.name,
			size: "lg"
		});
		$$renderer.push(`<!----> <div class="header-title svelte-qr246h"><span class="type-label svelte-qr246h">${escape_html(t.name.toUpperCase())}</span> <h1 class="svelte-qr246h">TYPE</h1></div></div> <div class="header-bar svelte-qr246h"${attr_style(`background-color: var(--color-type-${stringify(t.name)})`)}></div></header> <section class="matchups-grid svelte-qr246h"><div class="matchup-box svelte-qr246h"><h3 class="svelte-qr246h">OFFENSIVE (ATTACKING)</h3> <div class="matchup-row svelte-qr246h"><span class="label super svelte-qr246h">SUPER EFFECTIVE (2x):</span> <div class="badges svelte-qr246h"><!--[-->`);
		const each_array = ensure_array_like(doubleDamageTo);
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let m = each_array[$$index];
			TypeBadge($$renderer, {
				name: m.defenderName,
				size: "sm"
			});
		}
		$$renderer.push(`<!--]--> `);
		if (doubleDamageTo.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<span class="none svelte-qr246h">None</span>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div></div> <div class="matchup-row svelte-qr246h"><span class="label not-very svelte-qr246h">NOT VERY EFFECTIVE (0.5x):</span> <div class="badges svelte-qr246h"><!--[-->`);
		const each_array_1 = ensure_array_like(halfDamageTo);
		for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
			let m = each_array_1[$$index_1];
			TypeBadge($$renderer, {
				name: m.defenderName,
				size: "sm"
			});
		}
		$$renderer.push(`<!--]--> `);
		if (halfDamageTo.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<span class="none svelte-qr246h">None</span>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div></div> <div class="matchup-row svelte-qr246h"><span class="label immune svelte-qr246h">NO EFFECT (0x):</span> <div class="badges svelte-qr246h"><!--[-->`);
		const each_array_2 = ensure_array_like(noDamageTo);
		for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
			let m = each_array_2[$$index_2];
			TypeBadge($$renderer, {
				name: m.defenderName,
				size: "sm"
			});
		}
		$$renderer.push(`<!--]--> `);
		if (noDamageTo.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<span class="none svelte-qr246h">None</span>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div></div></div> <div class="matchup-box svelte-qr246h"><h3 class="svelte-qr246h">DEFENSIVE (DEFENDING)</h3> <div class="matchup-row svelte-qr246h"><span class="label super svelte-qr246h">WEAK TO (2x):</span> <div class="badges svelte-qr246h"><!--[-->`);
		const each_array_3 = ensure_array_like(doubleDamageFrom);
		for (let $$index_3 = 0, $$length = each_array_3.length; $$index_3 < $$length; $$index_3++) {
			let m = each_array_3[$$index_3];
			TypeBadge($$renderer, {
				name: m.attackerName,
				size: "sm"
			});
		}
		$$renderer.push(`<!--]--> `);
		if (doubleDamageFrom.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<span class="none svelte-qr246h">None</span>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div></div> <div class="matchup-row svelte-qr246h"><span class="label not-very svelte-qr246h">RESISTS (0.5x):</span> <div class="badges svelte-qr246h"><!--[-->`);
		const each_array_4 = ensure_array_like(halfDamageFrom);
		for (let $$index_4 = 0, $$length = each_array_4.length; $$index_4 < $$length; $$index_4++) {
			let m = each_array_4[$$index_4];
			TypeBadge($$renderer, {
				name: m.attackerName,
				size: "sm"
			});
		}
		$$renderer.push(`<!--]--> `);
		if (halfDamageFrom.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<span class="none svelte-qr246h">None</span>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div></div> <div class="matchup-row svelte-qr246h"><span class="label immune svelte-qr246h">IMMUNE TO (0x):</span> <div class="badges svelte-qr246h"><!--[-->`);
		const each_array_5 = ensure_array_like(noDamageFrom);
		for (let $$index_5 = 0, $$length = each_array_5.length; $$index_5 < $$length; $$index_5++) {
			let m = each_array_5[$$index_5];
			TypeBadge($$renderer, {
				name: m.attackerName,
				size: "sm"
			});
		}
		$$renderer.push(`<!--]--> `);
		if (noDamageFrom.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<span class="none svelte-qr246h">None</span>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div></div></div></section> <section class="pokemon-section svelte-qr246h"><div class="section-header svelte-qr246h"><h2 class="svelte-qr246h">POKÉMON OF THIS TYPE</h2> <div class="search-bar svelte-qr246h"><input type="text"${attr("value", searchQuery)} placeholder="Filter Pokémon..." class="search-input svelte-qr246h"/> <div class="results-count svelte-qr246h">${escape_html(filteredPokemon().length)} / ${escape_html(data.pokemonList.length)}</div></div></div> <div class="pokemon-grid svelte-qr246h"><!--[-->`);
		const each_array_6 = ensure_array_like(filteredPokemon());
		for (let $$index_6 = 0, $$length = each_array_6.length; $$index_6 < $$length; $$index_6++) {
			let p = each_array_6[$$index_6];
			PokemonCard($$renderer, { pokemon: {
				pokeapiId: p.pokeapiId || 0,
				name: p.name,
				slug: p.slug,
				spriteUrl: p.spriteDefault,
				types: p.types
			} });
		}
		$$renderer.push(`<!--]--></div></section></div>`);
	});
}

export { _page as default };
//# sourceMappingURL=_page.svelte-UKDLQyy1.js.map
