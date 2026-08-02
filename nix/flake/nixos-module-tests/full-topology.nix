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
        solve.layouts.desktop = {
          disks = {
            nvme = {
              path = "/dev/disk/by-id/nvme-solver-os";
              size = "232.9G";
              media = "nvme";
              primaryBoot = true;
            };
            ssd = {
              path = "/dev/disk/by-id/ata-solver-ssd";
              size = "465.8G";
              media = "ssd";
            };
            hdd = {
              path = "/dev/disk/by-id/ata-solver-hdd";
              size = "931.5G";
              media = "hdd";
            };
          };
          boot = {
            type = "efi-replicated";
            size = "1GiB";
            mountpoint = "/boot";
          };
          swap = {
            type = "tail";
            priorities = {
              nvme = 10;
              ssd = 5;
              hdd = 1;
            };
          };
          zfs = {
            pool = "zroot";
            sliceSize = "100GiB";
            vdevs.prefer = [
              {
                type = "raidz1";
                width = 3;
              }
              {
                type = "mirror";
                width = 2;
              }
            ];
            vdevs.unassignedSlicePolicy = "allow";
          };
        };
      };
    }
    ./full-topology/local-block.nix
    ./full-topology/network-and-advanced.nix
    ./full-topology/volumes-and-maps.nix
    ./full-topology/shared-and-snapshots.nix
  ];
}
