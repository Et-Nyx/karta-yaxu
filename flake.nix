{
  description = "Karta Yaxu local development";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }: {
    devShells = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        developmentPackages = with pkgs; [ rustc cargo rustfmt clippy sqlite curl ];
        browserPython = pkgs.python3.withPackages (p: [ p.playwright ]);
        browsers = pkgs.playwright-driver.selectBrowsers {
          withFirefox = false;
          withWebkit = false;
          withChromiumHeadlessShell = false;
          withFfmpeg = false;
        };
      in {
        default = pkgs.mkShell {
          packages = developmentPackages;
        };
        browser-tests = pkgs.mkShell {
          packages = developmentPackages ++ [ browserPython browsers ];
          PLAYWRIGHT_BROWSERS_PATH = "${browsers}";
          # Nix patches browser libraries/RPATHs; distro package probes do not apply.
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
          FONTCONFIG_FILE = pkgs.makeFontsConf {
            fontDirectories = [ pkgs.dejavu_fonts ];
          };
          PYTHONDONTWRITEBYTECODE = "1";
        };
      });
  };
}
