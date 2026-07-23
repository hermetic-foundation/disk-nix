{ pkgs, self }:

{
  nixosModuleCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        filesystems.local = {
          device = "/dev/disk/by-label/local";
          fsType = "xfs";
          mountpoint = "/srv/collision";
        };
        filesystems.secondary = {
          device = "/dev/disk/by-label/secondary";
          fsType = "ext4";
          mountpoint = "/srv/collision";
        };
        swaps.primary.path = "/dev/disk/by-label/swap-collision";
        swaps.secondary.target = "/dev/disk/by-label/swap-collision";
        luks.devices.primary = {
          target = "crypt-collision";
          device = "/dev/disk/by-id/primary-luks";
        };
        luks.devices.secondary = {
          mapper = "crypt-collision";
          device = "/dev/disk/by-id/secondary-luks";
        };
        exports.primary = {
          path = "/srv/export-collision";
          client = "192.0.2.0/24";
          options = "rw,sync";
        };
        exports.secondary = {
          target = "/srv/export-collision";
          client = "192.0.2.0/24";
          options = "ro,sync";
        };
      };
    }
  ];
  nixosModuleDiskCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        disks."/dev/disk/by-id/nvme-root".operation = "rescan";
        disks.rootAlias = {
          path = "/dev/disk/by-id/nvme-root";
          operation = "grow";
        };
      };
    }
  ];
  nixosModulePartitionCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        partitions.root = {
          device = "/dev/disk/by-id/nvme-root";
          number = "2";
          operation = "grow";
        };
        partitions.rootAlias = {
          device = "/dev/disk/by-id/nvme-root";
          partitionNumber = "2";
          operation = "rescan";
        };
      };
    }
  ];
  nixosModuleLuksKeyslotCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        luksKeyslots.rootAdd = {
          operation = "add-key";
          device = "/dev/disk/by-id/root-luks";
          keySlot = "4";
          newKeyFile = "/run/keys/root-new";
        };
        luksKeyslots.rootRotate = {
          device = "/dev/disk/by-id/root-luks";
          "key-slot" = "4";
          "key-file" = "/run/keys/root-old";
          properties.keyFile = "/run/keys/root-rotated";
        };
      };
    }
  ];
  nixosModuleLuksTokenCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        luksTokens.rootImport = {
          operation = "import-token";
          device = "/dev/disk/by-id/root-luks";
          tokenId = "7";
          tokenFile = "/run/keys/root-token.json";
        };
        luksTokens.rootRotate = {
          device = "/dev/disk/by-id/root-luks";
          "token-id" = "7";
          properties.tokenFile = "/run/keys/root-token-rotated.json";
        };
      };
    }
  ];
  nixosModuleBackingFileCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        backingFiles.rootImage = {
          operation = "rescan";
          path = "/var/lib/images/root.img";
        };
        backingFiles.duplicateRootImage = {
          operation = "grow";
          target = "/var/lib/images/root.img";
          desiredSize = "16GiB";
        };
      };
    }
  ];
  nixosModuleBtrfsSubvolumeCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        btrfsSubvolumes."/mnt/persist/@home".operation = "rescan";
        btrfsSubvolumes.homeAlias = {
          path = "/mnt/persist/@home";
          operation = "create";
        };
      };
    }
  ];
  nixosModuleBtrfsQgroupCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        btrfsQgroups."0/257" = {
          target = "/mnt/persist";
          operation = "rescan";
        };
        btrfsQgroups.homeLimit = {
          target = "0/257";
          path = "/mnt/persist";
          properties.limit = "25GiB";
        };
      };
    }
  ];
  nixosModuleDmMapCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        dmMaps.cryptroot = {
          operation = "rescan";
          target = "/dev/mapper/cryptroot";
        };
        dmMaps.duplicateCryptroot = {
          operation = "rescan";
          path = "/dev/mapper/cryptroot";
        };
      };
    }
  ];
  nixosModuleVdoVolumeCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        vdoVolumes.archive.operation = "rescan";
        vdoVolumes.archiveAlias = {
          target = "archive";
          operation = "grow";
          desiredSize = "4TiB";
        };
      };
    }
  ];
  nixosModulePhysicalVolumeCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        physicalVolumes."/dev/disk/by-id/nvme-pv".operation = "rescan";
        physicalVolumes.nvmeAlias = {
          path = "/dev/disk/by-id/nvme-pv";
          operation = "grow";
        };
      };
    }
  ];
  nixosModuleLoopDeviceCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        loopDevices."/dev/loop7".operation = "rescan";
        loopDevices.rootImage = {
          target = "/dev/loop7";
          operation = "create";
          device = "/var/lib/images/root.img";
        };
      };
    }
  ];
}
