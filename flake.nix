{
  description = "NixOS-native storage topology and lifecycle manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      perSystem = forAllSystems (
        system:
        import ./nix/flake/per-system.nix {
          inherit self nixpkgs system;
          root = ./.;
        }
      );
    in
    {
      formatter = forAllSystems (system: perSystem.${system}.formatter);
      packages = forAllSystems (system: perSystem.${system}.packages);
      apps = forAllSystems (system: perSystem.${system}.apps);
      checks = forAllSystems (system: perSystem.${system}.checks);
      devShells = forAllSystems (system: perSystem.${system}.devShells);
      nixosModules = import ./nix/flake/nixos-modules.nix { inherit self; };
      overlays.default = final: _prev: {
        disk-nix = self.packages.${final.stdenv.hostPlatform.system}.disk-nix;
      };
    };
}
