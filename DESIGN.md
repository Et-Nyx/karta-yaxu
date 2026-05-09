# Design System — Karta Yaxu

## 1. Filosofia de Modularidade

### Princípios

1. **Componente é lego brick** — recebe dados via props, não busca dados internamente. A página (`+page.server.ts`) carrega os dados e passa para os componentes.
2. **Um componente, múltiplos contextos** — `PokemonCard` funciona na wiki, na busca, no editor, e no team select. A diferença é controlada por props (`variant`).
3. **Slots para variação** — Componentes usam `<slot>` para áreas customizáveis, não criam 3 versões do mesmo card.
4. **Zero lógica de rota dentro de componentes** — Componentes não sabem em qual página estão. Navegação é feita via `<a>` com `href` passado como prop.
5. **Tipos compartilhados** — Interfaces TypeScript em `src/lib/types/` usados tanto por `+page.server.ts` quanto por componentes Svelte.

### Hierarquia de Componentes

```
Átomos        → Elementos indivisíveis (TypeBadge, StatBar, Sprite, Badge)
Moléculas      → Combinação de átomos (PokemonCard, MoveCard, FlavorTextList)
Organismos    → Combinação de moléculas + layout (Sidebar, FilterSidebar, ImportPanel)
Templates     → Layout de página (WikiDetailLayout, WikiListLayout, GameboyFrame)
```

---

## 2. Design Tokens

### Paleta de Cores por Tipo

Baseadas na referência `.toml` do usuário:

```
normal:    #9FA19F    fire:      #E62829    water:     #2980EF
electric:  #FAC000    grass:     #3FA129    ice:       #3DD8FF
fighting:  #FF8000    poison:    #9141CB    ground:    #915121
flying:    #81B9EF    psychic:   #EF4179    bug:       #91A119
rock:      #AFA981    ghost:     #704170    dragon:    #5060E1
dark:      #4F3F3D    steel:     #60A1B8    fairy:     #EF70EF
stellar:   #83CFC5
```

### Temas Estruturais (CSS Variables)

O site possui 3 temas principais. O público (Wiki) usa Claro/Escuro monocromático. As áreas administrativas (Editor/Import) forçam o tema "Admin Green".

**1. Light Theme (Padrão Público - Monocromático)**
```
--bg-primary:        #F0F0F0       (fundo principal)
--bg-secondary:      #FFFFFF       (fundo de cards e containers)
--bg-tertiary:       #E0E0E0       (fundo de headers e painéis destacados)
--text-primary:      #111111       (texto principal)
--text-secondary:    #444444       (texto secundário, labels)
--border-primary:    #000000       (bordas duras sempre pretas)
```

**2. Dark Theme (Público - Monocromático)**
```
--bg-primary:        #1A1A1A
--bg-secondary:      #2A2A2A
--bg-tertiary:       #111111
--text-primary:      #EEEEEE
--text-secondary:    #AAAAAA
--border-primary:    #000000       (ou um cinza muito escuro dependendo do contraste)
```

**3. Admin Theme (Gameboy Green - Áreas de Edição)**
```
--bg-primary:        #8BAC0F       (fundo verde clássico)
--bg-secondary:      #9BBC0F       (cards ligeiramente mais claros)
--bg-tertiary:       #0F380F       (headers verde muito escuro)
--text-primary:      #0F380F       (texto principal escuro para contraste no fundo claro)
--text-secondary:    #306230       (texto secundário)
--text-inverse:      #9BBC0F       (texto claro para o header escuro)
--border-primary:    #000000
```

Todas as interfaces compartilham as mesmas sombras duras:
```
--shadow-card:       3px 3px 0px #000000  (sombra dura offset)
--shadow-card-hover: 4px 4px 0px #000000
```

### Tipografia

```
Títulos e headers:    "Press Start 2P", 16px–24px, line-height: 1.4
Corpo de texto:       "Inter", 14px–16px, line-height: 1.6
Código/dados:         "JetBrains Mono", 13px, line-height: 1.5
Badges e labels:       "Press Start 2P", 10px–12px, line-height: 1.2
Flavor texts:          "Inter", 14px, italic
Números/stats:         "JetBrains Mono", 14px, tabular-nums
```

### Escala de Espaçamento

```
--space-1:  4px    (gap mínimo entre elementos inline)
--space-2:  8px    (padding interno de badges, gap entre labels)
--space-3:  12px   (padding de cards, gap entre stat bars)
--space-4:  16px   (margin entre cards em grid, padding de seções)
--space-5:  24px   (margin entre seções de página, gap sidebar-conteúdo)
--space-6:  32px   (margin entre header e conteúdo)
--space-8:  48px   (margin entre páginas distintas)
```

