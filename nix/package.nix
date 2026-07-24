{ pkgs }:
pkgs.rustPlatform.buildRustPackage {
  pname = "nirikit";
  version = "0.1.0";
  src = pkgs.lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "Launch applications on niri workspaces";
    license = pkgs.lib.licenses.mit;
    mainProgram = "nirikit";
    platforms = pkgs.lib.platforms.linux;
  };
}
