{ crane }:

crane.buildPackage {
  src = crane.cleanCargoSource ./.;
  cargoVendorDir = crane.vendorCargoDeps {
    src = ./.;
    cargoLock =  ./../Cargo.lock;
  };
}