### Breakpoints

```
Mobile:    < 768px    → sidebar vira drawer, cards 1 coluna, tabs scroll horizontal
Tablet:    768–1024px → sidebar colapsada (ícones), 2 colunas de cards
Desktop:   > 1024px  → sidebar expandida, 3–4 colunas, layout completo
```

### Sombras e Bordas (Estilo Gameboy)

```
Card:            border: 2px solid var(--border-primary)
                  box-shadow: var(--shadow-card)
Card (hover):     box-shadow: var(--shadow-card-hover)
Gameboy frame:    border: 4px solid #292929
                  box-shadow: inset 0 0 20px rgba(0,0,0,0.5)
Type badge:       border-radius: 4px
Buttons:          border: 2px solid, box-shadow: 3px 3px 0px #000
Input fields:     border: 2px solid var(--border-primary)
                  background: var(--bg-secondary)
```

---

## 3. Layout System

### Gameboy Frame

O jogo inteiro é envolvido por um frame que simula um Gameboy:

```
┌────────────────────────────────────────────────────────────────┐
│  ╔══════════════════════════════════════════════════════════╗  │
│  ║  KARTA YAXU                                    🔍 Login ║  │ ← Header (Press Start 2P)
│  ╠══════════════════════════════════════════════════════════╣  │
│  ║  Wiki  │  Editor  │  Import  │  Battle                ║  │ ← Nav tabs
│  ╠════════╦═══════════════════════════════════════════════╣  │
│  ║        ║  Raiz > Wiki > Pokémon > Bulbasaur            ║  │ ← Breadcrumbs
│  ║ SIDE   ║───────────────────────────────────────────────║  │
│  ║ BAR    ║                                               ║  │
│  ║        ║  <conteúdo da página>                          ║  │ ← Content (max-width 960px)
│  ║ Types  ║                                               ║  │
│  ║ Moves  ║                                               ║  │
│  ║ ...    ║                                               ║  │
│  ║        ║                                               ║  │
│  ║ ────── ║                                               ║  │
│  ║ Version║                                               ║  │
│  ║ [FR/LG▾]║                                              ║  │
│  ╚════════╩═══════════════════════════════════════════════╝  │
└────────────────────────────────────────────────────────────────┘
```

### Sidebar

- **Desktop (>1024px):** 200px fixa, expandida, com texto nos links
- **Tablet (768–1024px):** 56px colapsada, apenas ícones, expande no hover
- **Mobile (<768px):** Drawer deslizante da esquerda, overlay escuro no fundo, botão hamburger no header

Seções da sidebar (sempre visíveis):
- Wiki (Pokemon, Moves, Abilities, Items, Types, Locations, Pokedex, Games, Mechanics)
- Search
- Editor (se logado como editor)
- Import (se logado como editor)
- Battle
- Trainer (Login/Perfil)

No footer da sidebar: VersionSelector (dropdown simples).

### Breadcrumbs

```
Raiz > Wiki > Pokémon > Bulbasaur
```

- Cada parte é clicável e navega para a página correspondente
- Separador: `>` em `--text-muted`
- Último item em `--text-primary`, demais em `--text-secondary`

---

## 4. Catálogo de Componentes

### Átomos

#### TypeBadge

```svelte
<TypeBadge
  type={typeListItem}           // { id, name, slug, hexColor }
  size="md"                      // "sm" | "md" | "lg"
  link={true}                    // se true, clicável → /wiki/types/{slug}
/>
```

- Fundo: `hexColor` do tipo com 90% de opacidade
- Texto: nome do tipo em branco, "Press Start 2P" 10px (sm), 12px (md), 14px (lg)
- Border-radius: 4px
- Se `link={true}`: hover adiciona box-shadow e transform scale(1.05)

#### StatBar

```svelte
<StatBar
  label="HP"                     // string
  value={45}                      // number
  max={255}                       // number, default 255
  color="#78C850"                  // string, opcional (default: calcula pelo nome do stat)
/>
```

- Barra horizontal, 100% de largura do container
- Fundo: `--bg-tertiary`
- Preenchimento: cor do stat, width = (value / max) * 100%
- Label à esquerda, valor numérico à direita (JetBrains Mono)
- Altura: 20px

