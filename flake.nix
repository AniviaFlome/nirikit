{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      treefmt-nix,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      treefmtEval = system: treefmt-nix.lib.evalModule nixpkgs.legacyPackages.${system} ./nix/treefmt.nix;
    in
    {
      formatter = forAllSystems (system: (treefmtEval system).config.build.wrapper);

      checks = forAllSystems (system: {
        formatting = (treefmtEval system).config.build.check self;
      });

      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = import ./nix/package.nix { inherit pkgs; };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/nirikit";
        };
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              gcc
              rustc
              rustfmt
            ];
          };
        }
      );

      homeModules.default = ./nix/module.nix;
    };
}
