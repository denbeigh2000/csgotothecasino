{ stdenv }:

stdenv.mkDerivation {
  name = "viz";
  version = "0.0.1";

  src = ./.;

  installPhase = ''
    mkdir -p $out/share/www

    cp src/*.html \
      src/*.js \
      $out/share/www/

    cp -r src/views $out/share/www/views
  '';
}
