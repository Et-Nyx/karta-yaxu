import { o as head } from "../../chunks/dev.js";
//#region src/routes/+page.svelte
function _page($$renderer) {
	head("1uha8ag", $$renderer, ($$renderer) => {
		$$renderer.title(($$renderer) => {
			$$renderer.push(`<title>Karta Yaxu — Pokémon ROM Hack Wiki &amp; Simulator</title>`);
		});
		$$renderer.push(`<meta name="description" content="Wiki, Editor &amp; Battle Simulator for Pokémon Gen 3 ROM Hack"/>`);
	});
	$$renderer.push(`<div class="home-content svelte-1uha8ag"><div class="welcome-box svelte-1uha8ag"><h1 class="svelte-1uha8ag">BEM-VINDO A KARTA YAXU</h1> <p class="svelte-1uha8ag">O simulador e wiki open-source para a sua ROM hack da Geração 3.</p> <div class="quick-links svelte-1uha8ag"><a href="/wiki/pokemon" class="btn svelte-1uha8ag">Pokédex (Wiki)</a> <a href="/import" class="btn svelte-1uha8ag">Importar Dados (Admin)</a></div></div></div>`);
}
//#endregion
export { _page as default };
