{ naersk }:

final: prev:
let
  rust-version = "1.65.0";
  rust-toolchain = prev.rust-bin.stable."${rust-version}".default;

  naersk' = prev.callPackage naersk {
    cargo = rust-toolchain;
    rustc = rust-toolchain;
  };

  node-tools = with prev; [ nodejs-18_x yarn ];

  casino = prev.callPackage ./casino { inherit (naersk') buildPackage; };
  viz = prev.callPackage ./viz { };
in
{
  inherit casino viz node-tools rust-toolchain;
}
