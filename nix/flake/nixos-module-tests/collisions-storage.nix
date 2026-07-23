{ pkgs, self }:

{
  nixosModuleMdRaidCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        mdRaids."/dev/md/root" = {
          operation = "assemble";
          devices = [
            "/dev/disk/by-id/md-a"
            "/dev/disk/by-id/md-b"
          ];
        };
        mdRaids.rootAlias = {
          target = "/dev/md/root";
          operation = "rescan";
        };
      };
    }
  ];
  nixosModuleMultipathMapCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        multipathMaps.mpatha = {
          operation = "rescan";
        };
        multipathMaps.primaryPath = {
          target = "mpatha";
          operation = "grow";
        };
      };
    }
  ];
  nixosModuleNvmeNamespaceCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        nvmeNamespaces.root = {
          path = "/dev/nvme0";
          namespaceId = "4";
          operation = "rescan";
        };
        nvmeNamespaces.rootAlias = {
          target = "/dev/nvme0";
          nsid = "4";
          operation = "grow";
        };
      };
    }
  ];
  nixosModuleCacheCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        caches."/dev/bcache0".operation = "rescan";
        caches.writeback = {
          target = "/dev/bcache0";
          operation = "add-device";
          addDevices = [ "cache-set-uuid" ];
        };
      };
    }
  ];
  nixosModulePoolCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        pools.tank.operation = "rescan";
        pools.primaryPool = {
          target = "tank";
          operation = "import";
        };
      };
    }
  ];
  nixosModuleDatasetCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        datasets."tank/home".operation = "rescan";
        datasets.homeAlias = {
          target = "tank/home";
          operation = "create";
        };
      };
    }
  ];
  nixosModuleZvolCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        zvols."tank/vm/root".operation = "rescan";
        zvols.vmRootAlias = {
          path = "tank/vm/root";
          operation = "grow";
          desiredSize = "80GiB";
        };
      };
    }
  ];
  nixosModuleVolumeGroupCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        volumeGroups.vg0.operation = "rescan";
        volumeGroups.primaryVg = {
          target = "vg0";
          operation = "activate";
        };
      };
    }
  ];
  nixosModuleVolumeCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        volumes."vg0/root".operation = "rescan";
        volumes.rootAlias = {
          path = "vg0/root";
          operation = "grow";
          desiredSize = "80GiB";
        };
      };
    }
  ];
  nixosModuleThinPoolCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        thinPools."vg0/thinpool".operation = "rescan";
        thinPools.primaryThin = {
          target = "vg0/thinpool";
          operation = "grow";
          desiredSize = "500GiB";
        };
      };
    }
  ];
  nixosModuleLvmCacheCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        lvmCaches."vg0/root".operation = "rescan";
        lvmCaches.rootCacheAlias = {
          target = "vg0/root";
          operation = "create";
          device = "vg0/root-cache";
        };
      };
    }
  ];
  nixosModuleSnapshotCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        snapshots."/mnt/persist/@home-before" = {
          target = "/mnt/persist/@home";
          readOnly = true;
        };
        snapshots.homeBeforeAlias = {
          target = "/mnt/persist/@home";
          snapshotPath = "/mnt/persist/@home-before";
          operation = "rescan";
        };
      };
    }
  ];
  nixosModuleIscsiSessionCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        iscsi.sessions."iqn.2026-06.example:storage.root" = {
          portal = "192.0.2.10:3260";
          operation = "rescan";
        };
        iscsi.sessions.rootAlias = {
          target = "iqn.2026-06.example:storage.root";
          portal = "192.0.2.11:3260";
          operation = "login";
        };
      };
    }
  ];
  nixosModuleLunPathCollisionTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        luns.rootPrimary = {
          operation = "rescan";
          device = "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0";
        };
        luns.rootMultipath = {
          operation = "attach";
          paths = [
            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
          ];
        };
      };
    }
  ];
}
