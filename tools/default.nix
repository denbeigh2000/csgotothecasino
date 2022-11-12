{ naersk }:

final: prev:
let
  rust-version = "1.65.0";
  rust-toolchain = prev.rust-bin.stable.latest.default;

  naersk' = prev.callPackage naersk {
    cargo = rust-toolchain;
    rustc = rust-toolchain;
  };

  casino = prev.callPackage ../casino { inherit (naersk') buildPackage; };
  viz = prev.callPackage ../viz { };
in
{
  inherit casino viz;
}
