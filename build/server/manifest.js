const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set([]),
	mimeTypes: {},
	_: {
		client: {start:"_app/immutable/entry/start.BlnlXX_l.js",app:"_app/immutable/entry/app.CHy03HbM.js",imports:["_app/immutable/entry/start.BlnlXX_l.js","_app/immutable/chunks/DsitodRo.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/entry/app.CHy03HbM.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/O6-aNYLC.js","_app/immutable/chunks/CP97kCR3.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./chunks/0-C16nzy2x.js')),
			__memo(() => import('./chunks/1-COwn7utv.js')),
			__memo(() => import('./chunks/2-DVEnsa4K.js')),
			__memo(() => import('./chunks/3-Bgaf2t78.js')),
			__memo(() => import('./chunks/4-CjSeh9wO.js')),
			__memo(() => import('./chunks/5-1jDU5tnP.js')),
			__memo(() => import('./chunks/6-i9PY_2iU.js')),
			__memo(() => import('./chunks/7-DB-p0JpS.js')),
			__memo(() => import('./chunks/8-CdBisHVb.js')),
			__memo(() => import('./chunks/9-B8eoK9o5.js')),
			__memo(() => import('./chunks/10-_auNWfkl.js')),
			__memo(() => import('./chunks/11-D4JEGTAX.js')),
			__memo(() => import('./chunks/12-3ou57VoO.js'))
		],
		remotes: {
			
		},
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			},
			{
				id: "/api/import/sse",
				pattern: /^\/api\/import\/sse\/?$/,
				params: [],
				page: null,
				endpoint: __memo(() => import('./chunks/_server.ts-BW3NErze.js'))
			},
			{
				id: "/import",
				pattern: /^\/import\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 3 },
				endpoint: null
			},
			{
				id: "/wiki",
				pattern: /^\/wiki\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 4 },
				endpoint: null
			},
			{
				id: "/wiki/abilities",
				pattern: /^\/wiki\/abilities\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 5 },
				endpoint: null
			},
			{
				id: "/wiki/abilities/[slug]",
				pattern: /^\/wiki\/abilities\/([^/]+?)\/?$/,
				params: [{"name":"slug","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 6 },
				endpoint: null
			},
			{
				id: "/wiki/moves",
				pattern: /^\/wiki\/moves\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 7 },
				endpoint: null
			},
			{
				id: "/wiki/moves/[slug]",
				pattern: /^\/wiki\/moves\/([^/]+?)\/?$/,
				params: [{"name":"slug","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 8 },
				endpoint: null
			},
			{
				id: "/wiki/pokemon",
				pattern: /^\/wiki\/pokemon\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 9 },
				endpoint: null
			},
			{
				id: "/wiki/pokemon/[slug]",
				pattern: /^\/wiki\/pokemon\/([^/]+?)\/?$/,
				params: [{"name":"slug","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 10 },
				endpoint: null
			},
			{
				id: "/wiki/types",
				pattern: /^\/wiki\/types\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 11 },
				endpoint: null
			},
			{
				id: "/wiki/types/[slug]",
				pattern: /^\/wiki\/types\/([^/]+?)\/?$/,
				params: [{"name":"slug","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 12 },
				endpoint: null
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();

const prerendered = new Set([]);

const base = "";

export { base, manifest, prerendered };
//# sourceMappingURL=manifest.js.map
