{
  description = "Pokemon ROM hack simulator + wiki";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
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
            export SEED_DATA_PATH="$PWD/data/template"
            export RUST_LOG="debug"
            echo "🦀 Ambiente Pokemon ROM pronto"
          '';
        };
      }
    );
}
