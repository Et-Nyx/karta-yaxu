import { V as escape_html, i as derived, l as stringify, n as attr_class, r as attr_style } from "./dev.js";
//#region src/lib/components/TypeBadge.svelte
function TypeBadge($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { name = "normal", size = "md", class: className = "" } = $$props;
		const typeColors = {
			normal: "#9FA19F",
			fire: "#E62829",
			water: "#2980EF",
			electric: "#FAC000",
			grass: "#3FA129",
			ice: "#3DD8FF",
			fighting: "#FF8000",
			poison: "#9141CB",
			ground: "#915121",
			flying: "#81B9EF",
			psychic: "#EF4179",
			bug: "#91A119",
			rock: "#AFA981",
			ghost: "#704170",
			dragon: "#5060E1",
			dark: "#4F3F3D",
			steel: "#60A1B8",
			fairy: "#EF70EF",
			stellar: "#83CFC5"
		};
		let bgColor = derived(() => typeColors[name.toLowerCase()] || typeColors.normal);
		$$renderer.push(`<span${attr_class(`type-badge size-${stringify(size)} ${stringify(className)}`, "svelte-433zwn")}${attr_style(`background-color: ${stringify(bgColor())}`)}>${escape_html(name.toUpperCase())}</span>`);
	});
}
//#endregion
export { TypeBadge as t };
