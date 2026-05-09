import { a as attr, j as stringify, c as escape_html, e as ensure_array_like, f as derived } from './dev-Br_spHyP.js';
import { T as TypeBadge } from './TypeBadge-B5SmRQYc.js';

//#region src/lib/components/PokemonCard.svelte
function PokemonCard($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { pokemon } = $$props;
		let displayName = derived(() => pokemon.name ?? "???");
		let displaySlug = derived(() => pokemon.slug ?? "unknown");
		let displayTypes = derived(() => pokemon.types ?? []);
		let imageUrl = derived(() => pokemon.spriteUrl || `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/${pokemon.pokeapiId || 0}.png`);
		let dexNum = derived(() => pokemon.pokeapiId?.toString().padStart(3, "0") ?? "???");
		$$renderer.push(`<a${attr("href", `/wiki/pokemon/${stringify(displaySlug())}`)} class="pokemon-card card-interactive svelte-3cep7v"><div class="card-header svelte-3cep7v"><div class="dex-number svelte-3cep7v">#${escape_html(dexNum())}</div> <div class="name svelte-3cep7v">${escape_html(displayName().toUpperCase())}</div></div> <div class="sprite-container svelte-3cep7v"><img${attr("src", imageUrl())}${attr("alt", displayName())} loading="lazy" class="svelte-3cep7v"/></div> <div class="types-container svelte-3cep7v"><!--[-->`);
		const each_array = ensure_array_like(displayTypes());
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let typeName = each_array[$$index];
			TypeBadge($$renderer, {
				name: typeName,
				size: "sm",
				class: "type-badge-flex"
			});
		}
		$$renderer.push(`<!--]--></div></a>`);
	});
}

export { PokemonCard as P };
//# sourceMappingURL=PokemonCard-DHzEhfG1.js.map
