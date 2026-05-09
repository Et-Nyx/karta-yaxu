import { B as attr, V as escape_html, a as ensure_array_like, i as derived, l as stringify, o as head } from "../../../../chunks/dev.js";
//#region src/routes/wiki/abilities/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		let searchQuery = "";
		let filteredAbilities = derived(() => data.abilities.filter((a) => a.name.toLowerCase().includes(searchQuery.toLowerCase())));
		head("c06j6m", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>Abilities | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="abilities-page svelte-c06j6m"><div class="page-header svelte-c06j6m"><h2 class="svelte-c06j6m">ABILITIES</h2> <div class="search-bar svelte-c06j6m"><input type="text"${attr("value", searchQuery)} placeholder="Search by name... (/)" class="search-input svelte-c06j6m"/> <div class="results-count svelte-c06j6m">${escape_html(filteredAbilities().length)} RESULTS</div></div></div> <div class="abilities-list svelte-c06j6m"><!--[-->`);
		const each_array = ensure_array_like(filteredAbilities());
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let a = each_array[$$index];
			$$renderer.push(`<a${attr("href", `/wiki/abilities/${stringify(a.slug)}`)} class="ability-card card-interactive svelte-c06j6m"><div class="ability-name svelte-c06j6m">${escape_html(a.name.toUpperCase().replace("-", " "))}</div></a>`);
		}
		$$renderer.push(`<!--]--></div></div>`);
	});
}
//#endregion
export { _page as default };