Cores padrão por stat:
```
HP:  #FF0000    Atk: #F08030    Def: #F8D030
SpA: #6890F0    SpD: #78C850    Spe: #F85888
```

#### Badge

```svelte
<Badge
  text="edited"                   // string
  color="yellow"                  // string ou hex
  variant="filled"                // "filled" | "outlined"
/>
```

- fulfilled: fundo colorido, texto escuro
- outlined: borda colorida, texto colorido, fundo transparente
- Uso principal: `SourceLabel` (pokeapi=green, edited=yellow, custom=blue)

#### VersionSelect

```svelte
<VersionSelect
  versionGroups={versionGroups}   // VersionGroup[] (do load)
  selected="firered-leafgreen"    // string (slug do version_group selecionado)
  on:change={(e) => selectedVersion = e.detail}  // evento
/>
```

- Dropdown simples com estilização Gameboy
- Opções agrupadas por geração: "Gen III" > "Ruby/Sapphire", "Emerald", "FireRed/LeafGreen"
- Seleção persistida no localStorage (chave: `preferred-version`)
- Usado em: PokemonDetail (flavor texts, moves), MoveDetail, AbilityDetail, LocationDetail (troca para `versions` no lugar de `versionGroups`)

#### Sprite

```svelte
<Sprite
  src={pokemon.spriteDefault}     // URL da CDN
  alt="Bulbasaur"                // string
  shinySrc={pokemon.spriteShiny} // URL da CDN, opcional
  showShinyToggle={true}          // se true, mostra toggle "Shiny Charm"
  size="md"                      // "sm" (64px) | "md" (128px) | "lg" (256px)
/>
```

- Imagem carregada da CDN da PokeAPI (URL no banco)
- Se `showShinyToggle`: botão pequeno no canto que alterna entre `src` e `shinySrc`
- Estado do toggle persistido no localStorage (chave: `shiny-charm`)
- Loading: skeleton placeholder com fundo `--bg-tertiary` e animação de pulse
- Erro de carregamento: placeholder com ícone de pokebola

#### EvolutionArrow

```svelte
<EvolutionArrow
  trigger="level-up"              // string: 'level-up' | 'use-item' | 'trade' | ...
  condition="Level 16"            // string, opcional (ex: "Level 16", "Fire Stone", "Trade")
/>
```

- Seta visual (→) com label do trigger e condição
- Ícone pequeno para cada trigger type (⚔️ level-up, 🔶 item, 🔄 trade, etc.)
- Cor: `--text-secondary`

#### EncounterMethodIcon

```svelte
<EncounterMethodIcon method="surf" />
```

- Ícone de 16px representando o método (walk=👣, surf=🌊, old-rod=🎣, etc.)
- Usado em EncounterTable e LocationDetail

#### Pagination

```svelte
<Pagination
  page={1}                        // number, current page (1-indexed)
  totalPages={10}                  // number
  on:change={(e) => page = e.detail}
/>
```

- Estilo Gameboy: botões `<` `>` com fundo `--bg-tertiary`, texto "Press Start 2P"
- Número da página atual destacado
- Máximo de 5 números visíveis com `...` para truncar

#### SourceLabel

```svelte
<SourceLabel source="edited" />
```

- `pokeapi` → Badge green, text "PokeAPI"
- `edited` → Badge yellow, text "Edited"
- `custom` → Badge blue, text "Custom"
- Usado em: Editor (mostra origem do dado), ImportStatus

### Moléculas

#### PokemonCard

```svelte
<PokemonCard
  pokemon={pokemonListItem}       // PokemonListItem
  variant="full"                  // "compact" | "full" | "minimal"
  pokedexNumber={1}               // number, opcional (para Pokedex views)
  selectable={false}              // se true, mostra checkbox
  on:select                       // evento de seleção (para team builder)
/>
```

**Variants:**

| Variant | Layout | Conteúdo | Uso |
|---|---|---|---|
| `compact` | Horizontal, 1 linha | Sprite (64px) + Nome + Dex#+ TypeBadges | Search results, listagens, evolution chain |
| `full` | Vertical, card completo | Sprite (128px) + Nome + Dex#+ TypeBadges + HP bar | Wiki list, Pokedex list |
| `minimal` | Grid cell, ícone apenas | Sprite (48px) + Nome | Evolution chain mini, team slot vazio |

**Slot `extra`:** Conteúdo adicional abaixo do card (ex: botão de edição no Editor, badge de source no ImportStatus).

#### MoveCard

