import { B as attr, V as escape_html, a as ensure_array_like, i as derived, l as stringify, n as attr_class, o as head, r as attr_style } from "../../../../../chunks/dev.js";
import { t as TypeBadge } from "../../../../../chunks/TypeBadge.js";
import { t as PokemonCard } from "../../../../../chunks/PokemonCard.js";
import { t as MoveCategoryIcon } from "../../../../../chunks/MoveCategoryIcon.js";
//#region src/lib/components/StatBar.svelte
function StatBar($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { name = "hp", value = 0, max = 255 } = $$props;
		let cssName = derived(() => () => {
			switch (name) {
				case "hp": return "hp";
				case "attack": return "atk";
				case "defense": return "def";
				case "special-attack": return "spa";
				case "special-defense": return "spd";
				case "speed": return "spe";
				default: return "hp";
			}
		});
		let percentage = derived(() => Math.min(100, value / max * 100));
		$$renderer.push(`<div class="stat-row svelte-o2gr0j"><div class="stat-label svelte-o2gr0j">${escape_html(name.replace("special-", "sp. ").toUpperCase())}</div> <div class="stat-value svelte-o2gr0j">${escape_html(value.toString().padStart(3, "0"))}</div> <div class="stat-bar-container svelte-o2gr0j"><div class="stat-bar-fill svelte-o2gr0j"${attr_style(`width: ${stringify(percentage())}%; background-color: var(--color-stat-${stringify(cssName()())});`)}></div></div></div>`);
	});
}
//#endregion
//#region src/lib/components/EvolutionTree.svelte
function EvolutionTree($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { nodes = [], edges = [] } = $$props;
		function buildTree(currentSpeciesId) {
			const node = nodes.find((n) => n.speciesId === currentSpeciesId);
			if (!node || node.pokeapiId == null || node.name == null) return null;
			const childrenEdges = edges.filter((e) => e.fromSpeciesId === currentSpeciesId);
			const groupedEdges = /* @__PURE__ */ new Map();
			for (const e of childrenEdges) if (!groupedEdges.has(e.toSpeciesId)) groupedEdges.set(e.toSpeciesId, e);
			const children = Array.from(groupedEdges.entries()).map(([toSpeciesId, edge]) => {
				const childTree = buildTree(toSpeciesId);
				if (!childTree) return null;
				return {
					...childTree,
					edges: [edge]
				};
			}).filter((c) => c !== null);
			return {
				...node,
				children
			};
		}
		let roots = derived(() => () => {
			const toIds = new Set(edges.map((e) => e.toSpeciesId));
			return nodes.filter((n) => !toIds.has(n.speciesId)).map((r) => buildTree(r.speciesId)).filter((r) => r !== null);
		});
		function getLayoutMode(childCount) {
			if (childCount === 0) return "single";
			if (childCount === 1) return "linear";
			if (childCount <= 4) return "branched";
			return "multi-row";
		}
		function chunkArray(arr, size) {
			const chunks = [];
			for (let i = 0; i < arr.length; i += size) chunks.push(arr.slice(i, i + size));
			return chunks;
		}
		function formatTrigger(trigger, minLevel) {
			const label = trigger.toUpperCase().replace(/-/g, " ");
			if (minLevel) return `${label} Lvl ${minLevel}`;
			return label;
		}
		function RenderNode($$renderer, tree) {
			$$renderer.push(`<div class="node-wrapper svelte-1lpc3bx"><div class="node-card-wrapper svelte-1lpc3bx">`);
			PokemonCard($$renderer, { pokemon: {
				pokeapiId: tree.pokeapiId,
				name: tree.name,
				slug: tree.slug,
				spriteUrl: tree.spriteDefault,
				types: tree.types || []
			} });
			$$renderer.push(`<!----></div> `);
			RenderConnections($$renderer, tree);
			$$renderer.push(`<!----></div>`);
		}
		function RenderConnections($$renderer, tree) {
			const childCount = tree.children.length;
			const layout = getLayoutMode(childCount);
			const rows = layout === "multi-row" ? chunkArray(tree.children, 4) : [];
			if (layout === "single") $$renderer.push("<!--[0-->");
			else if (layout === "linear") {
				$$renderer.push("<!--[1-->");
				$$renderer.push(`<div class="linear-connector svelte-1lpc3bx"><span class="arrow-down svelte-1lpc3bx">↓</span> <span class="trigger-label svelte-1lpc3bx">${escape_html(formatTrigger(tree.children[0].edges[0].trigger, tree.children[0].edges[0].minLevel))}</span></div> `);
				RenderNode($$renderer, tree.children[0]);
				$$renderer.push(`<!---->`);
			} else if (layout === "branched") {
				$$renderer.push("<!--[2-->");
				$$renderer.push(`<div class="branched-connector svelte-1lpc3bx"><div class="connector-vertical-short svelte-1lpc3bx"></div> <div class="horizontal-bar svelte-1lpc3bx"><!--[-->`);
				const each_array = ensure_array_like(tree.children);
				for (let $$index_1 = 0, $$length = each_array.length; $$index_1 < $$length; $$index_1++) {
					let child = each_array[$$index_1];
					$$renderer.push(`<div class="branch-position svelte-1lpc3bx"><span class="arrow-down svelte-1lpc3bx">↓</span> <div class="branch-trigger svelte-1lpc3bx"><span class="trigger-label svelte-1lpc3bx">${escape_html(formatTrigger(child.edges[0].trigger, child.edges[0].minLevel))}</span></div> <div class="branch-card svelte-1lpc3bx">`);
					PokemonCard($$renderer, { pokemon: {
						pokeapiId: child.pokeapiId,
						name: child.name,
						slug: child.slug,
						spriteUrl: child.spriteDefault,
						types: child.types || []
					} });
					$$renderer.push(`<!----> `);
					RenderConnections($$renderer, child);
					$$renderer.push(`<!----></div></div>`);
				}
				$$renderer.push(`<!--]--></div></div>`);
			} else {
				$$renderer.push("<!--[-1-->");
				$$renderer.push(`<div class="multi-row-connector svelte-1lpc3bx"><div class="connector-vertical-short svelte-1lpc3bx"></div> <!--[-->`);
				const each_array_1 = ensure_array_like(rows);
				for (let rowIndex = 0, $$length = each_array_1.length; rowIndex < $$length; rowIndex++) {
					let row = each_array_1[rowIndex];
					$$renderer.push(`<div class="row-section svelte-1lpc3bx"><div class="horizontal-bar svelte-1lpc3bx"><!--[-->`);
					const each_array_2 = ensure_array_like(row);
					for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
						let child = each_array_2[$$index_2];
						$$renderer.push(`<div class="branch-position svelte-1lpc3bx"><span class="arrow-down svelte-1lpc3bx">↓</span> <div class="branch-trigger svelte-1lpc3bx"><span class="trigger-label svelte-1lpc3bx">${escape_html(formatTrigger(child.edges[0].trigger, child.edges[0].minLevel))}</span></div> <div class="branch-card svelte-1lpc3bx">`);
						PokemonCard($$renderer, { pokemon: {
							pokeapiId: child.pokeapiId,
							name: child.name,
							slug: child.slug,
							spriteUrl: child.spriteDefault,
							types: child.types || []
						} });
						$$renderer.push(`<!----> `);
						RenderConnections($$renderer, child);
						$$renderer.push(`<!----></div></div>`);
					}
					$$renderer.push(`<!--]--></div> `);
					if (rowIndex < rows.length - 1) {
						$$renderer.push("<!--[0-->");
						$$renderer.push(`<div class="connector-vertical-between-rows svelte-1lpc3bx"></div>`);
					} else $$renderer.push("<!--[-1-->");
					$$renderer.push(`<!--]--></div>`);
				}
				$$renderer.push(`<!--]--></div>`);
			}
			$$renderer.push(`<!--]-->`);
		}
		$$renderer.push(`<div class="evolution-tree svelte-1lpc3bx"><!--[-->`);
		const each_array_3 = ensure_array_like(roots()());
		for (let $$index = 0, $$length = each_array_3.length; $$index < $$length; $$index++) {
			let rootTree = each_array_3[$$index];
			if (rootTree) {
				$$renderer.push("<!--[0-->");
				$$renderer.push(`<div class="tree-container svelte-1lpc3bx">`);
				RenderNode($$renderer, rootTree);
				$$renderer.push(`<!----></div>`);
			} else $$renderer.push("<!--[-1-->");
			$$renderer.push(`<!--]-->`);
		}
		$$renderer.push(`<!--]--></div>`);
	});
}
//#endregion
//#region src/routes/wiki/pokemon/[slug]/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		let p = derived(() => data.pokemon);
		let s = derived(() => data.species);
		let spriteUrl = derived(() => p().spriteArtwork || p().spriteDefault || `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/${p().pokeapiId}.png`);
		let totalStats = derived(() => data.stats.reduce((acc, stat) => acc + stat.baseStat, 0));
		const versionIdsWithData = new Set([...data.flavorTexts.map((f) => f.versionGroupId), ...data.learnset.map((l) => l.versionGroupId)]);
		let validVersions = derived(() => data.availableVersions.filter((v) => versionIdsWithData.has(v.id)));
		let selectedVersionId = null;
		let currentFlavorText = derived(() => () => {
			return "No Pokédex entry available.";
		});
		let currentLearnset = derived(() => () => {
			return [];
		});
		head("f9ek29", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>${escape_html(p().name.toUpperCase())} | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="pokemon-detail svelte-f9ek29"><header class="detail-header svelte-f9ek29"><div class="dex-num svelte-f9ek29">#${escape_html(p().pokeapiId.toString().padStart(3, "0"))}</div> <h1 class="svelte-f9ek29">${escape_html(p().name.toUpperCase())}</h1> <div class="types svelte-f9ek29"><!--[-->`);
		const each_array = ensure_array_like(data.types);
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let t = each_array[$$index];
			TypeBadge($$renderer, {
				name: t.name,
				size: "lg"
			});
		}
		$$renderer.push(`<!--]--></div></header> <div class="content-grid svelte-f9ek29"><div class="left-col svelte-f9ek29"><div class="image-box svelte-f9ek29"><img${attr("src", spriteUrl())}${attr("alt", p().name)} class="svelte-f9ek29"/></div> `);
		if (validVersions().length > 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="version-selector-container svelte-f9ek29"><label for="versionSelect" class="svelte-f9ek29">VERSION:</label> `);
			$$renderer.select({
				id: "versionSelect",
				value: selectedVersionId,
				class: "version-select"
			}, ($$renderer) => {
				$$renderer.push(`<!--[-->`);
				const each_array_1 = ensure_array_like(validVersions());
				for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
					let v = each_array_1[$$index_1];
					$$renderer.option({ value: v.id }, ($$renderer) => {
						$$renderer.push(`${escape_html(v.name.toUpperCase().replace("-", " "))}`);
					});
				}
				$$renderer.push(`<!--]-->`);
			}, "svelte-f9ek29");
			$$renderer.push(`</div> <div class="flavor-text-box svelte-f9ek29"><p>${escape_html(currentFlavorText()())}</p></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--> <div class="info-box svelte-f9ek29"><h3 class="svelte-f9ek29">DATA</h3> <ul class="data-list svelte-f9ek29"><li class="svelte-f9ek29"><span class="label svelte-f9ek29">HEIGHT</span> <span class="value svelte-f9ek29">${escape_html(p().height / 10)} m</span></li> <li class="svelte-f9ek29"><span class="label svelte-f9ek29">WEIGHT</span> <span class="value svelte-f9ek29">${escape_html(p().weight / 10)} kg</span></li> <li class="svelte-f9ek29"><span class="label svelte-f9ek29">HABITAT</span> <span class="value svelte-f9ek29">${escape_html(s().habitat ? s().habitat.toUpperCase() : "UNKNOWN")}</span></li> <li class="svelte-f9ek29"><span class="label svelte-f9ek29">GROWTH</span> <span class="value svelte-f9ek29">${escape_html(s().growthRate ? s().growthRate.toUpperCase() : "UNKNOWN")}</span></li></ul></div> <div class="info-box svelte-f9ek29"><h3 class="svelte-f9ek29">ABILITIES</h3> <ul class="ability-list svelte-f9ek29"><!--[-->`);
		const each_array_2 = ensure_array_like(data.abilities);
		for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
			let a = each_array_2[$$index_2];
			$$renderer.push(`<li${attr_class("svelte-f9ek29", void 0, { "hidden-ability": a.isHidden })}><a${attr("href", `/wiki/abilities/${stringify(a.slug)}`)} class="ability-link svelte-f9ek29"><span class="name">${escape_html(a.name.toUpperCase())}</span> `);
			if (a.isHidden) {
				$$renderer.push("<!--[0-->");
				$$renderer.push(`<span class="badge svelte-f9ek29">HIDDEN</span>`);
			} else $$renderer.push("<!--[-1-->");
			$$renderer.push(`<!--]--></a></li>`);
		}
		$$renderer.push(`<!--]--></ul></div></div> <div class="right-col svelte-f9ek29"><div class="info-box stats-box svelte-f9ek29"><h3 class="svelte-f9ek29">BASE STATS</h3> <div class="stats-container svelte-f9ek29"><!--[-->`);
		const each_array_3 = ensure_array_like(data.stats);
		for (let $$index_3 = 0, $$length = each_array_3.length; $$index_3 < $$length; $$index_3++) {
			let stat = each_array_3[$$index_3];
			StatBar($$renderer, {
				name: stat.statName,
				value: stat.baseStat,
				max: 255
			});
		}
		$$renderer.push(`<!--]--> <div class="stat-total svelte-f9ek29"><span class="label">TOTAL:</span> <span class="value">${escape_html(totalStats())}</span></div></div></div></div></div> `);
		if (data.evolutionNodes && data.evolutionNodes.length > 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="info-box evolution-box svelte-f9ek29"><h3 class="svelte-f9ek29">EVOLUTION CHAIN</h3> `);
			EvolutionTree($$renderer, {
				nodes: data.evolutionNodes,
				edges: data.evolutionEdges
			});
			$$renderer.push(`<!----></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--> `);
		if (validVersions().length > 0) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="info-box learnset-box svelte-f9ek29"><h3 class="svelte-f9ek29">LEARNSET (${escape_html(validVersions().find((v) => v.id === selectedVersionId)?.name.toUpperCase().replace("-", " "))})</h3> `);
			if (currentLearnset()().length > 0) {
				$$renderer.push("<!--[0-->");
				$$renderer.push(`<div class="learnset-table-wrapper svelte-f9ek29"><table class="learnset-table svelte-f9ek29"><thead><tr><th class="svelte-f9ek29">LVL / METHOD</th><th class="svelte-f9ek29">MOVE</th><th class="svelte-f9ek29">TYPE</th><th class="svelte-f9ek29">CAT</th></tr></thead><tbody><!--[-->`);
				const each_array_4 = ensure_array_like(currentLearnset()());
				for (let $$index_4 = 0, $$length = each_array_4.length; $$index_4 < $$length; $$index_4++) {
					let move = each_array_4[$$index_4];
					$$renderer.push(`<tr><td class="method svelte-f9ek29">${escape_html(move.methodName === "level-up" ? `Lvl ${move.level}` : move.methodName.toUpperCase())}</td><td class="move-name svelte-f9ek29"><a${attr("href", `/wiki/moves/${stringify(move.moveSlug)}`)} class="svelte-f9ek29">${escape_html(move.moveName.toUpperCase().replace(/-/g, " "))}</a></td><td class="svelte-f9ek29">`);
					MoveCategoryIcon($$renderer, {
						category: move.damageClass,
						size: "sm"
					});
					$$renderer.push(`<!----></td><td class="svelte-f9ek29">`);
					TypeBadge($$renderer, {
						name: move.typeName,
						size: "sm"
					});
					$$renderer.push(`<!----></td></tr>`);
				}
				$$renderer.push(`<!--]--></tbody></table></div>`);
			} else {
				$$renderer.push("<!--[-1-->");
				$$renderer.push(`<p class="empty-text svelte-f9ek29">No moves learned in this version.</p>`);
			}
			$$renderer.push(`<!--]--></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></div>`);
	});
}
//#endregion
export { _page as default };
