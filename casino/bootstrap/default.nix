{ crane }:

crane.buildPackage {
  src = crane.cleanCargoSource ./.;
}
