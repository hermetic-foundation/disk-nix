{ pkgs, self }:

let
  args = { inherit pkgs self; };
in
(import ./nixos-module-tests/full-topology.nix args)
// (import ./nixos-module-tests/modes.nix args)
// (import ./nixos-module-tests/collisions-block.nix args)
// (import ./nixos-module-tests/collisions-storage.nix args)
