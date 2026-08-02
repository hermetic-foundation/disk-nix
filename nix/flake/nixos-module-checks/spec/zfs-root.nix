args@{
  pkgs,
  self,
  nixosModuleTest,
  ...
}:

let
  zfsRootModuleTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      networking.hostId = "d15c0001";

      boot.loader.grub.enable = false;

      services.disk-nix = {
        enable = true;
        filesystems.root = {
          device = "zroot/root";
          fsType = "zfs";
          mountpoint = "/";
          neededForBoot = true;
        };
        datasets."zroot/root" = {
          operation = "rescan";
          properties.mountpoint = "legacy";
        };
      };
    }
  ];
in
pkgs.runCommand "disk-nix-nixos-module-zfs-root-check" { } ''
  nonRootForce=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleTest.config.boot.zfs.forceImportRoot)}
  rootForce=${pkgs.lib.escapeShellArg (builtins.toJSON zfsRootModuleTest.config.boot.zfs.forceImportRoot)}

  test "$nonRootForce" = false
  test "$rootForce" = true

  touch "$out"
''
