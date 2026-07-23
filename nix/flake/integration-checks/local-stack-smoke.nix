{
  pkgs,
  root,
  ...
}:

let
  args = {
    inherit pkgs root;
  };
in
import ./local-stack-smoke/local-storage.nix args
// import ./local-stack-smoke/network-remote.nix args
