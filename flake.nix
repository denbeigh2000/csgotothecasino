{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils/master";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, naersk, devshell }:
    rec {
      overlays.default = import ./overlay.nix { inherit naersk; };
      nixosModules = {
        casino = import ./casino/module.nix {
          overlay = overlays.default;
        };
        viz = import ./viz/module.nix {
          overlay = overlays.default;
        };
      };
    } // flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            naersk.overlay
            fenix.overlays.default
            devshell.overlay
            self.overlays.default
          ];
        };
      in
      {
        devShells.default = pkgs.callPackage ./devshell.nix {};

        packages = rec {
          inherit (pkgs.casino) aggregator bootstrap collector bootstrap-windows collector-windows;
          inherit (pkgs) viz;

          default = pkgs.symlinkJoin {
            name = "casino";
            paths = [ aggregator bootstrap collector ];
          };
        };
      });
}
