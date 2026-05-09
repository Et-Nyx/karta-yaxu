import { h as head, e as ensure_array_like, c as escape_html, i as attr_style, a as attr, b as attr_class, j as stringify } from './dev-Br_spHyP.js';
import './client-Co30xGW6.js';
import './internal-DRKVZF4f.js';
import './chunk-BBx_TEkp.js';
import './index-DBqjc0Yf.js';

//#region src/lib/components/ImportStatusGrid.svelte
function ImportStatusGrid($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { status = {} } = $$props;
		const ENTITIES = [
			{
				key: "languages",
				label: "Languages",
				max: 13
			},
			{
				key: "regions",
				label: "Regions",
				max: 10
			},
			{
				key: "generations",
				label: "Generations",
				max: 9
			},
			{
				key: "types",
				label: "Types",
				max: 18
			},
			{
				key: "abilities",
				label: "Abilities",
				max: 307
			},
			{
				key: "moves",
				label: "Moves",
				max: 919
			},
			{
				key: "items",
				label: "Items",
				max: 2110
			},
			{
				key: "pokemon",
				label: "Pokémon",
				max: 1025
			}
		];
		function getPercentage(count, max) {
			if (!max || max === 0) return 0;
			return Math.min(100, Math.round(count / max * 100));
		}
		$$renderer.push(`<div class="status-grid svelte-1mlpm3p"><!--[-->`);
		const each_array = ensure_array_like(ENTITIES);
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let entity = each_array[$$index];
			const counts = status[entity.key] || {
				total: 0,
				pokeapi: 0,
				edited: 0,
				custom: 0
			};
			const pct = getPercentage(counts.total, entity.max);
			$$renderer.push(`<div class="status-card svelte-1mlpm3p"><div class="card-header svelte-1mlpm3p"><span class="entity-name svelte-1mlpm3p">${escape_html(entity.label)}</span> <span class="entity-count svelte-1mlpm3p">${escape_html(counts.total)} / ${escape_html(entity.max)}</span></div> <div class="progress-track svelte-1mlpm3p"><div class="progress-fill svelte-1mlpm3p"${attr_style(`width: ${stringify(pct)}%`)}></div></div> <div class="source-breakdown svelte-1mlpm3p"><div class="source-item svelte-1mlpm3p"><span class="dot pokeapi svelte-1mlpm3p"></span> <span>API: ${escape_html(counts.pokeapi)}</span></div> <div class="source-item svelte-1mlpm3p"><span class="dot edited svelte-1mlpm3p"></span> <span>Edit: ${escape_html(counts.edited)}</span></div> <div class="source-item svelte-1mlpm3p"><span class="dot custom svelte-1mlpm3p"></span> <span>Cust: ${escape_html(counts.custom)}</span></div></div></div>`);
		}
		$$renderer.push(`<!--]--></div>`);
	});
}
//#endregion
//#region src/lib/components/ImportButton.svelte
function ImportButton($$renderer, $$props) {
	let { action, label, loadingLabel = "IMPORTING...", disabled = false } = $$props;
	let state = "idle";
	$$renderer.push(`<div class="import-button-container svelte-j8osk3"><form method="POST"${attr("action", action)}><button type="submit"${attr_class("btn svelte-j8osk3", void 0, {
		"loading": state === "loading",
		"success": state === "success",
		"error": state === "error"
	})}${attr("disabled", disabled || state === "loading", true)}>`);
	{
		$$renderer.push("<!--[-1-->");
		$$renderer.push(`${escape_html(label)}`);
	}
	$$renderer.push(`<!--]--></button></form> `);
	$$renderer.push("<!--[-1-->");
	$$renderer.push(`<!--]--></div>`);
}
//#endregion
//#region src/lib/components/ImportLog.svelte
function ImportLog($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { logs = [] } = $$props;
		function formatTime(date) {
			if (!date) return "";
			return new Date(date).toLocaleTimeString("en-US", { hour12: false });
		}
		$$renderer.push(`<div class="log-container svelte-19koy0x"><div class="log-header svelte-19koy0x"><h3 class="svelte-19koy0x">IMPORT LOG (LATEST 50)</h3></div> <div class="log-body svelte-19koy0x">`);
		if (logs.length === 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="empty-state svelte-19koy0x">No import activity recorded yet.</div>`);
		} else {
			$$renderer.push("<!--[-1-->");
			$$renderer.push(`<table class="log-table svelte-19koy0x"><thead><tr><th class="svelte-19koy0x">TIME</th><th class="svelte-19koy0x">ENTITY</th><th class="svelte-19koy0x">ID</th><th class="svelte-19koy0x">ACTION</th><th class="svelte-19koy0x">REASON</th></tr></thead><tbody><!--[-->`);
			const each_array = ensure_array_like(logs);
			for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
				let log = each_array[$$index];
				$$renderer.push(`<tr${attr_class(`action-${stringify(log.action)}`, "svelte-19koy0x")}><td class="time svelte-19koy0x">${escape_html(formatTime(log.timestamp))}</td><td class="entity svelte-19koy0x">${escape_html(log.entityType)}</td><td class="id svelte-19koy0x">#${escape_html(log.pokeapiId)}</td><td class="action svelte-19koy0x"><span${attr_class(`badge ${stringify(log.action)}`, "svelte-19koy0x")}>${escape_html(log.action.toUpperCase())}</span></td><td class="reason svelte-19koy0x">${escape_html(log.reason || "-")}</td></tr>`);
			}
			$$renderer.push(`<!--]--></tbody></table>`);
		}
		$$renderer.push(`<!--]--></div></div>`);
	});
}
//#endregion
//#region src/routes/import/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		let liveLogs = data.logs;
		head("1kumcmu", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>Import | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="import-page svelte-1kumcmu"><div class="page-header svelte-1kumcmu"><h2 class="svelte-1kumcmu">PokeAPI Import Dashboard</h2> <p class="svelte-1kumcmu">Database seeded on-demand. Note that manual edits are protected from overwriting.</p></div> <section class="status-section">`);
		ImportStatusGrid($$renderer, { status: data.status });
		$$renderer.push(`<!----></section> <div class="actions-grid svelte-1kumcmu"><section class="action-panel svelte-1kumcmu"><h3 class="svelte-1kumcmu">1. Game Metadata</h3> <p class="svelte-1kumcmu">Imports Languages, Regions, Generations, Versions, and Pokedexes.</p> `);
		ImportButton($$renderer, {
			action: "?/importGameData",
			label: "Import All Game Data"
		});
		$$renderer.push(`<!----></section> <section class="action-panel svelte-1kumcmu"><h3 class="svelte-1kumcmu">2. Core Data</h3> <p class="svelte-1kumcmu">Imports individual core data collections.</p> <div class="btn-group svelte-1kumcmu">`);
		ImportButton($$renderer, {
			action: "?/importTypes",
			label: "Types"
		});
		$$renderer.push(`<!----> `);
		ImportButton($$renderer, {
			action: "?/importAbilities",
			label: "Abilities"
		});
		$$renderer.push(`<!----> `);
		ImportButton($$renderer, {
			action: "?/importMoves",
			label: "Moves"
		});
		$$renderer.push(`<!----> `);
		ImportButton($$renderer, {
			action: "?/importItems",
			label: "Items"
		});
		$$renderer.push(`<!----></div></section> <section class="action-panel svelte-1kumcmu"><h3 class="svelte-1kumcmu">3. Pokémon</h3> <p class="svelte-1kumcmu">Import Pokémon with full dependency resolution (types, abilities, moves, evolution chain).</p> <div class="form-row svelte-1kumcmu"><form method="POST" action="?/importSinglePokemon" class="inline-form svelte-1kumcmu"><label for="pokeapiId" class="svelte-1kumcmu">Single ID:</label> <input type="number" id="pokeapiId" name="pokeapiId" placeholder="25" required="" min="1" class="svelte-1kumcmu"/> <button type="submit" class="submit-btn svelte-1kumcmu">Import Single</button></form></div> <div class="form-row svelte-1kumcmu"><form method="POST" action="?/importRangePokemon" class="inline-form svelte-1kumcmu"><label class="svelte-1kumcmu">Range:</label> <input type="number" name="fromId" placeholder="1" required="" min="1" class="svelte-1kumcmu"/> <span class="svelte-1kumcmu">to</span> <input type="number" name="toId" placeholder="151" required="" min="1" class="svelte-1kumcmu"/> <button type="submit" class="submit-btn svelte-1kumcmu">Import Range</button></form></div></section></div> <section class="log-section svelte-1kumcmu">`);
		ImportLog($$renderer, { logs: liveLogs });
		$$renderer.push(`<!----></section></div>`);
	});
}

export { _page as default };
//# sourceMappingURL=_page.svelte-CQXUOrl3.js.map
