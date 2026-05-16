# Plano do Projeto — Pokemon ROM Hack Simulator & Wiki

## Visão Geral

Uma plataforma web com três pilares integrados para suportar o desenvolvimento de um ROM hack de Pokémon Gen 3 (RSE/FRLG):

- **Wiki** — documentação pública dos recursos da ROM (Pokémon, moves, abilities, itens, mecânicas, localizações, encounters)
- **Editor** — interface para o time de desenvolvimento editar os dados que alimentam a wiki e o simulador
- **Simulador** — batalhas online em tempo real com as regras customizadas da ROM

Todos os pilares compartilham o mesmo **banco de dados SQLite**. Quando o time atualiza um movimento no editor, a wiki e o simulador refletem a mudança automaticamente.

### Importação Sob Demanda da PokeAPI

O banco **não parte com dados pré-carregados**. Em vez de scripts de seed que despejam centenas de registros de uma vez, existe um **sistema de importação sob demanda**: o editor importa da PokeAPI apenas os dados que precisa, diretamente pela interface web. Qualquer dado importado pode ser editado e customizado no Editor — e essas customizações são protegidas contra sobrescrita em futuras re-importações.

Isso elimina a barreira de "adaptar manualmente a estrutura dos dados" e dá controle total sobre o que entra no banco: Pokémon de qualquer geração, moves, abilities, itens, tipos, locations, encounters — tudo importável com um clique.

---

## Stack

| Camada | Tecnologia | Motivo da Escolha |
| --- | --- | --- |
| Framework Fullstack | **SvelteKit** | SSR nativo, API routes integradas, reatividade compilada, menos boilerplate |
| Linguagem | **TypeScript** | Type-safe em todo o stack, mesmo ecossistema no client e server |
| Banco de Dados | **SQLite (WAL mode)** | Arquivo único, zero configuração, backups triviais, performático |
| ORM / Query Builder | **Drizzle ORM** | Type-safe, SQL-like, migrations automáticas, sem codegen pesado |
| Autenticação | **Lucia Auth** | Open-source, construída para SvelteKit, sessions seguras, extensível |
| Estilização | **TailwindCSS** | Utility-first, prototipação rápida, bundle otimizado |
| Busca Fuzzy | **Fuse.js** | Client-side, open-source, Levenshtein distance |
| WebSocket / Battles | **Socket.io** | Rooms nativas, fallback automático, reconexão integrada |
| Engine de Batalha | **TypeScript puro** | Roda no servidor (validação anti-cheat) e pode ser reusado no cliente |
| Runtime | **Node.js 20+** | Padrão SvelteKit com `adapter-node`, compatível com Docker e Coolify |
| Deploy | **Docker multi-stage** + **Coolify** + **Oracle Cloud (Free Tier)** | Infraestrutura já validada, 100% gratuita |

---

## Arquitetura — Shared Engine

A principal vantagem arquitetural é que o mesmo código TypeScript do **engine de batalha** pode rodar em dois ambientes:

```
┌─────────────────────────────────┐
│      engine/ (TS puro)          │
│  damage calc, battle loop,      │
│  gen 3 mechanics, validators    │
└──────────┬──────────────────────┘
           │ importado por...
     ┌─────┴──────┐
     ▼            ▼
  Cliente     Servidor
 (preview)   (validação)
```

---

## Arquitetura — Importação Sob Demanda

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   PokeAPI        │     │   Módulo de      │     │   SQLite         │
│   (pokeapi.co)   │────►│   Importação     │────►│   (Drizzle ORM)  │
│                  │     │   (src/lib/       │     │                  │
│   /pokemon/25    │     │    pokeapi/)      │     │   source=pokeapi │
│   /type/3        │     │                  │     │   source=edited   │
│   /move/1        │     │   - Busca JSON   │     │   source=custom  │
│   /generation/3  │     │   - Mapeia campos│     │                  │
│   /location/88   │     │   - Insere/       │     │   pokeapi_id=25  │
│   /pokedex/2     │     │     atualiza      │     │   pokeapi_id=3   │
│                  │     │   - Protege       │     └──────────────────┘
└──────────────────┘     │     edited/custom │              │
                          └──────────────────┘              ▼
                                                  ┌──────────────────┐
                                                  │   Painel Web     │
                                                  │   /import        │
                                                  │   (role=editor)  │
                                                  └──────────────────┘
