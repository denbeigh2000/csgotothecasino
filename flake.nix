{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils/master";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
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

  outputs = { self, nixpkgs, flake-utils, rust-overlay, naersk, devshell }:
    rec {
      overlays.default = import ./overlay.nix { inherit naersk; };
      nixosModules = {
        casino = import ./casino/module.nix {
          inherit rust-overlay;
          overlay = overlays.default;
        };
        viz = import ./viz/module.nix {
          inherit rust-overlay;
          overlay = overlays.default;
        };
      };
    } // flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
            naersk.overlay
            devshell.overlay
            self.overlays.default
          ];
        };
      in
      {
        devShells.default = pkgs.callPackage ./devshell.nix {};

        packages = rec {
          inherit (pkgs.casino) aggregator bootstrap collector;
          inherit (pkgs) viz;

          default = pkgs.symlinkJoin {
            name = "casino";
            paths = [ aggregator bootstrap collector ];
          };
        };
      });
}
