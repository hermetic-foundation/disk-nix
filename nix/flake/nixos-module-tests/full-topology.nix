{ pkgs, self }:

{
  nixosModuleTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      networking.hostId = "8425e349";
      boot.loader.grub.enable = false;
      boot.initrd.systemd.enable = false;
      services.disk-nix = {
        enable = true;
        apply = {
          mode = "activation";
          probeCurrent = true;
          allowDeviceReplacement = true;
          allowRebalance = true;
          allowPotentialDataLoss = false;
          requireBackup = false;
          backupVerified = false;
          requireConfirmation = false;
          confirmation = false;
          requireConfirmationFile = "/run/disk-nix/confirm";
          failOnBlocked = false;
          scriptOut = "/run/disk-nix/apply.sh";
          reportOut = "/run/disk-nix/apply-report.json";
          receiptOut = "/run/disk-nix/apply-receipt.json";
        };
      };
    }
    ./full-topology/local-block.nix
    ./full-topology/network-and-advanced.nix
    ./full-topology/volumes-and-maps.nix
    ./full-topology/shared-and-snapshots.nix
  ];
}