```

### Campo `source` — Proteção contra Sobrescrita

Toda tabela de dados importáveis possui duas colunas de rastreamento:

| Coluna | Tipo | Descrição |
| --- | --- | --- |
| `pokeapi_id` | `integer` | ID original na PokeAPI. `NULL` para registros criados manualmente. |
| `source` | `text` | Origem do registro: `pokeapi`, `edited`, ou `custom` |

| `source` | Significado | Comportamento em re-importação |
| --- | --- | --- |
| `pokeapi` | Importado da PokeAPI, nunca editado | Sobrescrito (atualiza dados) |
| `edited` | Importado + editado manualmente | **Protegido** — nunca sobrescrito |
| `custom` | Criado manualmente no Editor | **Protegido** — nunca tocado |

Quando o Editor salva uma alteração em um registro `source='pokeapi'`, o campo muda para `source='edited'`.

---

## Estrutura do Projeto

```
karta-yaxu-2.0/
├── docker/
│   ├── Dockerfile                 # ✅ Multi-stage build (Node 20 Alpine)
│   └── entrypoint.sh              # ✅ Runtime init script
├── scripts/
│   └── test-import.ts             # ✅ Script de verificação do módulo de importação
├── src/
│   ├── app.html                   # ✅ Template com Google Fonts (Press Start 2P, Inter, JetBrains Mono)
│   ├── app.css                    # ✅ Design tokens (TailwindCSS v4 @theme)
│   ├── app.d.ts                   # ✅ Tipos globais (Lucia session)
│   ├── hooks.server.ts            # ✅ Validação de sessão, kick de contas banidas
│   ├── lib/
│   │   ├── db/
│   │   │   ├── index.ts           # ✅ Conexão SQLite (WAL) + Drizzle client
│   │   │   └── schema/            # ✅ Schema modular (58 tabelas)
│   │   │       ├── index.ts       #    Barrel export
│   │   │       ├── game-metadata.ts #  Languages, Regions, Generations, Versions, Pokedexes
│   │   │       ├── locations.ts   #    Locations, Areas, Encounter Methods/Conditions
│   │   │       ├── core-data.ts   #    Types, Abilities, Moves, MoveLearnMethods
│   │   │       ├── items.ts       #    Items, Natures
│   │   │       ├── pokemon.ts     #    Species, Pokemon, Forms, Evolution (linked-list), Encounters
│   │   │       ├── sessions.ts    # ✅ Sessions (Lucia Auth)
│   │   │       └── trainers.ts    #    Trainers (bannedAt, isNpc), Teams, Battles, Edits, ImportLog
│   │   ├── auth/                  # ✅
│   │   │   ├── lucia.ts           #    Lucia Auth v3 + adapter Drizzle + getUserAttributes
│   │   │   ├── guards.ts          #    requireAuth, requireEditor
│   │   │   └── password.ts        #    hashPassword / verifyPassword (node:crypto scrypt)
│   │   ├── editor/                # ✅ Helpers para o painel /editor
│   │   │   ├── audit.ts           #    recordEdit() — escreve em `edits`
│   │   │   ├── source.ts          #    bumpSource() — pokeapi → edited on save
│   │   │   └── resource.ts        #    reimportEntity() — RE-SOURCE
│   │   ├── pokeapi/               # ✅ Módulo de importação completo
│   │   │   ├── index.ts           #    Barrel export
│   │   │   ├── types.ts           #    Interfaces TS das respostas da PokeAPI
│   │   │   ├── client.ts          #    HTTP client com cache, rate limiting, retry
│   │   │   ├── mappers.ts         #    JSON → registros Drizzle (funções puras)
│   │   │   ├── import-service.ts  #    Ensure* (game metadata) + batch imports
│   │   │   └── import-pokemon.ts  #    Ensure* (core data, pokemon, evo chains) + batch imports
│   │   ├── engine/                # (pendente)
│   │   │   ├── types.ts
│   │   │   ├── damage.ts
│   │   │   ├── battle.ts
│   │   │   ├── validators.ts
│   │   │   └── index.ts
│   │   ├── server/                # (pendente)
│   │   │   ├── queries.ts
│   │   │   └── search.ts
│   │   └── components/            # ✅ (parcial)
│   │       ├── PokemonCard.svelte        # ✅
│   │       ├── TypeBadge.svelte          # ✅
│   │       ├── SearchBar.svelte          # ✅
│   │       ├── MoveCard.svelte           # ✅
│   │       ├── ItemCard.svelte           # ✅
│   │       ├── LocationCard.svelte       # ✅
│   │       ├── MoveCategoryIcon.svelte   # ✅
│   │       ├── TypeIcon.svelte           # ✅
│   │       ├── TypeMatchupDisplay.svelte # ✅
│   │       ├── StatBar.svelte            # ✅
│   │       ├── NavTab.svelte             # ✅ Wrapper dos botões da top bar
│   │       ├── ImportActivity.svelte     # ✅
│   │       ├── ImportButton.svelte       # ✅
│   │       ├── ImportForm.svelte         # ✅
│   │       ├── ImportLog.svelte          # ✅
│   │       ├── ImportPanel.svelte        # ✅
│   │       ├── ImportStatusGrid.svelte   # ✅
│   │       └── FilterSidebar.svelte      # (pendente)
│   └── routes/
│       ├── +page.svelte           # ✅ Home placeholder (Gameboy frame)
│       ├── +layout.svelte         # ✅ Root layout (importa app.css)
│       ├── +layout.server.ts      # ✅ Expõe locals.user para o layout
│       ├── wiki/                  # ✅ pokemon, moves, abilities, types, natures, items, locations, games, pokedex
│       ├── search/                # (pendente)
│       ├── api/                   # ✅ (parcial)
│       │   └── editor/
│       │       └── base-form/     #    ✅ GET /[speciesId] for New Pokemon prefill
│       ├── editor/                # ✅ Painel de edição (role=editor)
│       │   ├── types/             #    ✅
│       │   ├── natures/           #    ✅
│       │   ├── abilities/         #    ✅
│       │   ├── items/             #    ✅
│       │   ├── moves/             #    ✅
│       │   ├── pokedex/           #    ✅
│       │   ├── pokemon/           #    ✅ (types, abilities, forms, evolution, learnset sub-editors)
│       │   └── species/           #    ✅ (auto-Base-Form, egg groups)
│       ├── import/                # ✅ Painel de importação PokeAPI
│       ├── battle/                # (pendente)
│       └── trainer/               # ✅
│           ├── +page.server.ts    #    Lista de trainers + ações admin
│           ├── +page.svelte       #    Página pública com controles de editor
│           ├── login/             #    ✅
│           ├── register/          #    ✅
│           ├── logout/            #    ✅
│           └── [username]/        #    ✅ Perfil + histórico de edições
│               ├── +page.{server.ts,svelte}
│               └── edits/
├── drizzle/
│   └── meta/
├── data/
│   └── rom.db                     # ✅ SQLite database (criado por db:push)
├── static/
├── flake.nix                      # ✅ Nix dev environment (Node 20, SQLite)
├── package.json
├── svelte.config.js               # ✅ adapter-node
├── vite.config.ts                 # ✅ TailwindCSS v4 + SvelteKit
├── tsconfig.json                  # ✅ Strict TypeScript
├── drizzle.config.ts              # ✅ Drizzle Kit config
└── docker-compose.yml             # ✅ Docker Compose com volume persistente
```

---

## Banco de Dados — SQLite com Drizzle ORM

O banco é gerenciado via **Drizzle ORM**. O schema é definido em TypeScript em módulos separados em `src/lib/db/schema/` (6 módulos, 58 tabelas), e o Drizzle Kit gera as migrations automaticamente.

### Convenções Gerais

Toda tabela de dados importáveis da PokeAPI segue estas convenções:

- **`id`**: Chave primária interna (auto-incremento)
- **`pokeapi_id`**: ID original na PokeAPI (para rastrear imports e re-imports). `NULL` para registros criados manualmente.
- **`source`**: Origem do registro — `'pokeapi'`, `'edited'`, ou `'custom'`
- **`slug`**: Versão URL-friendly do nome (lowercase, sem espaços/apóstrofos)

### Configuração de Conexão

```ts
// src/lib/db/index.ts
import { drizzle } from 'drizzle-orm/better-sqlite3';
import Database from 'better-sqlite3';
import * as schema from './schema';

const client = new Database(process.env.DATABASE_URL || './data/rom.db');
client.pragma('journal_mode = WAL');

export const db = drizzle(client, { schema });
```

### Schema — Game Metadata

Estas tabelas armazenam dados sobre as versões dos jogos, gerações, regiões, e idiomas. São importadas de uma vez via botão "Importar Game Data" no painel.

#### Languages

```ts
export const languages = sqliteTable('languages', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),       // 'en', 'fr', 'de', etc.
  iso639: text('iso639'),                        // 'en', 'ja', 'fr', 'de', etc.
  iso3166: text('iso3166'),                      // 'us', 'jp', 'fr', 'de', etc.
  official: integer('is_official', { mode: 'boolean' }).notNull().default(true),
});
```

#### Generations

```ts
export const generations = sqliteTable('generations', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'generation-i', 'generation-ii', etc.
  slug: text('slug').notNull().unique(),           // 'gen-1', 'gen-2', etc.
  regionId: integer('region_id').references(() => regions.id),
  order: integer('order'),
});

