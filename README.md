# Plano do Projeto — Pokemon ROM Hack Simulator & Wiki

## Visão Geral

Uma plataforma web com três pilares integrados para suportar o desenvolvimento de um ROM hack de Pokémon Gen 3 (RSE/FRLG):

- **Wiki** — documentação pública dos recursos da ROM (Pokémon, moves, abilities, itens, mecânicas)
- **Editor** — interface para o time de desenvolvimento editar os dados que alimentam a wiki e o simulador
- **Simulador** — batalhas online em tempo real com as regras customizadas da ROM

Todos os pilares compartilham o mesmo **banco de dados**. Quando o time atualiza um movimento no editor, a wiki e o simulador refletem a mudança automaticamente.

Os arquivos `.toml` do repositório (como os templates) servem como a linguagem base do sistema de edição no navegador. O Backend converte registros do SQLite para TOML na tela do Editor, e reverte as edições de volta para o Banco.

---

## Stack

| Camada                 | Tecnologia                        |
| ---------------------- | --------------------------------- |
| UI                     | HTML + CSS vanilla                |
| Lógica no navegador    | Rust → WebAssembly (wasm-bindgen) |
| Dados / edição         | TOML                              |
| Templates HTML         | askama (Rust, compile-time)       |
| Backend                | Rust + Axum + Tokio               |
| Busca fuzzy            | strsim (Levenshtein distance)     |
| Batalhas em tempo real | WebSockets nativos (Axum)         |
| Banco de dados         | SQLite via sqlx                   |
| Deploy                 | Fly.io (Docker)                  |

Sem frameworks de frontend. Sem bibliotecas de terceiros para WebSocket ou auth — só o ecossistema Rust e protocolos nativos. Busca fuzzy via crate `strsim` (Levenshtein distance).

---

## Arquitetura — Shared Engine

A principal vantagem da stack é que o mesmo código Rust pode compilar para dois alvos:

```
┌─────────────────────────────────┐
│      crate: engine              │
│  damage calc, battle loop,      │
│  gen 3 mechanics, TOML parser   │
└──────────┬──────────────────────┘
           │ compilado para...
     ┌─────┴──────┐
     ▼            ▼
  WASM          Binário
(navegador)    (servidor)
```

O `engine` é uma crate Rust pura, sem dependências de plataforma. Compila para WASM para uso offline no cliente, e é linkada no backend para validar batalhas online. Zero duplicação de lógica.

---

## Estrutura do Projeto

```
karta-yaxu/
├── flake.nix
├── rust-toolchain.toml
├── crates/
│   ├── engine/       # models.rs (Base de Dados) e battle.rs (Lógica de Simulação)
│   ├── frontend/     # compila para WASM, manipula DOM via wasm-bindgen
│   │   └── src/
│   │       └── search.rs  # toggle de facetas mobile, filtro client-side
│   └── backend/      # Axum + WebSockets + sqlx + askama templates
│       └── src/
│           └── search.rs  # busca fuzzy, filtros por tipo/categoria, scoring
├── static/
│   ├── index.html
│   ├── style.css
│   ├── sprites/
│   │   ├── pokemon/   # ex: bulbasaur.png
│   │   └── shiny/     # ex: bulbasaur_shiny.png
│   └── icons/
│       ├── types/     # svg dos 18 tipos base
│       ├── items/     # ex: miracle_seed.png
│       └── status/    # ex: brn.png
└── data/
    └── template/     # templates TOML para registros novos
        ├── pokemon.toml
        ├── move.toml
        └── ability.toml
```

### Formato TOML dos dados

Todo o estado dinâmico do projeto (Pokémon, movimentos, usuários, edições...) vive no **SQLite**. O TOML é a interface de leitura e escrita desse banco — não há arquivos TOML persistentes por registro.

Os TOMLs têm dois papéis:

**1. Templates (`data/template/`)** — definem a estrutura de cada tipo de dado. Quando um editor cria um registro novo, o backend usa o template como ponto de partida.

**2. Interface de edição (sessão)** — ao acessar `/editor/*`, o backend serializa o registro do banco para uma string TOML armazenada na sessão do usuário. O editor exibe o TOML cru num `textarea`. Ao salvar, o backend valida, deserializa e grava no banco.