```svelte
<MoveCard
  move={moveListItem}             // MoveListItem
  variant="compact"               // "compact" | "full"
/>
```

- `compact`: Nome + TypeBadge + Power + Accuracy + PP em 1 linha
- `full`: Nome + TypeBadge + Category badge + Power + Accuracy + PP + Description (truncada)
- Category badge cores: Physical=#C03028, Special=#6890F0, Status=#A8A878

#### AbilityCard

```svelte
<AbilityCard
  ability={abilityListItem}       // AbilityListItem
  variant="compact"               // "compact" | "full"
/>
```

- `compact`: Nome + short effect
- `full`: Nome + effect completo + flavor text (com VersionSelect)

#### ItemCard

```svelte
<ItemCard
  item={itemListItem}             // ItemListItem
  variant="compact"               // "compact" | "full"
/>
```

- `compact`: Ícone + Nome + Cost
- `full`: Ícone + Nome + Cost + Category + flavor text (com VersionSelect)

#### EffectivenessGrid

```svelte
<EffectivenessGrid
  attackingTypeId={3}             // number, se mostrando ofensiva
  defendingTypeIds={null}           // number[], se mostrando defensiva
  types={allTypes}                // TypeListItem[]
  direction="attacking"           // "attacking" | "defending"
/>
```

- Grid 18×18 com cores correspondentes
- Classe ofensiva: mostra o que o tipo ataca super efetivo (×2) e o que é resistido (×0.5)
- Classe defensiva: mostra o que o tipo resiste e o que é fraco contra
- Hover em uma célula mostra o multiplicador exato
- Células ×2 em verde, ×0.5 em vermelho, ×0 em preto, ×4 em verde brilhante

#### EvolutionChain

```svelte
<EvolutionChain
  chain={evolutionChainData}      // EvolutionChainData (árvore com from/to/details)
  speciesMap={speciesData}        // Record<number, SpeciesData>
/>
```

- Renderiza árvore horizontal com setas e EvolutionArrow para condições
- Cada node é um PokemonCard variant="minimal"
- Condições: "Level 16", "Fire Stone", "Trade", "Happiness + Day", etc.
- Suporta splits (Eevee → Vaporeon/Jolteon/Flareon)

#### EncounterTable

```svelte
<EncounterTable
  encounters={encounterList}       // Encounter[]
  methods={encounterMethods}       // EncounterMethod[]
  versions={versions}             // Version[]
  pokemon={pokemon}               // PokemonListItem (para filtro)
/>
```

- Agrupado por método (Walking, Surfing, Fishing)
- Cada grupo mostra: Pokémon, Level Range, Chance%, Condições
- Filtro por version (VersionSelect acima da tabela)
- Filtro por encounter method (tabs ou FilterGroup)
- Célula de chance colorida: >50% verde, 20–50% amarelo, <20% vermelho

#### FlavorTextList

```svelte
<FlavorTextList
  texts={flavorTexts}             // FlavorText[]
  versionGroups={versionGroups}   // VersionGroup[]
/>
```

- Lista de flavor texts filtrada pela version selecionada no VersionSelect
- Cada entrada: texto em itálico + badge da versão (ex: "FireRed/LeafGreen")
- Se múltiplas versões selecionáveis: dropdown VersionSelect acima
- Ordem: versão mais recente primeiro, fallback para a última disponível

#### SearchResultItem

```svelte
<SearchResultItem
  result={searchResult}            // SearchResult (unified type)
  entityType="pokemon"            // "pokemon" | "move" | "ability" | "item" | "type" | "location"
/>
```

- Layout horizontal: ícone/tipo + nome + subtítulo (dex number, category, etc.)
- Clicável → navega para `/wiki/{entityType}/{slug}`

#### FilterGroup

```svelte
<FilterGroup
  title="Type"                    // string
  options={facetOptions}          // FacetOption[] { value, label, count }
  selected={["fire", "water"]}  // string[]
  on:change={(e) => selected = e.detail}
  type="checkbox"                 // "checkbox" | "radio" | "toggle"
/>
```

- Checkbox: multi-seleção (usado para tipos, categorias)
- Radio: seleção única (usado para geração, versão)
- Toggle: on/off booleano (usado para "Fully Evolved", "Base Form Only")
- Cada opção mostra label + count entre parênteses

#### TeamSlot

```svelte
<TeamSlot
  pokemon={pokemonListItem}       // PokemonListItem | null
  ability="Overgrow"              // string | null
  moves={["Vine Whip", "Tackle"]} // string[] | null
  slotNumber={1}                  // number 1–6
  on:click                        // evento para selecionar
/>
```