export const generationNames = sqliteTable('generation_names', {
  generationId: integer('generation_id').notNull().references(() => generations.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.generationId, t.languageId] }),
}));
```

#### Version Groups & Versions

```ts
export const versionGroups = sqliteTable('version_groups', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'ruby-sapphire', 'emerald', 'firered-leafgreen'
  slug: text('slug').notNull().unique(),
  generationId: integer('generation_id').notNull().references(() => generations.id),
  order: integer('order'),
});

export const versionGroupNames = sqliteTable('version_group_names', {
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.versionGroupId, t.languageId] }),
}));

export const versions = sqliteTable('versions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'ruby', 'sapphire', 'emerald', 'firered'
  slug: text('slug').notNull().unique(),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
});

export const versionNames = sqliteTable('version_names', {
  versionId: integer('version_id').notNull().references(() => versions.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.versionId, t.languageId] }),
}));
```

#### Regions

```ts
export const regions = sqliteTable('regions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'kanto', 'johto', 'hoenn'
  slug: text('slug').notNull().unique(),
  mainGenerationId: integer('main_generation_id').references(() => generations.id),
});

export const regionNames = sqliteTable('region_names', {
  regionId: integer('region_id').notNull().references(() => regions.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.regionId, t.languageId] }),
}));
```

#### Pokedexes

```ts
export const pokedexes = sqliteTable('pokedexes', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'national', 'kanto', 'hoenn'
  slug: text('slug').notNull().unique(),
  regionId: integer('region_id').references(() => regions.id),
  isMainSeries: integer('is_main_series', { mode: 'boolean' }).notNull().default(true),
});

export const pokedexNames = sqliteTable('pokedex_names', {
  pokedexId: integer('pokedex_id').notNull().references(() => pokedexes.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
  description: text('description'),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokedexId, t.languageId] }),
}));

export const pokedexEntries = sqliteTable('pokedex_entries', {
  pokedexId: integer('pokedex_id').notNull().references(() => pokedexes.id),
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  entryNumber: integer('entry_number').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokedexId, t.speciesId] }),
}));
```

### Schema — Location & Encounter Data

#### Locations

```ts
export const locations = sqliteTable('locations', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'pallet-town', 'kanto-route-1'
  slug: text('slug').notNull().unique(),
  regionId: integer('region_id').references(() => regions.id),
});

export const locationNames = sqliteTable('location_names', {
  locationId: integer('location_id').notNull().references(() => locations.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.locationId, t.languageId] }),
}));
```

#### Location Areas

```ts
export const locationAreas = sqliteTable('location_areas', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),          // 'kanto-route-1-area', 'mt-moon-1f'
  slug: text('slug').notNull().unique(),
  locationId: integer('location_id').notNull().references(() => locations.id),
  gameIndex: integer('game_index'),
});

export const locationAreaNames = sqliteTable('location_area_names', {
  locationAreaId: integer('location_area_id').notNull().references(() => locationAreas.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.locationAreaId, t.languageId] }),
}));
```

#### Encounter Methods & Conditions

```ts
export const encounterMethods = sqliteTable('encounter_methods', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),          // 'walk', 'old-rod', 'good-rod', 'super-rod', 'surf'
  slug: text('slug').notNull().unique(),
  order: integer('order'),
});

export const encounterMethodNames = sqliteTable('encounter_method_names', {
  encounterMethodId: integer('encounter_method_id').notNull().references(() => encounterMethods.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
  description: text('description'),
}, (t) => ({
  pk: primaryKey({ columns: [t.encounterMethodId, t.languageId] }),
}));

export const encounterConditions = sqliteTable('encounter_conditions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),          // 'swarm', 'time-of-day', 'season'
  slug: text('slug').notNull().unique(),
});

export const encounterConditionValues = sqliteTable('encounter_condition_values', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  conditionId: integer('condition_id').notNull().references(() => encounterConditions.id),
  name: text('name').notNull().unique(),          // 'swarm-yes', 'day', 'night'
  slug: text('slug').notNull().unique(),
});
```

#### Encounters

```ts
export const encounters = sqliteTable('encounters', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  locationAreaId: integer('location_area_id').notNull().references(() => locationAreas.id),
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  versionId: integer('version_id').notNull().references(() => versions.id),
  encounterMethodId: integer('encounter_method_id').notNull().references(() => encounterMethods.id),
  conditionValueId: integer('condition_value_id').references(() => encounterConditionValues.id),
  minLevel: integer('min_level').notNull(),
  maxLevel: integer('max_level').notNull(),
  chance: integer('chance').notNull(),           // probabilidade (1-255)
  slot: integer('slot'),                         // posição no encounter table
});
```

### Schema — Core Game Data (Types, Abilities, Moves, etc.)

#### Types

```ts
export const types = sqliteTable('types', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  hexColor: text('hexcolor'),
  iconUrl: text('icon_url'),
  moveDamageClass: text('move_damage_class', { enum: ['physical', 'special', 'status'] }),
  generationId: integer('generation_id').references(() => generations.id),
});

export const typeEffectiveness = sqliteTable('type_effectiveness', {
  attackerId: integer('attacker_id').notNull().references(() => types.id),
  defenderId: integer('defender_id').notNull().references(() => types.id),
  multiplier: real('multiplier').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.attackerId, t.defenderId] }),
}));

export const typeNames = sqliteTable('type_names', {
  typeId: integer('type_id').notNull().references(() => types.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.typeId, t.languageId] }),
}));
```

#### Abilities

```ts
export const abilities = sqliteTable('abilities', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  isMainSeries: integer('is_main_series', { mode: 'boolean' }).notNull().default(true),
  generationId: integer('generation_id').references(() => generations.id),
});

export const abilityNames = sqliteTable('ability_names', {
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.abilityId, t.languageId] }),
}));

export const abilityFlavorTexts = sqliteTable('ability_flavor_texts', {
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.abilityId, t.languageId, t.versionGroupId] }),
}));

export const abilityEffects = sqliteTable('ability_effects', {
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  effect: text('effect').notNull(),
  shortEffect: text('short_effect').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.abilityId, t.languageId] }),
}));
```

#### Moves

```ts
export const moves = sqliteTable('moves', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  typeId: integer('type_id').notNull().references(() => types.id),
  damageClass: text('damage_class', { enum: ['physical', 'special', 'status'] }).notNull(),
  power: integer('power'),
  accuracy: integer('accuracy'),
  pp: integer('pp').notNull(),
  priority: integer('priority').notNull().default(0),
  target: text('target'),
  generationId: integer('generation_id').references(() => generations.id),
});

