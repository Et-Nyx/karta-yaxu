import { B as attr, V as escape_html, a as ensure_array_like, i as derived, l as stringify, o as head } from "../../../../chunks/dev.js";
import { t as TypeBadge } from "../../../../chunks/TypeBadge.js";
import { t as MoveCategoryIcon } from "../../../../chunks/MoveCategoryIcon.js";
//#region src/routes/wiki/moves/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		let searchQuery = "";
		let filteredMoves = derived(() => data.moves.filter((m) => m.name.toLowerCase().includes(searchQuery.toLowerCase()) || m.typeName && m.typeName.toLowerCase().includes(searchQuery.toLowerCase())));
		head("1toueju", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>Moves | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="moves-page svelte-1toueju"><div class="page-header svelte-1toueju"><h2 class="svelte-1toueju">MOVES</h2> <div class="search-bar svelte-1toueju"><input type="text"${attr("value", searchQuery)} placeholder="Search by name or type... (/)" class="search-input svelte-1toueju"/> <div class="results-count svelte-1toueju">${escape_html(filteredMoves().length)} RESULTS</div></div></div> <div class="moves-list svelte-1toueju"><!--[-->`);
		const each_array = ensure_array_like(filteredMoves());
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let m = each_array[$$index];
			$$renderer.push(`<a${attr("href", `/wiki/moves/${stringify(m.slug)}`)} class="move-card card-interactive svelte-1toueju"><div class="move-info svelte-1toueju"><div class="move-name svelte-1toueju">${escape_html(m.name.toUpperCase())}</div> `);
			MoveCategoryIcon($$renderer, {
				category: m.damageClass || "physical",
				size: "md"
			});
			$$renderer.push(`<!----> `);
			if (m.typeName) {
				$$renderer.push("<!--[0-->");
				TypeBadge($$renderer, {
					name: m.typeName,
					size: "sm"
				});
			} else $$renderer.push("<!--[-1-->");
			$$renderer.push(`<!--]--></div> <div class="move-stats svelte-1toueju"><div class="stat svelte-1toueju"><span class="label svelte-1toueju">PWR</span> <span class="value svelte-1toueju">${escape_html(m.power !== null ? m.power : "-")}</span></div> <div class="stat svelte-1toueju"><span class="label svelte-1toueju">ACC</span> <span class="value svelte-1toueju">${escape_html(m.accuracy !== null ? m.accuracy + "%" : "-")}</span></div> <div class="stat svelte-1toueju"><span class="label svelte-1toueju">PP</span> <span class="value svelte-1toueju">${escape_html(m.pp !== null ? m.pp : "-")}</span></div></div></a>`);
		}
		$$renderer.push(`<!--]--></div></div>`);
	});
}
//#endregion
export { _page as default };