```
DB → serializa → TOML na sessão → textarea no browser
 ↑                                          ↓
 └──── deserializa ←── POST /editor/* ───────┘
```

Exemplo do formato atual de um Pokémon (`data/template/pokemon.toml`):

```toml
[[pokemon]]
id = 1
dex_number = 1
name = "Bulbasaur"
sprite = "bulbasaur.png"
sprite_shiny = "bulbasaur_shiny.png"
type1_id = 12
type2_id = 4
hp = 45
atk = 49
def = 49
spa = 65
spd = 65
spe = 45
pokedex_entry = "A strange seed was planted on its back at birth."
```

*(Nota: As tabelas relacionais em Array, como `[learnset]` e evoluções, ganharam arquivos `.toml` independentes para preservar 100% da correspondência bidimensional com o banco de dados).*

### Seeding do Banco de Dados

O banco é populado pelo binário CLI `seed`, que pode ser invocado a qualquer momento. Ele cria o arquivo `.db` automaticamente (se não existir), aplica as migrations e insere todos os dados dos templates TOML em transação única.

```bash
# Popular (ou recriar do zero) o banco:
cargo run -p backend --bin seed

# Se precisar recriar completamente:
rm data/rom.db && cargo run -p backend --bin seed
```

Os arquivos `.toml` em `data/template/` são a fonte de verdade dos dados estáticos. Ao editar um template e rodar o seed novamente, o banco é atualizado via `INSERT OR REPLACE` (não-destrutivo para dados não presentes nos templates).

> **Nota:** O projeto usa SQLx com modo offline (`SQLX_OFFLINE=true` via `.cargo/config.toml`). Após alterar queries ou migrations, regenere o cache: `DATABASE_URL=sqlite:./data/rom.db cargo sqlx prepare --workspace`.

---

## Schema do Banco de Dados

### Type

Duas tabelas: uma para os dados do tipo, outra para a matriz de efetividade.

```sql
types
─────────────────────────────────────────
id        INTEGER  PRIMARY KEY
name      TEXT     NOT NULL UNIQUE
hexcolor  TEXT     NOT NULL        -- ex: "#F08030"
icon      TEXT     NOT NULL        -- nome do arquivo em static/icons/

type_effectiveness
─────────────────────────────────────────
attacker_id  INTEGER  NOT NULL  → types(id)
defender_id  INTEGER  NOT NULL  → types(id)
multiplier   REAL     NOT NULL  -- 0 | 0.5 | 1 | 2
PRIMARY KEY (attacker_id, defender_id)
```

### Ability

```sql
abilities
─────────────────────────────────────────
id           INTEGER  PRIMARY KEY
name         TEXT     NOT NULL UNIQUE
description  TEXT     NOT NULL DEFAULT ''

pokemon_abilities
─────────────────────────────────────────
pokemon_id   INTEGER  NOT NULL  → pokemon(id)
ability_id   INTEGER  NOT NULL  → abilities(id)
slot         INTEGER  NOT NULL  -- 1, 2, 3...
PRIMARY KEY (pokemon_id, slot)
```

### Move

```sql
moves
─────────────────────────────────────────
id           INTEGER  PRIMARY KEY
name         TEXT     NOT NULL UNIQUE
type_id      INTEGER  NOT NULL  → types(id)
category     TEXT     NOT NULL  -- 'physical' | 'special' | 'status'
power        INTEGER             -- NULL para moves de status
accuracy     INTEGER             -- NULL para moves que sempre acertam
pp           INTEGER  NOT NULL
description  TEXT     NOT NULL DEFAULT ''

learnset
─────────────────────────────────────────
pokemon_id   INTEGER  NOT NULL  → pokemon(id)
move_id      INTEGER  NOT NULL  → moves(id)
method       TEXT     NOT NULL  -- 'level_up' | 'tm' | 'egg' | 'tutor' | 'gmax' | 'max'
level        INTEGER             -- NULL quando method ≠ 'level_up'
PRIMARY KEY (pokemon_id, move_id, method)
```