- Se `pokemon` é null: slot vazio com ícone de pokebola cinza e texto "Empty"
- Se `pokemon` está preenchido: Sprite + Nome + TypeBadges + Ability + Moves
- Clicável para abrir modal de seleção
- Usado em: Team Builder, Battle Team Select

#### PokedexEntry

```svelte
<PokedexEntry
  entry={pokedexEntry}            // { entryNumber, speciesId, caught? }
  pokemon={pokemonListItem}       // PokemonListItem | null (se não importado)
/>
```

- Número da Pokédex regional à esquerda (ex: #001)
- Nome + Sprite no centro
- Se `caught` é true: bolinha verde ao lado
- Se pokemon é null (não importado): nome em cinza + "Not imported"

### Organismos

#### Sidebar

```svelte
<Sidebar
  sections={sidebarSections}      // SidebarSection[]
  currentPath={url.pathname}      // string
  versionGroups={versionGroups}   // VersionGroup[] (para o VersionSelect do footer)
>
  <svelte:fragment slot="footer">
    <VersionSelect ... />
  </svelte:fragment>
</Sidebar>
```

- Tipos de seção: `link` (navegação normal), `group` (grupo expansível), `divider`
- Cada item: ícone + texto (desktop), apenas ícone (tablet)
- Item ativo: fundo `--bg-active`, borda esquerda `--text-accent`
- Mobile: drawer deslizante com overlay

#### FilterSidebar

```svelte
<FilterSidebar
  types={allTypes}                // TypeListItem[]
  filters={currentFilters}        // SearchFilters
  on:filterChange={(e) => ...}   // evento
/>
```

- Contém múltiplos FilterGroups colapsáveis
- Por padrão: Type (checkbox, max 2), Category (radio: Physical/Special/Status), Fully Evolved (toggle), Base Form (toggle), Sort (dropdown)
- Mobile: botão "Filters" no header abre drawer

#### ImportPanel

```svelte
<ImportPanel
  importService={importService}    // ImportService instance
/>
```

Seções:
1. **Game Data** → botão "Import All" (languages, generations, versions, regions, pokedexes, etc.)
2. **Core Data** → botões individuais: Types, Abilities, Moves, Items, Natures
3. **Pokémon** → input para pokeapi_id/nome + botão "Import Gen 3"
4. **Locations & Encounters** → botão com aviso de volume (~100k registros)
5. **Re-import** → botão "Update All (source=pokeapi only)"

Cada botão: loading spinner durante execução, resultado (success/error) inline após completar.

#### ImportStatusDashboard

```svelte
<ImportStatusDashboard
  status={importStatus}            // Record<string, SourceCounts>
/>
```

Para cada entidade (types, pokemon, moves, etc):
- Total de registros no banco
- Quebra por source: `pokeapi` (green), `edited` (yellow), `custom` (blue)
- Barra de progresso visual

#### EditorForm

```svelte
<EditorForm
  schema={pokemonSchema}           // ZodSchema
  data={pokemonData}              // Record<string, any>
  onSave={saveHandler}            // function
  source="pokeapi"                // string (para SourceLabel)
>
  <svelte:fragment slot="fields">
    <!-- campos customizados por entidade -->
  </svelte:fragment>
  <svelte:fragment slot="actions">
    <button type="submit">Save</button>
  </svelte:fragment>
</EditorForm>
```

- Validação client-side via Zod
- Ao salvar: muda `source` de `pokeapi` para `edited` automaticamente
- Mostra SourceLabel no topo do formulário
- Campos com erro: borda vermelha + mensagem abaixo

#### PokemonDetailTabs

```svelte
<PokemonDetailTabs
  tabs={[
    { id: 'about', label: 'About' },
    { id: 'stats', label: 'Stats' },
    { id: 'moves', label: 'Moves' },
    { id: 'evolution', label: 'Evolution' },
    { id: 'encounters', label: 'Encounters' },
    { id: 'forms', label: 'Forms' },
  ]}
  activeTab="about"                // string
  on:tabChange={(e) => activeTab = e.detail}
/>
```

- Tabs horizontais com estilo Gameboy (bordas pixeladas, fundo `--bg-tertiary`)
- Tab ativa: fundo `--bg-active`, texto `--text-accent`
- Em mobile: scroll horizontal com setas

### Templates

#### WikiDetailLayout

```svelte
<WikiDetailLayout
  title="Bulbasaur"               // string
  subtitle="#001 · Seed Pokémon"  // string | null
>
  <svelte:fragment slot="badges">
    <TypeBadge type={grassType} />
    <TypeBadge type={poisonType} />
  </svelte:fragment>
  <svelte:fragment slot="actions">
    <SourceLabel source="pokeapi" />
  </svelte:fragment>
  
  <!-- conteúdo específico da entidade -->
  <PokemonDetailTabs ... />
</WikiDetailLayout>
```

- Container max-width 960px centrado
- Header: título (Press Start 2P 24px) + subtítulo (Inter 14px)
- Badges: alinhados à direita do header
- Actions: canto superior direito
- Conteúdo: abaixo do header, com padding `--space-4`
- Usado por: Pokemon, Move, Ability, Item, Type, Location

#### WikiListLayout

```svelte
<WikiListLayout
  title="Pokémon"                  // string
  count={151}                      // number | null
  description="All Pokémon in the database"
>
  <svelte:fragment slot="filters">
    <!-- FilterGroups opcionais -->
  </svelte:fragment>
  
  <!-- grid de cards -->
  {#each pokemonList as pokemon}
    <PokemonCard pokemon={pokemon} variant="full" />
  {/each}
  
  <Pagination {page} {totalPages} on:change />
</WikiListLayout>
```

- Header: título (Press Start 2P 20px) + contagem (Inter 14px, `--text-secondary`)
- Grid responsivo: 1 coluna (mobile), 2 (tablet), 3–4 (desktop)
- Gap: `--space-4` entre cards
- Pagination: abaixo do grid

#### GameboyFrame

Envolve toda a página. Não recebe props — é layout fixo.

- Borda exterior: 4px solid #292929
- Inner shadow: inset 0 0 20px rgba(0,0,0,0.5)
- Background: `--bg-primary`
- Header fixo no topo
- Sidebar fixo na esquerda
- Breadcrumbs abaixo do header
- Conteúdo centrado com max-width

---

## 5. Páginas — Specs Visuais + Comportamentais

### Home (`/`) — Busca Global

**Load:** Nenhum dado SSR necessário além da lista de nomes/slugs para Fuse.js.

**Comportamento:**
1. SearchBar no centro da página (estilo Google), grande, com placeholder "Search Pokémon, moves, abilities..."
2. Ao digitar: Fuse.js busca client-side nos nomes, slugs, e dex numbers
3. Resultados aparecem abaixo em lista de SearchResultItem (máx 10, priorizando Pokémon)
4. Ao pressionar Enter ou clicar em resultado: navega para `/wiki/{entityType}/{slug}`
5. Aba "Filters" abaixo da search bar colapsa para mostrar FilterGroup (Type, Category, etc.)
6. Versão mobile: search bar full-width, filtros em drawer

### Wiki Pokémon Detail (`/wiki/pokemon/[slug]`)

**Load (+page.server.ts):**
1. Species por slug
2. Pokemon (default form) + stats + types + abilities
3. Pokemon forms (todas as alternativas)
4. Evolution chain data
5. Flavor texts (language=en, todos os version_groups)
6. Pokemon moves (filtrado por version_group selecionado, default=firered-leafgreen)
7. Encounters (filtrado por version selecionado, se existirem)
8. Pokedex entries (qual dex inclui este Pokémon)

**Comportamento:**
1. WikiDetailLayout → header com nome, dex number, TypeBadges, SourceLabel
2. Sprite (com Shiny toggle) à esquerda, dados à direita
3. Tabs: About | Stats | Moves | Evolution | Encounters | Forms
4. Tab "About": FlavorTextList (com VersionSelector) + genus + habitat + color + capture rate
5. Tab "Stats": 6 StatBars + total
6. Tab "Moves": tabela filtrada por VersionSelect + FilterGroup (learn method)
7. Tab "Evolution": EvolutionChain (árvore com condições detalhadas)
8. Tab "Encounters": EncounterTable filtrado por VersionSelect + FilterGroup (method)
9. Tab "Forms": grid de PokemonCard variant="compact" das formas alternativas

### Wiki Type Detail (`/wiki/types/[slug]`)

**Load:** Type data + effectiveness grid data

**Comportamento:**
1. WikiDetailLayout → header com nome do tipo, TypeBadge large, move damage class
2. EffectivenessGrid (direction="attacking"): o que este tipo ataca super efetivo
3. EffectivenessGrid (direction="defending"): o que resiste e o que é fraco contra
4. Lista de Pokémon com este tipo (PokemonCard variant="compact")
5. Lista de moves com este tipo (MoveCard variant="compact")

### Wiki Move Detail (`/wiki/moves/[slug]`)

**Load:** Move data + flavor texts + effect

**Comportamento:**
1. WikiDetailLayout → header com nome, TypeBadge, category badge, power, accuracy, PP
2. FlavorTextList com VersionSelect
3. Effect description
4. Lista de Pokémon que aprendem este move (PokemonCard variant="compact"), filtrado por VersionSelect

### Wiki Ability Detail (`/wiki/abilities/[slug]`)

**Load:** Ability data + flavor texts + effect

**Comportamento:**
1. WikiDetailLayout → header com nome, generation badge
2. FlavorTextList com VersionSelect
3. Effect description (short + long)
4. Lista de Pokémon com esta ability (PokemonCard variant="compact"), com indicação de hidden ability

### Wiki Item Detail (`/wiki/items/[slug]`)

**Load:** Item data + flavor texts + effect

**Comportamento:**
1. WikiDetailLayout → header com nome, ícone, cost, category
2. Sprite do item (da CDN)
3. FlavorTextList com VersionSelect
4. Effect description
5. Lista de Pokémon que podem segurar este item (se disponível)

### Wiki Location Detail (`/wiki/locations/[slug]`)

**Load:** Location data + region info + location areas + encounters

**Comportamento:**
1. WikiDetailLayout → header com nome da localização + região
2. Mapa/nome da região (link para /wiki/games/{region})
3. Lista de sub-áreas (LocationArea) com collapse
4. Para cada sub-área: EncounterTable filtrado por VersionSelect
5. Filtro por encounter method (FilterGroup type="checkbox")
6. Lista de Pokémon encontráveis (link para wiki)

### Wiki Pokedex (`/wiki/pokedex/[slug]`)

**Load:** Pokedex data + entries + species data

**Comportamento:**
1. WikiDetailLayout → header com nome da Pokédex + região + contagem
2. Grid de PokedexEntry (numerado regionalmente)
3. Entry vazio se Pokémon não importado (nome em cinza)
4. FilterGroup: Type (checkbox), Fully Evolved (toggle), Sort (dropdown)

### Wiki Games (`/wiki/games/[slug]`)

**Load:** Version group data + region + generation info

**Comportamento:**
1. WikiDetailLayout → header com nome da version group + generation
2. Seção "Pokémon disponíveis": contagem + link para Pokédex
3. Seção "Novidades desta geração": abilities e moves introduzidos
4. Seção "Localizações": link para /wiki/locations?region={region}
5. Info box: region, generation, versões incluídas

### Search Page (`/search`)

**Load:** Search results from query params

**Comportamento:**
1. SearchBar no topo (mesma da home)
2. FilterSidebar à esquerda (desktop) / drawer (mobile)
3. Grid de cards no centro (dependendo do entity_type)
4. Pagination no fundo
5. Query params: `?q=chari&entity_type=pokemon&type_name=fire&sort=dex_asc`
6. Se nenhum resultado: mensagem "No results found" com sugestões

### Import Page (`/import`)

**Load:** ImportStatus (contagem de registros por entidade e source)

**Comportamento:**
1. ImportStatusDashboard no topo (overview)
2. Seção "Game Data" → botão "Import All Game Data"
3. Seção "Core Data" → botões individuais por tipo
4. Seção "Pokémon" → input + botões por geração
5. Seção "Locations & Encounters" → botão com aviso de volume
6. Seção "Re-import" → botão "Update All (pokeapi only)"
7. Cada botão: loading state inline, resultado após completar
8. Log de imports em tempo real na parte inferior

### Editor (`/editor/*`)

**Load:** Dados da entidade + source label

**Comportamento:**
1. WikiListLayout com botão "New" no canto
2. Cada card tem ícone de edição (lápis) no canto
3. EditorForm para edição: campos pré-preenchidos, validação Zod, SourceLabel
4. Ao salvar: source muda de `pokeapi` para `edited`
5. Na criação: source é `custom`

### Battle Pages (`/battle/new`, `/battle/join`, `/battle/[id]`)

**Create:** Formulário para criar sala (gera roomId de 6 caracteres)
**Join:** Input para roomId ou link direto
**Room:** BattleHeader +Pokemon selection (6 slots com TeamSlot) + action buttons + battle log

---

## 6. Padrões de Interação

### Busca Fuzzy (Fuse.js)

1. No load da home, o servidor envia um payload leve com `[{ name, slug, dexNumber, entityType }]` para todas as entidades importadas
2. Fuse.js é inicializado client-side com esse payload
3. Ao digitar: debounce de 150ms, busca fuzzy com threshold 0.3
4. Resultados aparecem abaixo da search bar em tempo real (sem round-trip ao servidor)
5. Busca por dex number funciona: `25` ou `#25` → Pikachu
6. Se o usuário pressiona Enter: navega para `/search?q=...` com resultados completos do servidor

### VersionSelector

1. Dropdown de version groups agrupados por geração
2. Seleção persistida no localStorage (chave: `preferred-version`)
3. Default: `firered-leafgreen` (Gen 3, mais relevante para ROM hack)
4. Ao mudar: filtra flavor texts, learnsets, e encounters dinamicamente (dados já carregados no load, sem fetch adicional)
5. Se a versão selecionada não tem dados para a entidade: fallback para a versão mais recente disponível

### Import Flow

1. Usuário clica no botão (ex: "Import Gen 1 Pokémon")
2. Botão muda para spinner + "Importing..."
3. Request POST para `/api/import` com `{ action: 'import_generation', generation: 1 }`
4. Response streaming via SSE ou polling: mostra progresso (X/Y imported, Z skipped)
5. Ao completar: botão muda para verde ✓ + contagem
6. Se erro: botão muda para vermelho ✗ + mensagem de erro + botão "Retry"
7. ImportStatusDashboard atualiza em tempo real

### Editor Form Submission

1. Usuário edita um campo no formulário
2. Validação client-side via Zod em tempo real (on blur)
3. Ao salvar: POST para `/editor/{entityType}/{slug}`
4. Se source era `pokeapi`: muda para `edited`
5. Grava diff na tabela `edits` (previous_data vs new_data)
6. Toast de sucesso/erro

### Battle Flow

1. Criar sala: gera roomId, redireciona para `/battle/[id]`
2. Oponente entra via código ou link
3. Ambos selecionam time (6 TeamSlots)
4. Socket.io: turno-based, aguarda ambos os jogadores enviarem ação
5. Servidor valida via engine, processa turno, emite resultado
6. Ao final: resultado gravado no banco, redireciona para perfil

---

## 7. Responsividade

### Mobile (< 768px)

```
┌──────────────────────┐
│ ☰  KARTA YAXU  🔍 👤 │ ← Header compacto, hamburger menu
├──────────────────────┤
│ Breadcrumbs          │
├──────────────────────┤
│                      │
│  <conteúdo full-width│
│  cards empilham 1×1  │
│  tabs scroll         │
│  horizontal          │
│                      │
└──────────────────────┘
```

- Sidebar: drawer deslizante, overlay escuro
- Cards: empilham 1 coluna full-width
- Tabs: scroll horizontal com setas
- FilterSidebar: botão "Filters" abre drawer
- Search bar: full-width
- TypeBadge: size="sm"
- VersionSelect: dropdown full-width

### Tablet (768–1024px)

```
┌─────┬─────────────────────────────────┐
│  🎮 │  KARTA YAXU           🔍 Login  │
│ ███ │─────────────────────────────────│
│ ███ │  Breadcrumbs                  │
│ ███ │─────────────────────────────────│
│ ███ │                                │
│ ███ │  <conteúdo max-width 700px    │
│ ███ │   cards 2 colunas>             │
│ ███ │                                │
└─────┴─────────────────────────────────┘
```

- Sidebar: 56px, apenas ícones, expande no hover
- Cards: 2 colunas
- Tabs: normais
- VersionSelect: dropdown normal

### Desktop (> 1024px)

```
┌──────┬─────────────────────────────────────────────────┐
│      │  KARTA YAXU                          🔍 Login   │
│ WIKI │─────────────────────────────────────────────────│
│      │  Raiz > Wiki > Pokémon > Bulbasaur             │
│ Types│─────────────────────────────────────────────────│
│ Moves│                                                 │
│ Abils│  <conteúdo max-width 960px                     │
│ Items│   cards 3–4 colunas>                          │
│ Games│                                                 │
│ Locs │                                                 │
│ Dex  │                                                 │
│ Mech │                                                 │
│ ──── │                                                 │
│ Ver  │                                                 │
│[FR/] │                                                 │
└──────┴─────────────────────────────────────────────────┘
```

- Sidebar: 200px expandida, com texto
- Cards: 3–4 colunas
- Tudo visível sem scroll horizontal