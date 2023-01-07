{ naersk }:

final: prev:
let

  inherit (builtins) attrNames trace;
  inherit (prev) fenix callPackage nodejs-18_x yarn;
  inherit (prev.stdenvNoCC.platform) system;

  mkToolchain = { dev ? false }:
    let
      inherit (fenix) combine targets;
      inherit (fenix.stable) cargo clippy rust-analyzer rust-src rustc;

      devPackages = if dev then [ clippy rust-analyzer rust-src ] else [];
    in
    combine ([
      cargo
      rustc
      targets.x86_64-pc-windows-gnu.stable.rust-std
    ] ++ devPackages);

  rust-toolchain = mkToolchain {};
  rust-toolchain-dev = mkToolchain { dev = true; };

  naersk' = callPackage naersk {
    cargo = rust-toolchain;
    rustc = rust-toolchain;
  };

  node-tools = [ nodejs-18_x yarn ];

  casino = callPackage ./casino { inherit (naersk') buildPackage; };
  viz = callPackage ./viz { };
in
{
  inherit casino viz node-tools rust-toolchain rust-toolchain-dev;
}