### Pokémon

```sql
pokemon
─────────────────────────────────────────
id             INTEGER  PRIMARY KEY
dex_number     INTEGER  NOT NULL        -- pode repetir em formas alternativas
name           TEXT     NOT NULL UNIQUE
sprite         TEXT     NOT NULL        -- renderizado em static/sprites/pokemon/
sprite_shiny   TEXT     NOT NULL        -- renderizado em static/sprites/shiny/
type1_id       INTEGER  NOT NULL  → types(id)
type2_id       INTEGER             → types(id)   -- NULL se monotype
hp             INTEGER  NOT NULL
atk            INTEGER  NOT NULL
def            INTEGER  NOT NULL
spa            INTEGER  NOT NULL
spd            INTEGER  NOT NULL
spe            INTEGER  NOT NULL
pokedex_entry  TEXT     NOT NULL DEFAULT ''

evolutions
─────────────────────────────────────────
from_id          INTEGER  NOT NULL  → pokemon(id)
to_id            INTEGER  NOT NULL  → pokemon(id)
condition        TEXT     NOT NULL  -- 'level' | 'item' | 'trade' | 'happiness' | ...
condition_value  TEXT               -- ex: "36" para level, "Fire Stone" para item
PRIMARY KEY (from_id, to_id)

pokemon_forms  (formas alternativas e Mega Evoluções)
─────────────────────────────────────────
base_id    INTEGER  NOT NULL  → pokemon(id)  -- ex: Venusaur
form_id    INTEGER  NOT NULL  → pokemon(id)  -- ex: Mega Venusaur
form_name  TEXT     NOT NULL                 -- 'Mega' | 'Alolan' | 'Attack Form' | ...
PRIMARY KEY (base_id, form_id)
```

Cada forma alternativa e Mega Evolução é um registro independente em `pokemon` (com seus próprios stats, tipos e sprites), ligado à forma base via `pokemon_forms`. Mega Evoluções ficam fora da tabela `evolutions`, mantendo a query de evoluções finais limpa.

