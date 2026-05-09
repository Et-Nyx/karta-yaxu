import { h as head, e as ensure_array_like, a as attr, j as stringify, c as escape_html, i as attr_style } from './dev-Br_spHyP.js';
import { T as TypeIcon } from './TypeIcon-B6kcdt8A.js';

//#region src/routes/wiki/types/+page.svelte
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { data } = $$props;
		head("knyswf", $$renderer, ($$renderer) => {
			$$renderer.title(($$renderer) => {
				$$renderer.push(`<title>Types | Karta Yaxu</title>`);
			});
		});
		$$renderer.push(`<div class="types-page svelte-knyswf"><div class="page-header svelte-knyswf"><h2 class="svelte-knyswf">TYPES</h2></div> <div class="types-grid svelte-knyswf"><!--[-->`);
		const each_array = ensure_array_like(data.types);
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let t = each_array[$$index];
			$$renderer.push(`<a${attr("href", `/wiki/types/${stringify(t.slug)}`)} class="type-card card-interactive svelte-knyswf"><div class="type-content svelte-knyswf">`);
			TypeIcon($$renderer, {
				typeName: t.name,
				size: "lg"
			});
			$$renderer.push(`<!----> <span class="type-name svelte-knyswf">${escape_html(t.name.toUpperCase())}</span></div> <div class="type-bar svelte-knyswf"${attr_style(`background-color: var(--color-type-${stringify(t.name)})`)}></div></a>`);
		}
		$$renderer.push(`<!--]--></div></div>`);
	});
}

export { _page as default };
//# sourceMappingURL=_page.svelte-BTZ1VLAp.js.map
