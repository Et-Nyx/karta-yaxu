import { B as attr, i as derived, l as stringify, n as attr_class } from "./dev.js";
//#region src/lib/components/MoveCategoryIcon.svelte
function MoveCategoryIcon($$renderer, $$props) {
	$$renderer.component(($$renderer) => {
		let { category = "physical", size = "md", class: className = "" } = $$props;
		const iconUrls = {
			physical: "https://archives.bulbagarden.net/media/upload/b/b4/PhysicalIC_SV.png",
			special: "https://archives.bulbagarden.net/media/upload/5/5b/SpecialIC_SV.png",
			status: "https://archives.bulbagarden.net/media/upload/e/e0/StatusIC_SV.png"
		};
		let iconUrl = derived(() => iconUrls[category.toLowerCase()] || iconUrls.physical);
		let sizeClass = derived(() => ({
			sm: "move-category-icon-sm",
			md: "move-category-icon-md",
			lg: "move-category-icon-lg"
		})[size] || "move-category-icon-md");
		$$renderer.push(`<img${attr("src", iconUrl())}${attr("alt", `${stringify(category)} category`)}${attr_class(`move-category-icon ${stringify(sizeClass())} ${stringify(className)}`, "svelte-1bzbt6g")} loading="lazy"/>`);
	});
}
//#endregion
export { MoveCategoryIcon as t };
