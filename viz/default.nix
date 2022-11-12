{ stdenv }:

stdenv.mkDerivation {
  name = "viz";
  version = "0.0.1";

  src = ./.;

  installPhase = ''
    mkdir -p $out/share/www

    cp src/index.html \
      src/utils.js \
      src/websocket_test.js \
      $out/share/www/

    cp -r src/views $out/share/www/views
  '';
}