*Convenção de Continuidade (ID):* O projeto mapeia variações usando um sistema de saltos baseado no ID principal original do Pokémon de forma previsível:
- **`+ 2000`** = Mega Evolução (Ex: Venusaur é *#3*, Mega Venusaur é *2003*)
- **`+ 3000`** = Formas Especiais (Gigantamax, Dynamax, Formas Regionais)

Independente do pulo do `.id`, o banco mantém a coluna separada `.dex_number` intocada entre todas as formas para agrupar visualmente na Wiki Oficial sem furar a continuidade da Pokedex.

*Herança de Propriedades entre Formas:* Formas alternativas que não possuem entradas próprias no banco herdam automaticamente as propriedades da forma base:
- Sem evoluções próprias → herda a linha evolutiva da base (BFS a partir da raiz)
- Sem learnset próprio → herda o moveset completo da base
- Formas com entradas próprias (ex: Gigantamax com G-Max Moves) sobrescrevem o fallback

### Items

```sql
items
─────────────────────────────────────────
id           INTEGER  PRIMARY KEY
name         TEXT     NOT NULL UNIQUE
description  TEXT     NOT NULL DEFAULT ''
icon         TEXT     NOT NULL             -- nome do arquivo em static/icons/items/
```

### Status Conditions

```sql
status_conditions
─────────────────────────────────────────
id           INTEGER  PRIMARY KEY
name         TEXT     NOT NULL UNIQUE
description  TEXT     NOT NULL DEFAULT ''
icon         TEXT     NOT NULL             -- nome da imagem do ícone para o front
is_volatile  INTEGER  NOT NULL DEFAULT 0   -- 0 = Status Principal (Burn), 1 = Volátil (Sleep)
```

### Nature

```sql
natures
─────────────────────────────────────────
id              INTEGER  PRIMARY KEY
name            TEXT     NOT NULL UNIQUE   -- 'Adamant', 'Timid', 'Hardy', etc
increased_stat  TEXT                       -- 'atk', 'def', 'spa', 'spd', 'spe' (NULL se neutra)
decreased_stat  TEXT                       -- (NULL se neutra)
```

### Trainer

```sql
trainers
─────────────────────────────────────────
id             TEXT     PRIMARY KEY  -- uuid
username       TEXT     NOT NULL UNIQUE
password_hash  TEXT     NOT NULL     -- argon2
role           TEXT     NOT NULL DEFAULT 'viewer'  -- 'viewer' | 'editor'
is_npc         INTEGER  NOT NULL DEFAULT 0  -- 1 para personagens canônicos
created_at     TEXT     NOT NULL DEFAULT (datetime('now'))

teams
─────────────────────────────────────────
id          INTEGER  PRIMARY KEY
trainer_id  TEXT     NOT NULL  → trainers(id)
name        TEXT     NOT NULL  -- nickname do time
created_at  TEXT     NOT NULL DEFAULT (datetime('now'))

team_members
─────────────────────────────────────────
id            INTEGER  PRIMARY KEY
team_id       INTEGER  NOT NULL  → teams(id)
slot          INTEGER  NOT NULL  -- 1 a 6
pokemon_id    INTEGER  NOT NULL  → pokemon(id)
ability_id    INTEGER  NOT NULL  → abilities(id)
nature_id     INTEGER  NOT NULL  → natures(id)
held_item_id  INTEGER            → items(id)
move1_id      INTEGER            → moves(id)
move2_id      INTEGER            → moves(id)
move3_id      INTEGER            → moves(id)
move4_id      INTEGER            → moves(id)

battles
─────────────────────────────────────────
id            INTEGER  PRIMARY KEY
trainer_id_1  TEXT     NOT NULL  → trainers(id)
trainer_id_2  TEXT     NOT NULL  → trainers(id)  -- nunca NULL
team_id_1     INTEGER  NOT NULL  → teams(id)
team_id_2     INTEGER  NOT NULL  → teams(id)     -- nunca NULL
winner_id     TEXT               → trainers(id)  -- NULL se empate
log           TEXT     NOT NULL DEFAULT ''
played_at     TEXT     NOT NULL DEFAULT (datetime('now'))


edits
─────────────────────────────────────────
id          INTEGER  PRIMARY KEY
trainer_id  TEXT     NOT NULL  → trainers(id)
table_name  TEXT     NOT NULL  -- 'pokemon' | 'moves' | 'abilities' | ...
record_id   INTEGER  NOT NULL  -- id do registro editado
diff        TEXT     NOT NULL  -- diff do TOML antes/depois
edited_at   TEXT     NOT NULL DEFAULT (datetime('now'))


```

---

## Rotas

A navegação reflete uma hierarquia de sistema de arquivos. Qualquer usuário consegue adivinhar a URL de uma página só de saber onde ela está na hierarquia.

```
/
├── search              ← busca global (home page)
│
├── wiki/
│   ├── pokemon/
│   │   ├── bulbasaur
│   │   ├── charmander
│   │   └── ...
│   ├── moves/
│   ├── abilities/
│   ├── items/
│   ├── types/
│   └── mechanics/
│       ├── damage-formula
│       ├── status-conditions
│       └── ...
│
├── search/                ← busca fuzzy com filtros facetados
│   └── ?q=...&entity_type=...&type_name=...
│
├── api/
│   └── search/
│       └── suggestions    ← autocomplete JSON
│
├── editor/
│   ├── pokemon/
│   │   ├── bulbasaur     ← edita o mesmo recurso da wiki
│   │   └── new
│   ├── moves/
│   ├── abilities/
│   ├── items/
│   └── types/
│
├── battle/
│   ├── new              ← criar sala
│   ├── join             ← entrar com código
│   └── :id              ← sala em andamento
│
└── trainer/
    ├── login
    ├── register
    └── :username        ← perfil público
        ├── teams
        ├── battles
        └── edits
```

### Simetria entre wiki e editor

`/wiki/pokemon/bulbasaur` e `/editor/pokemon/bulbasaur` apontam para o mesmo recurso — só muda o que o usuário pode fazer com ele:

```
GET  /wiki/pokemon/bulbasaur    → página de leitura
GET  /editor/pokemon/bulbasaur  → formulário de edição
POST /editor/pokemon/bulbasaur  → salva as alterações
```

### Navegação

- **Sidebar** fixa com os diretórios principais
- **Breadcrumbs** no topo mostrando a posição atual, cada parte clicável:
  ```
  Raiz > Wiki > Pokémon > Bulbasaur
  ```

---

## Sistema de Autenticação — Trainers

Usuários do sistema se chamam **Trainers**. O cadastro é aberto, mas o acesso ao editor é restrito por role.

### Dados de um Trainer

```
trainer
├── id            (uuid)
├── username
├── password_hash (argon2)
├── role          (viewer | editor)
├── created_at
├── teams[]       (times salvos com nickname)
├── battles[]     (histórico de batalhas)
└── edits[]       (histórico de edições na wiki)
```

### Roles

| Role     | Acesso                    |
| -------- | ------------------------- |
| `viewer` | Wiki (pública) + batalhas |
| `editor` | Tudo acima + `/editor/*`  |

Qualquer pessoa pode se cadastrar como `viewer`. O role `editor` é promovido manualmente por um membro do time no banco de dados.

### Controle de acesso por rota

| Rota                                  | Acesso                         |
| ------------------------------------- | ------------------------------ |
| `/` (busca)                           | Público — sem login necessário |
| `/search`                              | Público — sem login necessário |
| `/api/search/suggestions`             | Público                        |
| `/wiki/*`                             | Público — sem login necessário |
| `/trainer/register`, `/trainer/login` | Público                        |
| `/trainer/:username`                  | Público                        |
| `/battle/*`                           | Requer login (qualquer role)   |
| `/editor/*`                           | Requer role `editor`           |

### Segurança

- Senhas armazenadas com hash **argon2** (crate `argon2`)
- Sessões via cookie gerenciado pela crate `tower-sessions`
- Nenhuma senha em texto puro no banco

---

## Ambiente de Desenvolvimento — NixOS + Flakes

O flake garante que todo o time tenha exatamente o mesmo ambiente.

```nix
# flake.nix
{
  description = "Pokemon ROM hack simulator + wiki";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays     = [ (import rust-overlay) ];
        pkgs         = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            wasm-bindgen-cli
            wasm-pack
            cargo-watch
            sqlx-cli
            flyctl
            mdbook
            pkg-config
            openssl
          ];

          shellHook = ''
            export DATABASE_URL="sqlite:./data/rom.db"
            export RUST_LOG="debug"
            echo "🦀 Ambiente Pokemon ROM pronto"
          '';
        };
      }
    );
}
```

```toml
# rust-toolchain.toml
[toolchain]
channel    = "stable"
targets    = ["wasm32-unknown-unknown"]
components = ["rustfmt", "clippy", "rust-analyzer"]
```

O `rust-toolchain.toml` é reconhecido pelo `rustup` também — membros do time que não usem Nix ainda pegam a versão correta automaticamente.

### Uso diário

```bash
# entrar no ambiente manualmente
nix develop

# ou com direnv (ativa automaticamente ao entrar na pasta)
echo "use flake" > .envrc
direnv allow
```

---

## Deploy — Fly.io

- Projeto empacotado via **Dockerfile** que compila o backend Rust
- `fly deploy` sobe o container em ~30 segundos
- WebSockets persistentes suportados nativamente (sem configuração extra)
- Volume persistente para o arquivo SQLite
- HTTPS e roteamento gerenciados pelo Fly automaticamente

---

## Ordem de Desenvolvimento

### ✅ Bloco 1 — Fundação Operacional (concluído)

- Seed movido para binário CLI (`cargo run -p backend --bin seed`) com `create_if_missing` e migrations automáticas
- Tratamento de erros no seed com mensagens descritivas por arquivo/linha
- `sqlx::migrate!()` chamado automaticamente no `main.rs`
- Cache SQLx offline gerado (`.sqlx/`) — builds não precisam de banco ativo em compile time
- Variável `DATABASE_URL` definida via `flake.nix` e `.env`

### ✅ Bloco 2 — Wiki (leitura pública) (concluído)

- Templates Askama SSR: `base.html`, pokemon/list, pokemon/detail, moves/list, moves/detail, types/list, types/detail, abilities/list, abilities/detail, mechanics/nature, mechanics/status
- Rotas wiki completas: `/wiki/pokemon`, `/wiki/pokemon/{slug}`, `/wiki/types/{slug}`, `/wiki/moves/{slug}`, `/wiki/abilities/{slug}`, `/wiki/mechanics/nature`, `/wiki/mechanics/status`
- Slugificação de URLs (lowercase, sem espaços/apóstrofos) via filtro `|slugify`
- Linha evolutiva calculada por BFS — inclui toda a genealogia da raiz ao estágio final
- Formas alternativas bidirecionais — base lista formas, formas listam a base
- Herança inteligente de learnset e linha evolutiva para formas sem entradas próprias
- Tipos como links navegáveis (badges clicáveis em todos os templates)
- Cache-busting (`?v=1`) em todos os assets de imagem
- `.prettierignore` protegendo templates Askama de formatadores automáticos
- Navegação aprimorada com Breadcrumbs dinâmicos Vanilla JS integrados a partir do `base.html`.
- Sistema de alternância local para sprites Shiny via toggle "Shiny Charm".
- Reestruturação da UI no layout "Gameboy Edition": cards flex-row fluídos (`.move-card`, `.ability-card`) e grid responsivo de Fraquezas vs Resistências.
- Centralização do tráfego interno através das Landing Pages raiz `/wiki` e `/editor` para navegação modular sem poluição na barra principal.
- Sistema de busca global com filtros facetados: `/` é a página de busca (home), `/search` com query params (`q`, `entity_type`, `type_name`, `type_name_2`, `category`, `stat`, `fully_evolved`, `only_base_form`, `sort`).
- Busca fuzzy com Levenshtein distance (crate `strsim`), incluindo busca por dex number (ex: `25` ou `#25` encontra Pikachu) e autocomplete via `/api/search/suggestions`.
- Sidebar de filtros com radio buttons para Páginas (Pokémon, Moves, Abilities, Types, Natures, Status) e sub-filtros dinâmicos por entidade.
- Filtros de tipo com checkboxes (máx. 2, lógica AND), filtros Fully Evolved e Base Form como toggles, dropdown customizado para ordenação.
- Frontend WASM para toggle de facetas mobile e filtro client-side por nome.
- Estilização Gameboy com font Press Start 2P, cards com bordas/sombras, grids coloridos por tipo e seletores `:has(input:checked)`.

### ✅ Bloco 3 — Autenticação e Editor (concluído)

> Objetivo: sistema de login, controle de painel de edição e blindagem de transações isoladas do banco.

- Autenticação e controle de acesso estritos implementados via `argon2` e `tower-sessions` (`viewer` e `editor`).
- Editor TOML funcional via rotas `/editor` (serialização DB → TOML e parser TOML → DB em endpoints seguros).
- Blindagem e rollback automático (`tx.rollback()`) em falhas transacionais complexas.
- Prevenção ativa de ID Hijacking baseada na validação do slug originário via DB `SELECT`.
- Habilitação estrita de restrições por chaves estrangeiras (`PRAGMA foreign_keys = ON`) para consistência dos relacionamentos em árvore.

### 🔲 Bloco 4 — Engine de Batalha

> Objetivo: implementar a lógica de combate Gen 3 no crate `engine`.

- Converter `weather` e `terrain` para enums em `battle.rs`
- Implementar a damage formula Gen 3 com lookup de `type_effectiveness` e stat stages
- Escrever testes unitários para o engine

### 🔲 Bloco 5 — Frontend WASM e WebSockets

> Objetivo: interatividade no cliente e batalha online.

- Implementar `crate frontend` com `wasm-bindgen` (validação TOML, preview de damage calc)
- Implementar WebSockets no backend (sala de batalha, matchmaking, loop de turno)

### 🔲 Bloco 6 — Deploy

- Criar Dockerfile multi-stage (build → runtime mínimo)
- Criar `fly.toml` com volume persistente para `rom.db`
- CI básico (GitHub Actions): `cargo test`, `cargo clippy`, `cargo fmt --check`
