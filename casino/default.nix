{ buildPackage
, writeScriptBin
, stdenv
, pkg-config
, openssl
}:

let
  version = "0.0.1";
  group = buildPackage {
    pname = "casino";
    inherit version;

    src = ./.;
    root = ./.;

    nativeBuildInputs = [ pkg-config openssl ];
  };

  mkBinary = name: writeScriptBin name "${group}/bin/${name} $@";
in
{
  aggregator = mkBinary "aggregator";
  bootstrap = mkBinary "bootstrap";
  collector = mkBinary "collector";
}
