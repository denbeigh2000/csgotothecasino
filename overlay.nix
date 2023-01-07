{ naersk }:

final: prev:
let

  inherit (builtins) attrNames trace;
  inherit (prev) fenix callPackage nodejs-18_x yarn;
  inherit (prev.stdenvNoCC.platform) system;

  rust-toolchain =
    let
      inherit (fenix) combine targets;
      inherit (fenix.stable) cargo rustc;
    in
    combine [
      cargo
      rustc
      targets.x86_64-pc-windows-gnu.stable.rust-std
    ];

  naersk' = callPackage naersk {
    cargo = rust-toolchain;
    rustc = rust-toolchain;
  };

  node-tools = [ nodejs-18_x yarn ];

  casino = callPackage ./casino { inherit (naersk') buildPackage; };
  viz = callPackage ./viz { };
in
{
  inherit casino viz node-tools rust-toolchain;
}
