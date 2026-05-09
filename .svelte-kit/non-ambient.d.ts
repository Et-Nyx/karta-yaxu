
// this file is generated — do not edit it


declare module "svelte/elements" {
	export interface HTMLAttributes<T> {
		'data-sveltekit-keepfocus'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-noscroll'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-preload-code'?:
			| true
			| ''
			| 'eager'
			| 'viewport'
			| 'hover'
			| 'tap'
			| 'off'
			| undefined
			| null;
		'data-sveltekit-preload-data'?: true | '' | 'hover' | 'tap' | 'off' | undefined | null;
		'data-sveltekit-reload'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-replacestate'?: true | '' | 'off' | undefined | null;
	}
}

export {};


declare module "$app/types" {
	type MatcherParam<M> = M extends (param : string) => param is (infer U extends string) ? U : string;

	export interface AppTypes {
		RouteId(): "/" | "/api" | "/api/import" | "/api/import/sse" | "/import" | "/wiki" | "/wiki/abilities" | "/wiki/abilities/[slug]" | "/wiki/moves" | "/wiki/moves/[slug]" | "/wiki/pokemon" | "/wiki/pokemon/[slug]" | "/wiki/types" | "/wiki/types/[slug]";
		RouteParams(): {
			"/wiki/abilities/[slug]": { slug: string };
			"/wiki/moves/[slug]": { slug: string };
			"/wiki/pokemon/[slug]": { slug: string };
			"/wiki/types/[slug]": { slug: string }
		};
		LayoutParams(): {
			"/": { slug?: string };
			"/api": Record<string, never>;
			"/api/import": Record<string, never>;
			"/api/import/sse": Record<string, never>;
			"/import": Record<string, never>;
			"/wiki": { slug?: string };
			"/wiki/abilities": { slug?: string };
			"/wiki/abilities/[slug]": { slug: string };
			"/wiki/moves": { slug?: string };
			"/wiki/moves/[slug]": { slug: string };
			"/wiki/pokemon": { slug?: string };
			"/wiki/pokemon/[slug]": { slug: string };
			"/wiki/types": { slug?: string };
			"/wiki/types/[slug]": { slug: string }
		};
		Pathname(): "/" | "/api/import/sse" | "/import" | "/wiki" | "/wiki/abilities" | `/wiki/abilities/${string}` & {} | "/wiki/moves" | `/wiki/moves/${string}` & {} | "/wiki/pokemon" | `/wiki/pokemon/${string}` & {} | "/wiki/types" | `/wiki/types/${string}` & {};
		ResolvedPathname(): `${"" | `/${string}`}${ReturnType<AppTypes['Pathname']>}`;
		Asset(): string & {};
	}
}