export const moveNames = sqliteTable('move_names', {
  moveId: integer('move_id').notNull().references(() => moves.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveId, t.languageId] }),
}));

export const moveFlavorTexts = sqliteTable('move_flavor_texts', {
  moveId: integer('move_id').notNull().references(() => moves.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveId, t.languageId, t.versionGroupId] }),
}));

export const moveEffects = sqliteTable('move_effects', {
  moveId: integer('move_id').notNull().references(() => moves.id),
  effect: text('effect').notNull(),
  shortEffect: text('short_effect').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveId, t.languageId] }),
}));

export const moveLearnMethods = sqliteTable('move_learn_methods', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),            // 'level-up', 'machine', 'egg', 'tutor'
  slug: text('slug').notNull().unique(),
  description: text('description'),
});

export const moveLearnMethodNames = sqliteTable('move_learn_method_names', {
  moveLearnMethodId: integer('move_learn_method_id').notNull().references(() => moveLearnMethods.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
  description: text('description'),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveLearnMethodId, t.languageId] }),
}));
```

#### Pokémon

A PokeAPI separa Pokémon em 3 endpoints. O banco espelha isso:

- **`pokemon`** — dados de combate e sprite (stats, tipos, height, weight, abilities, sprites)
- **`pokemon_species`** — dados de lore e taxonomia (flavor text, genus, color, habitat, evolution chain)
- **`pokemon_forms`** — dados visuais do form (sprites específicos, se é default/battle-only)

```ts
export const pokemon = sqliteTable('pokemon', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  height: integer('height').notNull(),
  weight: integer('weight').notNull(),
  hp: integer('hp').notNull(),
  atk: integer('atk').notNull(),
  def: integer('def').notNull(),
  spa: integer('spa').notNull(),
  spd: integer('spd').notNull(),
  spe: integer('spe').notNull(),
  spriteDefault: text('sprite_default'),
  spriteShiny: text('sprite_shiny'),
  spriteArtwork: text('sprite_artwork'),
  isDefault: integer('is_default', { mode: 'boolean' }).notNull().default(true),
  order: integer('order'),
  evolvesFromPokemonId: integer('evolves_from_pokemon_id'), // self-FK, linked-list head
});

// Linked-list evolution: one row per pokémon-with-predecessor
export const pokemonEvolutionDetails = sqliteTable('pokemon_evolution_details', {
  pokemonId:       integer('pokemon_id').primaryKey().references(() => pokemon.id),
  trigger:         text('trigger').notNull(),
  minLevel:        integer('min_level'),
  itemId:          integer('item_id').references(() => items.id),
  heldItemId:      integer('held_item_id').references(() => items.id),
  knownMoveId:     integer('known_move_id').references(() => moves.id),
  knownMoveTypeId: integer('known_move_type_id').references(() => types.id),
  minHappiness:    integer('min_happiness'),
  timeOfDay:       text('time_of_day'),
  gender:          integer('gender'),
  location:        text('location'),
  source:          text('source', { enum: ['pokeapi','edited','custom'] }).notNull().default('pokeapi'),
});

export const pokemonTypes = sqliteTable('pokemon_types', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  typeId: integer('type_id').notNull().references(() => types.id),
  slot: integer('slot').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.slot] }),
}));

export const pokemonAbilities = sqliteTable('pokemon_abilities', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  slot: integer('slot').notNull(),
  isHidden: integer('is_hidden', { mode: 'boolean' }).notNull().default(false),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.abilityId] }),
}));

export const pokemonStats = sqliteTable('pokemon_stats', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  statName: text('stat_name').notNull(),
  baseStat: integer('base_stat').notNull(),
  effort: integer('effort').notNull().default(0),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.statName] }),
}));

export const pokemonMoves = sqliteTable('pokemon_moves', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  moveId: integer('move_id').notNull().references(() => moves.id),
  learnMethodId: integer('learn_method_id').notNull().references(() => moveLearnMethods.id),
  level: integer('level'),
  versionGroupId: integer('version_group_id').references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.moveId, t.learnMethodId, t.level] }),
}));
```

#### Pokemon Species

```ts
export const pokemonSpecies = sqliteTable('pokemon_species', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  genus: text('genus'),
  dexNumber: integer('dex_number').notNull(),
  isBaby: integer('is_baby', { mode: 'boolean' }).notNull().default(false),
  isLegendary: integer('is_legendary', { mode: 'boolean' }).notNull().default(false),
  isMythical: integer('is_mythical', { mode: 'boolean' }).notNull().default(false),
  habitat: text('habitat'),
  color: text('color'),
  shape: text('shape'),
  growthRate: text('growth_rate'),
  baseHappiness: integer('base_happiness'),
  captureRate: integer('capture_rate'),
  genderRate: integer('gender_rate'),
  hatchCounter: integer('hatch_counter'),
  generationId: integer('generation_id').references(() => generations.id),
  order: integer('order'),
});

export const pokemonFlavorTexts = sqliteTable('pokemon_flavor_texts', {
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.speciesId, t.languageId, t.versionGroupId] }),
}));

export const pokemonGenera = sqliteTable('pokemon_genera', {
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  genus: text('genus').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.speciesId, t.languageId] }),
}));

export const eggGroups = sqliteTable('egg_groups', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
});

export const speciesEggGroups = sqliteTable('species_egg_groups', {
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  eggGroupId: integer('egg_group_id').notNull().references(() => eggGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.speciesId, t.eggGroupId] }),
}));
```

#### Pokemon Forms

```ts
export const pokemonForms = sqliteTable('pokemon_forms', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  formName: text('form_name'),
  isDefault: integer('is_default', { mode: 'boolean' }).notNull().default(false),
  isBattleOnly: integer('is_battle_only', { mode: 'boolean' }).notNull().default(false),
  isMega: integer('is_mega', { mode: 'boolean' }).notNull().default(false),
  order: integer('order'),
  spriteDefault: text('sprite_default'),
  spriteShiny: text('sprite_shiny'),
  spriteArtwork: text('sprite_artwork'),
});
```

#### Items

```ts
export const items = sqliteTable('items', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  cost: integer('cost'),
  flingPower: integer('fling_power'),
  category: text('category'),
  spriteUrl: text('sprite_url'),
});

export const itemNames = sqliteTable('item_names', {
  itemId: integer('item_id').notNull().references(() => items.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.itemId, t.languageId] }),
}));

