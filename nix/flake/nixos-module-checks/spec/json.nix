{
  pkgs,
  nixosModuleTest,
  ...
}:

let
  filterParts = [
    (import ./json/local-storage.nix)
    (import ./json/network-targets.nix)
    (import ./json/advanced-local.nix)
    (import ./json/snapshots-apply.nix)
  ];
  filter = pkgs.writeText "disk-nix-nixos-module-spec-json.jq" (
    builtins.concatStringsSep "\n" filterParts
  );
in
pkgs.runCommand "disk-nix-nixos-module-spec-json-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
  spec=${nixosModuleTest.config.environment.etc."disk-nix/spec.json".source}
  jq -e -f ${filter} "$spec"
  touch "$out"
''
