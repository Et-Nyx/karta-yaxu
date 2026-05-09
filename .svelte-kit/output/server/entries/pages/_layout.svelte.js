import "../../chunks/environment.js";
import { B as attr, V as escape_html, a as ensure_array_like, c as store_get, et as getContext, i as derived, n as attr_class, u as unsubscribe_stores } from "../../chunks/dev.js";
import "../../chunks/client.js";
//#region node_modules/@sveltejs/kit/src/runtime/app/stores.js
/**
* A function that returns all of the contextual stores. On the server, this must be called during component initialization.
* Only use this if you need to defer store subscription until after the component has mounted, for some reason.
*
* @deprecated Use `$app/state` instead (requires Svelte 5, [see docs for more info](https://svelte.dev/docs/kit/migrating-to-sveltekit-2#SvelteKit-2.12:-$app-stores-deprecated))
*/
var getStores = () => {
	const stores$1 = getContext("__svelte__");
	return {
		/** @type {typeof page} */
		page: { subscribe: stores$1.page.subscribe },
		/** @type {typeof navigating} */
		navigating: { subscribe: stores$1.navigating.subscribe },
		/** @type {typeof updated} */
		updated: stores$1.updated
	};
};
/**
* A readable store whose value contains page data.
*
* On the server, this store can only be subscribed to during component initialization. In the browser, it can be subscribed to at any time.
*
* @deprecated Use `page` from `$app/state` instead (requires Svelte 5, [see docs for more info](https://svelte.dev/docs/kit/migrating-to-sveltekit-2#SvelteKit-2.12:-$app-stores-deprecated))
* @type {import('svelte/store').Readable<import('@sveltejs/kit').Page>}
*/
var page = { subscribe(fn) {
	return getStores().page.subscribe(fn);
} };
//#endregion
//#region src/routes/+layout.svelte
function _layout($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		var $$store_subs;
		let { children } = $$props;
		let isAdminRoute = derived(() => store_get($$store_subs ??= {}, "$page", page).url.pathname.startsWith("/import") || store_get($$store_subs ??= {}, "$page", page).url.pathname.startsWith("/editor"));
		let selectedTheme = "auto";
		let breadcrumbs = derived(() => () => {
			const paths = store_get($$store_subs ??= {}, "$page", page).url.pathname.split("/").filter(Boolean);
			const crumbs = [{
				label: "HOME",
				href: "/"
			}];
			let currentPath = "";
			for (const segment of paths) {
				currentPath += `/${segment}`;
				const label = segment.toUpperCase().replace(/-/g, " ");
				crumbs.push({
					label,
					href: currentPath
				});
			}
			return crumbs;
		});
		let parentHref = derived(() => () => {
			const crumbs = breadcrumbs()();
			return crumbs.length >= 2 ? crumbs[crumbs.length - 2].href : "/";
		});
		$$renderer.push(`<div class="app-container svelte-12qhfyh"${attr("data-theme", isAdminRoute() ? "admin" : null)}><div class="gameboy-frame svelte-12qhfyh"><header class="header svelte-12qhfyh"><div class="logo svelte-12qhfyh"><a href="/" class="svelte-12qhfyh">KARTA YAXU</a> <span class="edition svelte-12qhfyh">GAMEBOY EDITION</span></div> <div class="nav-tabs svelte-12qhfyh">`);
		$$renderer.select({
			value: selectedTheme,
			class: "theme-select"
		}, ($$renderer) => {
			$$renderer.option({ value: "auto" }, ($$renderer) => {
				$$renderer.push(`Theme: Auto`);
			});
			$$renderer.option({ value: "light" }, ($$renderer) => {
				$$renderer.push(`Theme: Light`);
			});
			$$renderer.option({ value: "dark" }, ($$renderer) => {
				$$renderer.push(`Theme: Dark`);
			});
			$$renderer.option({ value: "admin" }, ($$renderer) => {
				$$renderer.push(`Theme: Admin (Verde)`);
			});
		}, "svelte-12qhfyh");
		$$renderer.push(` <a href="/wiki"${attr_class("tab svelte-12qhfyh", void 0, { "active": store_get($$store_subs ??= {}, "$page", page).url.pathname.startsWith("/wiki") })}>WIKI</a> <a href="/editor"${attr_class("tab svelte-12qhfyh", void 0, { "active": store_get($$store_subs ??= {}, "$page", page).url.pathname.startsWith("/editor") })}>EDITOR</a> <a href="/import"${attr_class("tab svelte-12qhfyh", void 0, { "active": store_get($$store_subs ??= {}, "$page", page).url.pathname.startsWith("/import") })}>IMPORT</a> <a href="/battle"${attr_class("tab svelte-12qhfyh", void 0, { "active": store_get($$store_subs ??= {}, "$page", page).url.pathname.startsWith("/battle") })}>BATTLE</a> <a href="/trainer/login" class="tab login svelte-12qhfyh">LOGIN</a></div></header> <div class="layout svelte-12qhfyh"><aside class="sidebar svelte-12qhfyh"><nav class="side-nav"><div class="nav-section svelte-12qhfyh"><h3 class="svelte-12qhfyh">WIKI</h3> <a href="/wiki/pokemon" class="svelte-12qhfyh">Pokémon</a> <a href="/wiki/moves" class="svelte-12qhfyh">Moves</a> <a href="/wiki/abilities" class="svelte-12qhfyh">Abilities</a> <a href="/wiki/types" class="svelte-12qhfyh">Types</a> <a href="/wiki/items" class="svelte-12qhfyh">Items</a> <a href="/wiki/locations" class="svelte-12qhfyh">Locations</a></div> `);
		if (isAdminRoute()) {
			$$renderer.push("<!--[0-->");
			$$renderer.push(`<div class="nav-section svelte-12qhfyh"><h3 class="svelte-12qhfyh">ADMIN</h3> <a href="/editor" class="svelte-12qhfyh">Database Editor</a> <a href="/import" class="svelte-12qhfyh">PokeAPI Import</a></div>`);
		} else $$renderer.push("<!--[-1-->");
		$$renderer.push(`<!--]--></nav> <div class="version-select svelte-12qhfyh"><label for="vg-select" class="svelte-12qhfyh">VERSION</label> <select id="vg-select" class="svelte-12qhfyh">`);
		$$renderer.option({}, ($$renderer) => {
			$$renderer.push(`FireRed / LeafGreen`);
		});
		$$renderer.option({}, ($$renderer) => {
			$$renderer.push(`Ruby / Sapphire`);
		});
		$$renderer.option({}, ($$renderer) => {
			$$renderer.push(`Emerald`);
		});
		$$renderer.push(`</select></div></aside> <main class="content-wrapper svelte-12qhfyh"><div class="breadcrumbs svelte-12qhfyh"><a${attr("href", parentHref()())} class="back-link svelte-12qhfyh">← BACK</a> <span class="breadcrumb-divider svelte-12qhfyh">|</span> <!--[-->`);
		const each_array = ensure_array_like(breadcrumbs()());
		for (let i = 0, $$length = each_array.length; i < $$length; i++) {
			let crumb = each_array[i];
			if (i < breadcrumbs()().length - 1) {
				$$renderer.push("<!--[0-->");
				$$renderer.push(`<a${attr("href", crumb.href)} class="crumb svelte-12qhfyh">${escape_html(crumb.label)}</a> <span class="separator svelte-12qhfyh">></span>`);
			} else {
				$$renderer.push("<!--[-1-->");
				$$renderer.push(`<span class="crumb active svelte-12qhfyh">${escape_html(crumb.label)}</span>`);
			}
			$$renderer.push(`<!--]-->`);
		}
		$$renderer.push(`<!--]--></div> <div class="content svelte-12qhfyh">`);
		children($$renderer);
		$$renderer.push(`<!----></div></main></div></div></div>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}
//#endregion
export { _layout as default };
