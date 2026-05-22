{
  description = "Karta Yaxu 2.0 — Pokémon ROM Hack Simulator & Wiki";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.nodejs_22
            pkgs.sqlite
          ];

          shellHook = ''
            echo ""
            echo "🎮 Karta Yaxu 2.0 — Dev Environment"
            echo "   Node.js: $(node --version)"
            echo "   SQLite:  $(sqlite3 --version | cut -d' ' -f1)"
            echo "   npm:     $(npm --version)"
            echo ""
            echo "Commands:"
            echo "   npm run dev        — Start dev server"
            echo "   npm run db:push    — Push schema to SQLite"
            echo "   npm run db:studio  — Open Drizzle Studio"
            echo ""
          '';
        };
      }
    );
}