export const itemFlavorTexts = sqliteTable('item_flavor_texts', {
  itemId: integer('item_id').notNull().references(() => items.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.itemId, t.languageId, t.versionGroupId] }),
}));

export const itemEffects = sqliteTable('item_effects', {
  itemId: integer('item_id').notNull().references(() => items.id),
  effect: text('effect').notNull(),
  shortEffect: text('short_effect').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.itemId, t.languageId] }),
}));
```

#### Natures

```ts
export const natures = sqliteTable('natures', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  increasedStat: text('increased_stat'),
  decreasedStat: text('decreased_stat'),
  likesFlavor: text('likes_flavor'),
  hatesFlavor: text('hates_flavor'),
});
```

#### Trainer

```ts
export const trainers = sqliteTable('trainers', {
  id: text('id').primaryKey(),
  username: text('username').notNull().unique(),
  passwordHash: text('password_hash').notNull(),
  role: text('role', { enum: ['viewer', 'editor'] }).notNull().default('viewer'),
  isNpc: integer('is_npc', { mode: 'boolean' }).notNull().default(false),
  bannedAt: integer('banned_at', { mode: 'timestamp' }),  // null = ativo, timestamp = banido
  createdAt: integer('created_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

// Sessions (Lucia Auth)
export const sessions = sqliteTable('sessions', {
  id: text('id').primaryKey(),
  userId: text('user_id').notNull().references(() => trainers.id, { onDelete: 'cascade' }),
  expiresAt: integer('expires_at').notNull(),
});

export const teams = sqliteTable('teams', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  trainerId: text('trainer_id').notNull().references(() => trainers.id),
  name: text('name').notNull(),
  createdAt: integer('created_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

export const teamMembers = sqliteTable('team_members', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  teamId: integer('team_id').notNull().references(() => teams.id),
  slot: integer('slot').notNull(),
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  natureId: integer('nature_id').notNull().references(() => natures.id),
  heldItemId: integer('held_item_id').references(() => items.id),
  move1Id: integer('move1_id').references(() => moves.id),
  move2Id: integer('move2_id').references(() => moves.id),
  move3Id: integer('move3_id').references(() => moves.id),
  move4Id: integer('move4_id').references(() => moves.id),
});
```

#### Battles & Audit

```ts
export const battles = sqliteTable('battles', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  trainerId1: text('trainer_id_1').notNull().references(() => trainers.id),
  trainerId2: text('trainer_id_2').notNull().references(() => trainers.id),
  teamId1: integer('team_id_1').notNull().references(() => teams.id),
  teamId2: integer('team_id_2').notNull().references(() => teams.id),
  winnerId: text('winner_id').references(() => trainers.id),
  log: text('log').notNull().default(''),
  playedAt: integer('played_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

export const edits = sqliteTable('edits', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  trainerId: text('trainer_id').notNull().references(() => trainers.id),
  tableName: text('table_name').notNull(),
  recordId: integer('record_id').notNull(),
  previousData: text('previous_data'),
  newData: text('new_data'),
  editedAt: integer('edited_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

export const importLog = sqliteTable('import_log', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  entityType: text('entity_type').notNull(),
  pokeapiId: integer('pokeapi_id').notNull(),
  action: text('action', { enum: ['inserted', 'updated', 'skipped'] }).notNull(),
  reason: text('reason'),
  timestamp: integer('timestamp', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});
```

---

## Módulo de Importação — PokeAPI

A lógica de importação vive em `src/lib/pokeapi/` e é acionada via **painel web** (`/import`).

### Client da PokeAPI (`src/lib/pokeapi/client.ts`)

```ts
const BASE_URL = 'https://pokeapi.co/api/v2';

// Entidades principais
export async function fetchPokemon(id: number): Promise<PokeApiPokemon> { ... }
export async function fetchPokemonSpecies(id: number): Promise<PokeApiPokemonSpecies> { ... }
export async function fetchPokemonForm(id: number): Promise<PokeApiPokemonForm> { ... }
export async function fetchType(id: number): Promise<PokeApiType> { ... }
export async function fetchMove(id: number): Promise<PokeApiMove> { ... }
export async function fetchAbility(id: number): Promise<PokeApiAbility> { ... }
export async function fetchItem(id: number): Promise<PokeApiItem> { ... }
export async function fetchNature(id: number): Promise<PokeApiNature> { ... }
export async function fetchEvolutionChain(id: number): Promise<PokeApiEvolutionChain> { ... }

// Game metadata
export async function fetchGeneration(id: number): Promise<PokeApiGeneration> { ... }
export async function fetchVersionGroup(id: number): Promise<PokeApiVersionGroup> { ... }
export async function fetchVersion(id: number): Promise<PokeApiVersion> { ... }
export async function fetchRegion(id: number): Promise<PokeApiRegion> { ... }
export async function fetchPokedex(id: number): Promise<PokeApiPokedex> { ... }
export async function fetchLanguage(id: number): Promise<PokeApiLanguage> { ... }

// Location & encounter data
export async function fetchLocation(id: number): Promise<PokeApiLocation> { ... }
export async function fetchLocationArea(id: number): Promise<PokeApiLocationArea> { ... }
export async function fetchEncounterMethod(id: number): Promise<PokeApiEncounterMethod> { ... }
export async function fetchEncounterCondition(id: number): Promise<PokeApiEncounterCondition> { ... }

// Listagem (para imports em lote)
export async function fetchList(endpoint: string, limit?: number): Promise<NamedAPIResourceList> { ... }
```

### Import Service — Botões do Painel

| Botão no Painel | O que faz | Quantidade aproximada |
| --- | --- | --- |
| **Importar Game Data** | Languages, Generations, Regions, Version Groups, Versions, Pokedexes, Pokedex Entries, Natures, Encounter Methods, Encounter Conditions, Move Learn Methods, Egg Groups | ~7,000 registros (leve) |
| **Importar todos os Tipos** | Types + type_effectiveness + type_names | ~20 × 3 = ~60 registros |
| **Importar todas as Abilities** | Abilities + ability_names + ability_flavor_texts + ability_effects | ~300 × ~8 = ~2,400 registros |
| **Importar todos os Moves** | Moves + move_names + move_flavor_texts + move_effects | ~900 × ~8 = ~7,200 registros |
| **Importar todos os Items** | Items + item_names + item_flavor_texts + item_effects | ~1,000 × ~8 = ~8,000 registros |
| **Importar Pokémon (individual)** | Species + pokemon + forms + types + abilities + stats + moves + flavor texts + evolution chain | ~5,000+ registros por importação em lote |
| **Importar Locations & Encounters** | Locations + location_areas + encounter methods + encounters | ~100,000+ registros (pesado) |
| **Re-importar tudo (source=pokeapi)** | Atualiza apenas registros com `source='pokeapi'`, protege `edited`/`custom` | Variável |

---

## Rotas

```
/
├── search                    ← busca global (home page)
│
├── wiki/
│   ├── pokemon/              ← lista de todos os Pokémon e detalhe por slug
│   ├── moves/
│   ├── abilities/
│   ├── items/
│   ├── types/
│   ├── natures/
│   ├── locations/            ← lista por região + detalhe com encounter table
│   │   ├── +page.svelte
│   │   └── [slug]/
│   ├── games/                ← gerações e version groups
│   │   ├── +page.svelte     ← lista agrupada por geração
│   │   └── [slug]/          ← ex: /wiki/games/firered-leafgreen
│   ├── pokedex/              ← pokédexes regionais e customizados
│   │   ├── +page.svelte     ← lista de pokédexes disponíveis
│   │   └── [slug]/          ← ex: /wiki/pokedex/hoenn (Pokémon em ordem regional)
│   └── mechanics/
│       ├── damage-formula
│       ├── natures
│       ├── status-conditions
│       └── evolution
│
├── search/
│   └── ?q=...&entity_type=...&type_name=...&version_group=firered-leafgreen
│
├── api/
│   ├── search/
│   │   └── suggestions/
│   ├── editor/
│   │   └── base-form/[speciesId]  ← GET prefill da Base Form para New Pokemon
│   └── import/
│       ├── POST /            ← dispara importação
│       └── GET /status       ← status de imports em andamento
│
├── editor/                   ← protegido por role=editor
│   ├── types/
│   ├── natures/
│   ├── abilities/
│   ├── items/
│   ├── moves/
│   ├── pokedex/
│   ├── pokemon/              ← sub-editors: types, abilities, forms, evolution, learnset
│   └── species/              ← auto-Base-Form, egg groups
│
├── import/                   ← painel de importação (role=editor)
│
├── battle/
│   ├── new
│   ├── join
│   └── [id]
│
└── trainer/
    ├── login
    ├── register
    └── [username]/
        ├── teams
        ├── battles
        └── edits
```

### Rotas de Wiki implementadas

| Rota | Descrição |
| --- | --- |
| `/wiki/pokemon` | Lista todos os Pokémon importados (nome, sprite, tipos). **Nota: chama-se "Pokémon", não "Pokédex" — a distinção é importante.** |
| `/wiki/pokemon/[slug]` | Detalhe de um Pokémon: stats, tipos, abilities, learnset por versão, evolution tree. |
| `/wiki/moves` | Lista de todos os moves com tipo, categoria, power, accuracy, pp. |
| `/wiki/moves/[slug]` | Detalhe de um move com Pokémon que o aprendem. |
| `/wiki/abilities` | Lista de todas as abilities. |
| `/wiki/abilities/[slug]` | Detalhe de uma ability com Pokémon que a possuem. |
| `/wiki/types` | Grid de todos os tipos com ícone e cor. |
| `/wiki/types/[slug]` | Tabela de efetividade + Pokémon do tipo. |
| `/wiki/natures` | Tabela das 25 natures com stat modifier e flavor. |
| `/wiki/items` | Grid de itens com sprite, categoria e preço. |
| `/wiki/items/[slug]` | Detalhe de um item com flavor text por versão e efeito. |
| `/wiki/locations` | Lista de localizações agrupadas por região. |
| `/wiki/locations/[slug]` | Encounter table filtrada por versão, com método e level range. |
| `/wiki/games` | Lista de gerações e version groups agrupada por geração. |
| `/wiki/games/[slug]` | Detalhe de um version group (ex: FireRed/LeafGreen) com suas versões individuais. |
| `/wiki/pokedex` | Lista de pokédexes disponíveis (nacional, regionais, customizados). Requer import de Game Data. |
| `/wiki/pokedex/[slug]` | Pokémon em ordem numérica daquela pokédex regional. Permite criar pokédexes customizados no Editor. |

### Distinção Pokémon vs Pokédex

- **`/wiki/pokemon`** — catálogo global de todos os Pokémon importados, sem ordenação de dex específica
- **`/wiki/pokedex/[slug]`** — ordenação regional ou customizada; o Editor permite criar pokédexes próprias para o ROM hack (ex: `/wiki/pokedex/yaxu`)

### Query params novos na busca

- **`version_group`**: Filtra flavor texts e learnsets por versão (ex: `?version_group=firered-leafgreen`)
- **`region`**: Filtra locations por região (ex: `?region=kanto`)

---

## Ordem de Desenvolvimento

### Bloco 0 — Setup e Fundação ✅

- [x] Inicializar projeto SvelteKit com TypeScript
- [x] Configurar TailwindCSS v4 (via `@tailwindcss/vite`)
- [x] Configurar Drizzle ORM + SQLite (WAL mode)
- [x] Criar schema modular do banco em `src/lib/db/schema/` (6 módulos, 58 tabelas)
- [x] Configurar Docker multi-stage (`docker/Dockerfile`, `docker-compose.yml`)
- [x] Configurar ambiente Nix (`flake.nix` com Node.js 20 + SQLite)
- [x] Criar design tokens em `src/app.css` conforme DESIGN.md
- [x] Validar build e deploy localmente (`npm run build` ✅, `npm run dev` ✅)

### Bloco 1 — Game Metadata & Importação ✅

- [x] Implementar `src/lib/pokeapi/types.ts` — interfaces TS para todas as respostas da PokeAPI
- [x] Implementar `src/lib/pokeapi/client.ts` — HTTP client com cache in-memory, rate limiting (80 req/min), retry com backoff exponencial
- [x] Implementar `src/lib/pokeapi/mappers.ts` — transformações JSON → Drizzle (game metadata, core data, pokémon, evolution chains, locations)
- [x] Implementar `src/lib/pokeapi/import-service.ts` — `ensure*` resolvers para game metadata, `protectedUpsert` com proteção por `source`, batch imports
- [x] Implementar `src/lib/pokeapi/import-pokemon.ts` — `ensure*` resolvers para types, abilities, moves, items, pokémon, species, evolution chains com resolução recursiva de dependências
- [x] Aplicar schema ao banco via `db:push` (58 tabelas criadas)
- [x] Verificar: importação de Raichu (#26) auto-importa Pichu, Pikachu, 109 moves, 18 types, evo chain completa
- [x] Verificar: `PRAGMA foreign_key_check` — zero violações

### Bloco 2 — Painel de Importação Web ✅

- [x] Layout base (`+layout.svelte`) com sidebar e breadcrumbs (Gameboy Theme)
- [x] Página `/import` com suporte a API routes (Server Actions)
- [x] Botões "Import All Game Data", "Types", "Abilities", "Moves", "Items"
- [x] Importação individual de Pokémon e por faixa (range)
- [x] Dashboard de status (`ImportStatusGrid.svelte`) com contagens por source
- [x] Log de importações em tempo real (`ImportLog.svelte`)
- [x] SSE (Server-Sent Events) para log em tempo real via `EventSource` + `ReadableStream`

### Bloco 3.1 — Wiki Pública (Listagens e Componentes) ✅

- [x] Landing page `/wiki`
- [x] Componentes base (PokemonCard, TypeBadge, StatBar, EvolutionTree)
- [x] Pokédex Completa (`/wiki/pokemon`)
- [x] Lista Global de Movimentos (`/wiki/moves`) com cards clicáveis para `/wiki/moves/[slug]`
- [x] Lista de Habilidades (`/wiki/abilities`) com cards clicáveis para `/wiki/abilities/[slug]`
- [x] Lista de Tipos (`/wiki/types`) com cards clicáveis para `/wiki/types/[slug]`
- [x] Detalhes do Pokémon (`/wiki/pokemon/[slug]`) com stats, types, abilities, Pokédex e learnset por versão
- [x] Detalhes do Movimento (`/wiki/moves/[slug]`) com lista de Pokémon que aprendem
- [x] Detalhes da Habilidade (`/wiki/abilities/[slug]`) com lista de Pokémon
- [x] Detalhes do Tipo (`/wiki/types/[slug]`) com fraquezas e lista de Pokémon

### Bloco 3.2 — Wiki Pública (Mecânicas Avançadas) ✅

- [x] Seletor de `version_group` para flavor texts e learnsets do Pokémon
- [x] Componente visual de Árvore de Evolução (`/wiki/pokemon/[slug]`) — layout top-down
- [x] Breadcrumbs clicáveis com links navegáveis em todas as páginas (incluindo botão "← BACK")
- [x] Links internos da learnset para página de move corrigidos (usando slug)
- [x] Flavor texts persistidos corretamente no import (incluindo reimports futuros)
- [x] Correção de cadeia de evolução complexa (Eevee e todas as evoluções)
- [x] Cards das listagens (Moves, Abilities, Types) redirecionam para páginas de detalhe (`[slug]`)
- [x] Classe utilitária `.card-interactive` no `app.css` — efeito de hover universal (outline `--theme-text-accent` + sombra + movimento -2px) aplicado em todos os cards
- [x] Cards de Types redesenhados com ícones SV (Scarlet/Violet) e barra de cor sutil na parte inferior
- [x] Header do detalhe de Type (`/wiki/types/[slug]`) com ícone grande e barra de cor
- [x] Componente `MoveCategoryIcon.svelte` — ícones Physical/Special/Status em todas as páginas de moves
- [x] Evolution Tree com layout adaptativo: linear, ramificado, multi-row — formas regionais em tabs separadas
- [x] Fallback de sprites para Megas customizados e ícone Stellar local
- [x] Toggle Shiny nas sprites com fundo roxo sutil
- [x] Página `/wiki/natures` — tabela das 25 natures com modificadores e flavor
- [x] Páginas de Items — `/wiki/items` (grid com search) e `/wiki/items/[slug]` (flavor text, efeito)
- [x] Fix Evolution Chain Eevee — dedup por `(from,to)` descarta arestas level-up redundantes
- [x] Páginas de Locations — `/wiki/locations` (lista por região) e `/wiki/locations/[slug]` (encounter table)
- [x] SearchBar compartilhado em todas as páginas de listagem (Pokémon, Moves, Abilities, Types, Items, Locations)
- [x] SSE movido para `+layout.svelte` — indicador de importação persistente na sidebar (`ImportActivity.svelte`)
- [x] `.gitignore` criado — `build/` e `.svelte-kit/` removidos do git
- [x] Páginas de Games — `/wiki/games` (gerações e version groups) e `/wiki/games/[slug]` (detalhe do version group)
- [x] Páginas de Pokédex — `/wiki/pokedex` (lista de pokédexes com empty-state) e `/wiki/pokedex/[slug]` (Pokémon em ordem regional)
- [x] Renomeação: `/wiki/pokemon` passa a se chamar "Pokémon" (não "Pokédex") — reserva o nome Pokédex para os pokédexes regionais/customizados

### Bloco 4 — Autenticação ✅

- [x] Instalar e configurar Lucia Auth v3 + `@lucia-auth/adapter-drizzle`
- [x] Adicionar tabela `sessions` ao schema + `db:push`
- [x] `src/lib/auth/lucia.ts` — inicialização com adapter Drizzle
- [x] `src/lib/auth/guards.ts` — helpers `requireAuth` e `requireEditor`
- [x] `src/hooks.server.ts` — validação de session em cada request, injeção de `locals.user`
- [x] Rotas `/trainer/register` e `/trainer/login` com hash de senha (node:crypto scrypt)
- [x] Rota `/trainer/logout` — invalida session
- [x] Proteger `/import` e `/editor` com `requireEditor`
- [x] Sidebar mostra estado de auth (LOGIN / username / badge EDITOR)
- [x] Bootstrap: primeiro registro é auto-promovido a role 'editor'

### Bloco 4.5 — Trainer Pages ✅

- [x] Perfil público de trainer em `/trainer/[username]` (role badge, join date, stats, badge YOU)
- [x] Histórico de edições em `/trainer/[username]/edits` (tabela paginada por trainer)
- [x] Lista global de trainers em `/trainer` (usuários reais + NPCs em sections separadas)
- [x] Controles de admin (editor-only) na lista: PROMOTE / DEMOTE / BAN / UNBAN
- [x] Criação e deleção de NPCs (game characters) com placeholder password hash
- [x] Schema: coluna `bannedAt` em `trainers` (null = ativo)
- [x] Banimento imediato — `hooks.server.ts` invalida sessão do usuário banido no próximo request
- [x] Login bloqueia contas banidas e contas NPC com mensagem específica
- [x] Guardas: não pode banir/rebaixar a si mesmo; não pode rebaixar o último editor
- [x] Componente `NavTab.svelte` — estilo unificado dos botões da top bar

### Bloco 5 — Editor ✅

- [x] Formulários de edição para todas as entidades (Pokémon, moves, abilities, items, types)
- [x] Validação server-side com Zod
- [x] Gravação de edições na tabela `edits` com `previous_data` e `new_data`
- [x] Marcação automática `source='edited'` ao salvar
- [x] Criação de pokédexes customizadas (`source='custom'`) — ex: pokédex do ROM hack
- [x] Proteção por role=editor em todos os form actions
- [x] Botão RE-SOURCE em cada entidade editada (reverte edits ao estado PokeAPI canonical via forceRefresh)

### Bloco 5.1 — Editor Phase 2: Pokédex / Moves / Pokémon ✅

- [x] Editor de Pokédex em `/editor/pokedex` com criação inicial de dex custom (ROM hack) e sub-editor de entries
- [x] Editor de Moves em `/editor/moves` com FK dropdowns (type, generation) e sub-form de English `moveEffects` (upsert via `onConflictDoUpdate`)
- [x] Editor de Pokémon em `/editor/pokemon` com main row + sub-forms para `pokemon_types` (slot 1 + 2) e `pokemon_abilities` (slot 1, 2, hidden)

### Bloco 5.2 — Editor Phase 3a: Species + Base Form ✅

- [x] Editor de Pokémon Species em `/editor/species` com todos os campos flat (genus, dexNumber, flags, habitat, color, shape, growth, capture/gender/happiness/hatch)
- [x] Auto-criação da **Base Form** na criação de Species (insere default `pokemon` + default `pokemon_forms` em transação)
- [x] API endpoint `/api/editor/base-form/[speciesId]` para prefill client-side
- [x] Form de **New Pokémon** copia stats/types/abilities/learnset da Base Form ao selecionar Species

### Bloco 5.3 — Editor Phase 3b/c: Forms + Evolution + Learnset ✅

- [x] Sub-editor de **Forms** inline em `/editor/pokemon/[slug]` (formName, isDefault, isBattleOnly, isMega, sprites)
- [x] **Migração para linked-list evolution**: drop de `evolution_chains` e `evolution_details`; novas colunas `pokemon.evolvesFromPokemonId` + tabela `pokemon_evolution_details` (uma row por filho-com-predecessor)
- [x] PokeAPI import refatorado: `populateChainLinkedList()` popula o linked-list automaticamente em cada importação (sem script de backfill)
- [x] Sub-editor de **Evolution** inline com PREDECESSOR/SUCCESSORS panels (form-specific evolutions agora corretas: Galarian Meowth → Perrserker)
- [x] Sub-editor de **Learnset** inline com `?vg=` URL param, COPY FROM VG, CLEAR THIS VG, add/remove de entries

### Bloco 5.4 — RE-SOURCE Button + forceRefresh ✅

- [x] Botão **RE-SOURCE FROM POKEAPI** em cada `/editor/<entity>/[slug]` quando `source='edited'`
- [x] Todas as `ensureX()` no `import-pokemon.ts` / `import-service.ts` aceitam `{ forceRefresh: true }` — pula early-return, limpa sub-rows, re-upsert main
- [x] Pokémon RE-SOURCE reverte main + types + abilities + stats + moves + forms (não custom) + linked-list evolution
- [x] Helper compartilhado `src/lib/editor/resource.ts` (`reimportEntity()`)

### Bloco 5.5 — Polish ✅

- [x] Egg groups multi-select no Species editor (sub-form `updateEggGroups` + UI)
- [x] Move-picker autocomplete (datalist) no Learnset editor — substitui o `<select>` de 610 opções

### Bloco 6 — Busca Global

- [ ] Busca facetada com filtros por versão e região
- [ ] Busca fuzzy com Fuse.js
- [ ] Autocomplete `/api/search/suggestions`

### Bloco 7 — Engine de Batalha

- [ ] Interfaces em `src/lib/engine/types.ts`
- [ ] Fórmula de dano Gen 3
- [ ] Cálculo de stat stages e type effectiveness
- [ ] Loop de turno e validadores anti-cheat
- [ ] Testes unitários

### Bloco 8 — Batalhas Online (Socket.io)

- [ ] Integrar Socket.io ao SvelteKit
- [ ] Rooms, matchmaking, team select
- [ ] UI de batalha
- [ ] Persistência de resultados

### Bloco 9 — Deploy e Operações

- [ ] Ajustar Dockerfile para build otimizado
- [ ] Configurar Coolify (volumes, variáveis, domínio)
- [ ] Validar que banco vazio funciona e importação via painel web
- [ ] Backup e CI

---

## Notas de Decisão Arquitetural

### Por que importação sob demanda em vez de seed scripts?

1. **Zero adaptação manual**: O mapeamento é direto do JSON da PokeAPI para o banco via `mappers.ts`.
2. **Controle total**: Importe só o que precisa. Quer só Gen 3? Importe só species 1-386.
3. **Proteção de dados customizados**: O campo `source` protege edições manuais contra sobrescrita.
4. **Sem dependência de scripts externos**: A lógica vive dentro do app.

### Por que espelhar a estrutura da PokeAPI?

A PokeAPI separa Pokemon, Pokemon Species e Pokemon Forms em endpoints distintos porque representam conceitos distintos. Espelhar isso no banco significa mapeamento 1:1 com o JSON, sem perda de dados (flavor texts por versão, evoluções ricas, abilities com efeitos).

### Por que incluir encounters (~100,000 registros)?

Para um ROM hack simulator, saber onde cada Pokémon aparece, em qual versão, com qual método e level range, é fundamental para: (1) a wiki ser uma referência completa, (2) o validador do simulador checar se um Pokémon é obtível em determinada versão, (3) futuras features como randomizer ou симulação de rotas.

### Por que localização apenas em inglês?

Começar com inglês simplifica o seed e o armazenamento. O schema usa FKs para `languages.id` em todas as tabelas de nomes, então adicionar português ou outros idiomas é questão de importar mais registros — sem mudança de schema.

### Por que usar FKs para `language_id` e `version_group_id` em vez de texto?

FKs garantem integridade referencial, permitem JOINs eficientes, e facilitam filtrar por versão do jogo (ex: "mostre flavor texts de FireRed/LeafGreen apenas"). O campo texto era frágil — um typo em `'firered-leafgreen'` quebraria queries.

---

## Licença e Créditos

- **Pokémon** é propriedade da Nintendo / Game Freak / The Pokémon Company. Este projeto é uma ferramenta de fã sem fins comerciais.
- **PokeAPI**: Dados obtidos via PokeAPI (`pokeapi.co`), licenciados sob BSD 3-Clause. Sprites servidos da PokeAPI CDN.
- Todas as tecnologias utilizadas são **open-source** (SvelteKit, Drizzle, Lucia, Socket.io, Fuse.js, TailwindCSS).

---

*Última atualização do plano: 2026-05-11 (revisão pós-Bloco 5.5)*
*Versão da stack: 2.0 (TypeScript / SvelteKit — Importação sob demanda da PokeAPI — Game metadata completo — Locations & Encounters)*
*Progresso: Blocos 0–5.5 concluídos — Editor completo (todas as entidades editáveis, linked-list evolution, RE-SOURCE, polish). Próximo: Bloco 6 (Busca Global)*