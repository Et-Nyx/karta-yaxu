import { B as attr, i as derived, l as stringify, n as attr_class } from "./dev.js";
//#region src/lib/components/TypeIcon.svelte
function TypeIcon($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { typeName = "normal", size = "md", class: className = "" } = $$props;
		const typeIdMap = {
			normal: 1,
			fighting: 2,
			flying: 3,
			poison: 4,
			ground: 5,
			rock: 6,
			bug: 7,
			ghost: 8,
			steel: 9,
			fire: 10,
			water: 11,
			grass: 12,
			electric: 13,
			psychic: 14,
			ice: 15,
			dragon: 16,
			dark: 17,
			fairy: 18,
			stellar: 19
		};
		let typeId = derived(() => typeIdMap[typeName.toLowerCase()] || 1);
		let iconUrl = derived(() => `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/types/generation-ix/scarlet-violet/small/${typeId()}.png`);
		let sizeClass = derived(() => ({
			sm: "type-icon-sm",
			md: "type-icon-md",
			lg: "type-icon-lg"
		})[size] || "type-icon-md");
		$$renderer.push(`<img${attr("src", iconUrl())}${attr("alt", `${stringify(typeName)} type icon`)}${attr_class(`type-icon ${stringify(sizeClass())} ${stringify(className)}`, "svelte-163njsz")} loading="lazy"/>`);
	});
}
//#endregion
export { TypeIcon as t };
