{ buildPackage
, writeScriptBin
, pkg-config
, openssl
, stdenvNoCC
, pkgsCross
}:

let
  version = "0.0.1";
  common = {
    inherit version;

    src = ./.;
    root = ./.;
  };

  default = buildPackage (common // {
    name = "casino";

    nativeBuildInputs = [ pkg-config openssl ];
  });

  win = buildPackage (common // {
    name = "casino-windows";
    strictDeps = true;
    doCheck = false;

    CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

    depsBuildBuild = with pkgsCross.mingwW64; [
      stdenv.cc
      windows.pthreads
    ];

    nativeBuildInputs = with pkgsCross.mingwW64; [
      pkg-config
      openssl
    ];
  });

  mkBinary = { group, name, suffix ? "" }:
    stdenvNoCC.mkDerivation {
      inherit name;
      inherit version;

      phases = [ "installPhase" ];
      src = group;

      installPhase = ''
        mkdir -p $out/bin
        ln -s $src/bin/${name}${suffix} $out/bin/${name}${suffix}
      '';
    };
in
{
  aggregator = mkBinary { group = default; name = "aggregator"; };
  bootstrap = mkBinary { group = default; name = "bootstrap"; };
  collector = mkBinary { group = default; name = "collector"; };
  collector-windows = mkBinary { group = win; name = "collector"; suffix = ".exe"; };
  bootstrap-windows = mkBinary { group = win; name = "bootstrap"; suffix = ".exe"; };
}
