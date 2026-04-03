# Plano do Projeto — Pokemon ROM Hack Simulator & Wiki

## Visão Geral

Uma plataforma web com três pilares integrados para suportar o desenvolvimento de um ROM hack de Pokémon Gen 3 (RSE/FRLG):

- **Wiki** — documentação pública dos recursos da ROM (Pokémon, moves, abilities, itens, mecânicas)
- **Editor** — interface para o time de desenvolvimento editar os dados que alimentam a wiki e o simulador
- **Simulador** — batalhas online em tempo real com as regras customizadas da ROM

Todos os pilares compartilham o mesmo **banco de dados**. Quando o time atualiza um movimento no editor, a wiki e o simulador refletem a mudança automaticamente.

Os arquivos `.toml` no repositório servem como _seeds_ para popular o banco de dados e como formato de exportação.

---

## Stack

| Camada                 | Tecnologia                        |
| ---------------------- | --------------------------------- |
| UI                     | HTML + CSS vanilla                |
| Lógica no navegador    | Rust → WebAssembly (wasm-bindgen) |
| Dados / edição         | TOML                              |
| Templates HTML         | askama (Rust, compile-time)       |
| Backend                | Rust + Axum + Tokio               |
| Batalhas em tempo real | WebSockets nativos (Axum)         |
| Banco de dados         | SQLite via sqlx                   |
| Deploy                 | Fly.io (Docker)                   |

Sem frameworks de frontend. Sem bibliotecas de terceiros para WebSocket ou auth — só o ecossistema Rust e protocolos nativos.

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
│   ├── engine/       # battle engine, damage calc — Rust puro
│   ├── frontend/     # compila para WASM, manipula DOM via wasm-bindgen
│   └── backend/      # Axum + WebSockets + sqlx + askama templates
├── static/
│   ├── index.html
│   └── style.css
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

Exemplo do template de um Pokémon (`data/template/pokemon.toml`):

```toml
[base]
name = ""
types = []
hp = 0
atk = 0
def = 0
spa = 0
spd = 0
spe = 0
pokedex_entry = ""

[learnset]
level_up = []
```

---

## Schema do Banco de Dados

### Type

Duas tabelas: uma para os dados do tipo, outra para a matriz de efetividade.

```
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

```
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

```
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
method       TEXT     NOT NULL  -- 'level_up' | 'tm' | 'egg' | 'tutor'
level        INTEGER             -- NULL quando method ≠ 'level_up'
PRIMARY KEY (pokemon_id, move_id, method)
```

### Pokémon

```
pokemon
─────────────────────────────────────────
id             INTEGER  PRIMARY KEY
dex_number     INTEGER  NOT NULL        -- pode repetir em formas alternativas
name           TEXT     NOT NULL UNIQUE
sprite         TEXT     NOT NULL        -- nome do arquivo em static/sprites/
sprite_shiny   TEXT     NOT NULL        -- nome do arquivo em static/sprites/shiny/
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

### Trainer

```
trainers
─────────────────────────────────────────
id             TEXT     PRIMARY KEY  -- uuid
username       TEXT     NOT NULL UNIQUE
password_hash  TEXT     NOT NULL     -- argon2
role           TEXT     NOT NULL DEFAULT 'viewer'  -- 'viewer' | 'editor'
is_npc         INTEGER  NOT NULL DEFAULT 0  -- 1 para personagens canônicos
created_at     TEXT     NOT NULL DEFAULT (datetime('now'))

trainers (já definido)

teams
─────────────────────────────────────────
id          INTEGER  PRIMARY KEY
trainer_id  TEXT     NOT NULL  → trainers(id)
name        TEXT     NOT NULL  -- nickname do time
created_at  TEXT     NOT NULL DEFAULT (datetime('now'))

team_members
─────────────────────────────────────────
id          INTEGER  PRIMARY KEY
team_id     INTEGER  NOT NULL  → teams(id)
slot        INTEGER  NOT NULL  -- 1 a 6
pokemon_id  INTEGER  NOT NULL  → pokemon(id)
ability_id  INTEGER  NOT NULL  → abilities(id)
nature      TEXT     NOT NULL  -- 'Hardy' | 'Lonely' | 'Brave' | ...
held_item   TEXT               -- referência a items (tabela a definir)
move1_id    INTEGER            → moves(id)
move2_id    INTEGER            → moves(id)
move3_id    INTEGER            → moves(id)
move4_id    INTEGER            → moves(id)

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

1. Modelar o esquema do banco de dados SQLite e definir as structs Rust
2. Implementar a crate `engine` — ler estruturas do banco, damage formula Gen 3
3. Backend Axum — rotas, templates `askama`, servir estáticos, ler dados do banco
4. Editor TOML — serializar registros do banco para TOML na sessão, edição cru no browser, deserializar no POST
5. Frontend WASM — partes interativas (validação TOML em tempo real, batalha)
6. Sistema de Trainers — autenticação, roles e registro de edições
7. WebSockets — sala de batalha, conectar dois jogadores
8. Histórico — salvar replays, edits e times dos Trainers
9. Deploy no Fly.io
