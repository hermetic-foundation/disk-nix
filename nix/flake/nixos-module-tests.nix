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
        luks.devices.cryptroot = {
          device = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
          operation = "grow";
          desiredSize = "100%";
          allowDiscards = true;
          properties.label = "cryptroot";
          properties."luks.subsystem" = "nixos";
        };
        luks.devices.cryptTargetSize = {
          target = "cryptTargetSizeMapper";
          device = "/dev/disk/by-id/target-size-luks";
          operation = "grow";
          targetSize = "90%";
        };
        luks.devices.cryptSize = {
          device = "/dev/disk/by-id/size-luks";
          operation = "grow";
          size = "80%";
        };
        luks.devices.cryptold = {
          device = "/dev/disk/by-partuuid/old-luks";
          destroy = true;
        };
        luks.devices.cryptarchive = {
          device = "/dev/disk/by-id/archive-luks";
          operation = "open";
        };
        luks.devices.cryptclosed = {
          device = "/dev/disk/by-id/closed-luks";
          operation = "close";
        };
        filesystems.root = {
          device = "/dev/disk/by-label/nixos-root";
          fsType = "xfs";
          mountpoint = "/";
          neededForBoot = true;
          resizePolicy = "grow-only";
          desiredSize = "100%";
        };
        filesystems.data = {
          device = "/dev/disk/by-label/data";
          fsType = "btrfs";
          mountpoint = "/data";
          operation = "rebalance";
          addDevices = [ "/dev/disk/by-id/nvme-btrfs-new" ];
          removeDevices = [ "/dev/disk/by-id/nvme-btrfs-old" ];
          replaceDevices = {
            "/dev/disk/by-id/nvme-btrfs-aging" = "/dev/disk/by-id/nvme-btrfs-replacement";
          };
          properties = {
            label = "bulk-data";
            "btrfs.balance.data" = "usage=50";
          };
          metadata = {
            pool = "tank";
            role = "bulk-data";
          };
        };
        filesystems.scratch = {
          device = "/dev/disk/by-label/scratch";
          fsType = "xfs";
          mountpoint = "/scratch";
          operation = "check";
        };
        filesystems.scrub = {
          device = "/dev/disk/by-label/scrub";
          fsType = "btrfs";
          mountpoint = "/scrub";
          operation = "scrub";
        };
        filesystems.trim = {
          device = "/dev/disk/by-label/trim";
          fsType = "xfs";
          mountpoint = "/trim";
          operation = "trim";
        };
        filesystems.remount = {
          device = "/dev/disk/by-label/remount";
          fsType = "xfs";
          mountpoint = "/remount";
          operation = "remount";
          options = [
            "rw"
            "noatime"
            "discard=async"
          ];
        };
        filesystems.localMount = {
          device = "/dev/disk/by-label/local-mount";
          fsType = "xfs";
          mountpoint = "/mnt/local-mount";
          operation = "mount";
          options = [
            "rw"
            "noatime"
          ];
        };
        filesystems.localUnmount = {
          device = "/dev/disk/by-label/local-unmount";
          fsType = "ext4";
          mountpoint = "/mnt/local-unmount";
          operation = "unmount";
        };
        filesystems.localRescan = {
          device = "/dev/disk/by-label/local-rescan";
          fsType = "xfs";
          mountpoint = "/mnt/local-rescan";
          operation = "rescan";
        };
        filesystems.actionRescan = {
          device = "/dev/disk/by-label/action-rescan";
          fsType = "xfs";
          mountpoint = "/mnt/action-rescan";
          action = "rescan";
        };
        filesystems.actionUnmount = {
          device = "/dev/disk/by-label/action-unmount";
          fsType = "xfs";
          mountpoint = "/mnt/action-unmount";
          action = "unmount";
        };
        filesystems.teardownOnly = {
          device = "/dev/disk/by-label/teardown-only";
          fsType = "jfs";
          mountpoint = "/mnt/teardown-only";
          operation = "unmount";
        };
        filesystems.destroyed = {
          device = "/dev/disk/by-label/destroyed";
          fsType = "ext4";
          mountpoint = "/mnt/destroyed";
          destroy = true;
        };
        filesystems.targetSizeAlias = {
          device = "/dev/disk/by-label/target-size";
          fsType = "xfs";
          mountpoint = "/mnt/target-size";
          operation = "rescan";
          targetSize = "200GiB";
        };
        filesystems.sizeAlias = {
          device = "/dev/disk/by-label/size-alias";
          fsType = "ext4";
          mountpoint = "/mnt/size-alias";
          operation = "rescan";
          size = "150GiB";
        };
        filesystems.runTmpfs = {
          device = "tmpfs";
          fsType = "tmpfs";
          mountpoint = "/run/disk-nix-tmp";
          options = [
            "mode=0755"
            "size=64M"
            "nosuid"
            "nodev"
          ];
        };
        filesystems.bindCache = {
          device = "/var/cache/disk-nix";
          fsType = "none";
          mountpoint = "/srv/disk-nix-cache";
          options = [
            "bind"
            "x-systemd.requires-mounts-for=/var/cache/disk-nix"
          ];
        };
        filesystems.overlayScratch = {
          device = "overlay";
          fsType = "overlay";
          mountpoint = "/srv/disk-nix-overlay";
          options = [
            "lowerdir=/nix/store"
            "upperdir=/var/lib/disk-nix/overlay/upper"
            "workdir=/var/lib/disk-nix/overlay/work"
            "index=off"
          ];
        };
        filesystems.mobile = {
          device = "/dev/disk/by-label/mobile";
          fsType = "f2fs";
          mountpoint = "/mobile";
          operation = "check";
        };
        filesystems.bulk = {
          device = "/dev/disk/by-label/bulk";
          fsType = "bcachefs";
          mountpoint = "/bulk";
          operation = "repair";
        };
        swaps.primary = {
          device = "/dev/disk/by-label/swap";
          operation = "format";
          desiredSize = "8GiB";
          priority = 5;
          properties.label = "swap";
          properties."swap.uuid" = "01234567-89ab-cdef-0123-456789abcdef";
        };
        swaps.inventory = {
          device = "/dev/disk/by-label/swap-inventory";
          operation = "rescan";
        };
        swaps.targetSizeAlias = {
          device = "/dev/disk/by-label/swap-target-size";
          operation = "grow";
          targetSize = "12GiB";
        };
        swaps.sizeAlias = {
          device = "/dev/disk/by-label/swap-size";
          operation = "grow";
          size = "10GiB";
        };
        swaps.old = {
          device = "/dev/disk/by-label/old-swap";
          operation = "destroy";
        };
        swaps.actionOld = {
          device = "/dev/disk/by-label/action-old-swap";
          action = "destroy";
        };
        swaps.destroyed = {
          device = "/dev/disk/by-label/destroyed-swap";
          destroy = true;
        };
        zram = {
          enable = true;
          operation = "rescan";
          swapDevices = 2;
          memoryPercent = 40;
          memoryMax = 8589934592;
          priority = 20;
          algorithm = "zstd";
          properties."zram.compression-ratio-target" = "2.0";
        };
        luks.devices.cryptaction = {
          device = "/dev/disk/by-id/action-luks";
          action = "open";
        };
        nfs.mounts.shared = {
          source = "nas.example.com:/srv/shared";
          mountpoint = "/srv/shared";
          fsType = "nfs4";
          operation = "mount";
          options = [
            "_netdev"
            "x-systemd.automount"
            "vers=4.2"
          ];
          metadata = {
            server = "nas.example.com";
            export = "/srv/shared";
          };
        };
        nfs.mounts."/srv/tuned" = {
          source = "nas.example.com:/srv/tuned";
          fsType = "nfs4";
          operation = "remount";
          options = [
            "_netdev"
            "ro"
            "vers=4.2"
          ];
        };
        nfs.mounts."/srv/action" = {
          source = "nas.example.com:/srv/action";
          fsType = "nfs4";
          action = "remount";
        };
        nfs.mounts."/srv/inventory" = {
          source = "nas.example.com:/srv/inventory";
          fsType = "nfs4";
          operation = "rescan";
        };
        nfs.mounts."/srv/old" = {
          source = "nas.example.com:/srv/old";
          operation = "unmount";
        };
        iscsi = {
          initiatorName = "iqn.2026-06.example:host";
          enableAutoLoginOut = true;
          boot = {
            enable = true;
            target = "iqn.2026-06.example:storage.root";
          };
          sessions."iqn.2026-06.example:storage.root" = {
            operation = "grow";
            desiredSize = "2TiB";
            portal = "192.0.2.10:3260";
          };
          sessions."iqn.2026-06.example:storage.alias" = {
            operation = "grow";
            targetSize = "3TiB";
            portal = "192.0.2.10:3260";
          };
          sessions."iqn.2026-06.example:storage.login" = {
            operation = "login";
            portal = "192.0.2.10:3260";
          };
          sessions."iqn.2026-06.example:storage.logout" = {
            operation = "logout";
            portal = "192.0.2.11:3260";
          };
          sessions."iqn.2026-06.example:storage.rescan" = {
            operation = "rescan";
            portal = "192.0.2.10:3260";
          };
        };
        pools.tank = {
          operation = "rebalance";
          addDevices = [ "/dev/disk/by-id/nvme-replacement" ];
          removeDevices = [ "/dev/disk/by-id/old-disk" ];
          properties.autotrim = "on";
        };
        pools.vault = {
          operation = "import";
          readOnly = true;
        };
        pools.archiveImport = {
          operation = "import";
          readonly = true;
        };
        pools.moveme.operation = "export";
        volumeGroups.importvg.operation = "import";
        volumeGroups.exportvg.operation = "export";
        volumeGroups.activevg.operation = "activate";
        volumeGroups.refreshvg.operation = "rescan";
        volumeGroups.actionvg.action = "rescan";
        partitions.root = {
          operation = "grow";
          device = "/dev/disk/by-id/nvme-root";
          number = "2";
          endOffset = "100%";
        };
        partitions.dataTable = {
          operation = "rescan";
          device = "/dev/disk/by-id/nvme-data";
        };
        vdoVolumes.archiveLifecycle = {
          target = "archive";
          operation = "grow";
          desiredSize = "4TiB";
          physicalSize = "6TiB";
          properties = {
            writePolicy = "sync";
            compression = "enabled";
            deduplication = "disabled";
          };
        };
        vdoVolumes.warmArchive = {
          target = "warm-archive";
          operation = "start";
        };
        vdoVolumes.coldArchive = {
          target = "cold-archive";
          operation = "stop";
        };
        vdoVolumes.refreshArchive = {
          target = "refresh-archive";
          operation = "rescan";
        };
        physicalVolumes.nvmePvGrow = {
          operation = "grow";
          path = "/dev/disk/by-id/nvme-pv-grow";
        };
        physicalVolumes."/dev/disk/by-id/nvme-pv-refresh" = {
          operation = "rescan";
        };
        luksKeyslots."cryptroot:1" = {
          operation = "add-key";
          device = "/dev/disk/by-id/root-luks";
          keySlot = "1";
          newKeyFile = "/run/keys/root-new";
        };
        luksKeyslots."cryptroot:2" = {
          operation = "remove-key";
          device = "/dev/disk/by-id/root-luks";
          keySlot = "2";
        };
        luksKeyslots."cryptroot:3" = {
          operation = "add-key";
          device = "/dev/disk/by-id/root-luks";
          "key-slot" = "3";
          "new-key-file" = "/run/keys/root-new-alias";
        };
        luksKeyslots."cryptroot:4" = {
          operation = "remove-key";
          device = "/dev/disk/by-id/root-luks";
          slot = "4";
        };
        luksTokens."cryptroot:0" = {
          operation = "import-token";
          device = "/dev/disk/by-id/root-luks";
          tokenId = "0";
          tokenFile = "/run/keys/root-token.json";
        };
        luksTokens."cryptroot:1" = {
          operation = "remove-token";
          device = "/dev/disk/by-id/root-luks";
          tokenId = "1";
        };
        luksTokens."cryptroot:2" = {
          operation = "import-token";
          device = "/dev/disk/by-id/root-luks";
          token = "2";
          "token-file" = "/run/keys/root-token-alias.json";
        };
        luksTokens."cryptroot:3" = {
          operation = "remove-token";
          device = "/dev/disk/by-id/root-luks";
          "token-id" = "3";
        };
        btrfsSubvolumes."/mnt/persist/@home" = {
          operation = "create";
          path = "/mnt/persist/@home";
        };
        btrfsSubvolumes."/mnt/persist/@inventory" = {
          operation = "rescan";
          path = "/mnt/persist/@inventory";
        };
        btrfsSubvolumes."/mnt/persist/@old-name" = {
          operation = "rename";
          renameTo = "/mnt/persist/@new-name";
        };
        btrfsQgroups."0/257" = {
          target = "/mnt/persist";
          properties.limit = "25GiB";
        };
        btrfsQgroups."0/258" = {
          operation = "rescan";
          target = "/mnt/persist";
        };
        volumes.scratch = {
          operation = "create";
          target = "vg0/scratch";
          desiredSize = "10GiB";
        };
        volumes."vg0/size-alias" = {
          operation = "create";
          size = "12GiB";
        };
        volumes."vg0/archive".operation = "deactivate";
        volumes."vg0/reporting".operation = "rescan";
        datasets."tank/archive" = {
          destroy = true;
        };
        datasets."tank/home" = {
          operation = "create";
        };
        datasets."tank/legacy" = {
          operation = "rename";
          renameTo = "tank/legacy-staged";
        };
        datasets."tank/legacy-alias" = {
          operation = "rename";
          renameTarget = "tank/legacy-alias-staged";
        };
        datasets."tank/legacy-short" = {
          operation = "rename";
          newName = "tank/legacy-short-staged";
        };
        datasets."tank/home-review" = {
          operation = "promote";
        };
        datasets."tank/inventory" = {
          operation = "rescan";
        };
        zvols."tank/vm/root" = {
          operation = "grow";
          desiredSize = "80GiB";
        };
        zvols."tank/vm/inventory" = {
          operation = "rescan";
        };
        thinPools.primaryPool = {
          operation = "grow";
          path = "vg0/thinpool";
          desiredSize = "500GiB";
        };
        thinPools."vg0/newthin" = {
          operation = "create";
          desiredSize = "100GiB";
        };
        thinPools."vg0/reporting".operation = "rescan";
        lvmSnapshots."vg0/root-snap" = {
          operation = "snapshot";
          target = "vg0/root";
          desiredSize = "20GiB";
        };
        lvmSnapshots."vg0/root-inspect".operation = "rescan";
        lvmCaches."vg0/root" = {
          operation = "create";
          device = "vg0/root-cache";
          properties."lvm.cache-mode" = "writethrough";
        };
        lvmCaches."vg0/archive".operation = "rescan";
        loopDevices.rootImage = {
          operation = "create";
          path = "/dev/loop7";
          device = "/var/lib/images/root.img";
        };
        loopDevices."/dev/loop10".operation = "rescan";
        backingFiles."/var/lib/images/new.img" = {
          operation = "create";
          desiredSize = "8GiB";
        };
        backingFiles."/var/lib/images/root.img" = {
          operation = "grow";
          desiredSize = "16GiB";
        };
        backingFiles.inventoryImage = {
          operation = "rescan";
          path = "/var/lib/images/inventory.img";
        };
        dmMaps.cryptroot = {
          operation = "rescan";
          target = "/dev/mapper/cryptroot";
        };
        dmMaps.cryptswap = {
          operation = "rename";
          target = "/dev/mapper/cryptswap";
          renameTo = "cryptswap-retired";
        };
        dmMaps.oldmap = {
          operation = "destroy";
          target = "/dev/mapper/oldmap";
        };
        mdRaids.root = {
          target = "/dev/md/root";
          raidLevel = "1";
          devices = [
            "/dev/disk/by-id/nvme-md-a"
            "/dev/disk/by-id/nvme-md-b"
          ];
          addDevices = [ "/dev/disk/by-id/nvme-md-spare" ];
          replaceDevices = {
            "/dev/disk/by-id/nvme-md-aging" = "/dev/disk/by-id/nvme-md-replacement";
          };
        };
        mdRaids.existing = {
          target = "/dev/md/existing";
          operation = "assemble";
          devices = [
            "/dev/disk/by-id/existing-md-a"
            "/dev/disk/by-id/existing-md-b"
          ];
        };
        mdRaids.oldroot = {
          target = "/dev/md/oldroot";
          operation = "stop";
        };
        mdRaids.inventory.operation = "rescan";
        multipathMaps.mpatha = {
          target = "mpatha";
          addDevices = [ "/dev/sdb" ];
          replaceDevices = {
            "/dev/sdc" = "/dev/sdd";
          };
        };
        multipathMaps.mpathb = {
          target = "mpathb";
          operation = "rescan";
        };
        multipathMaps.mpathOld = {
          target = "mpath-old";
          operation = "destroy";
        };
        luns."iqn.2026-06.example:storage/root:0" = {
          operation = "grow";
          device = "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0";
          devices = [
            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0"
          ];
          metadata = {
            target = "iqn.2026-06.example:storage/root";
            lun = 0;
          };
        };
        luns."iqn.2026-06.example:storage/new:2" = {
          operation = "attach";
          device = "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-2";
        };
        luns."iqn.2026-06.example:storage/old:3" = {
          operation = "detach";
          devices = [
            "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3"
          ];
        };
        luns."iqn.2026-06.example:storage/rescan:4" = {
          operation = "rescan";
          paths = [
            "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-4"
          ];
        };
        nvmeNamespaces.rootNamespace = {
          operation = "create";
          path = "/dev/nvme0";
          desiredSize = "100G";
          namespaceId = "4";
          controllers = "0x1";
        };
        nvmeNamespaces."/dev/nvme1".operation = "rescan";
        nvmeNamespaces."/dev/nvme2" = {
          operation = "attach";
          nsid = "7";
          controllerId = "0x2";
        };
        nvmeNamespaces."/dev/nvme3" = {
          operation = "detach";
          namespaceId = "8";
          controller = "0x3";
        };
        exports.share = {
          operation = "export";
          path = "/srv/share";
          client = "192.0.2.0/24";
          options = "rw,sync,no_subtree_check";
        };
        exports."/srv/inventory".operation = "rescan";
        exports."/srv/old-share" = {
          operation = "unexport";
          client = "192.0.2.55";
        };
        caches."tank/l2arc0" = {
          operation = "replace-device";
          replaceDevices = {
            "/dev/disk/by-id/old-cache" = "/dev/disk/by-id/new-cache";
          };
          cacheSetUuid = "11111111-2222-3333-4444-555555555555";
        };
        caches."/dev/bcache0" = {
          operation = "rescan";
          addDevices = [ "cache-set-uuid" ];
          cacheSetUuid = "cache-set-uuid";
          properties."bcache.cache-mode" = "writethrough";
          properties."bcache.set-journal-delay-ms" = "100";
        };
        snapshots."tank/home@before-upgrade" = {
          target = "tank/home";
          hold = "disk-nix-retain";
          rollback = true;
          cloneTo = "tank/home-review";
          renameTo = "tank/home@before-upgrade-retained";
          recursiveRollback = true;
        };
        snapshots."tank/home@clone-only" = {
          operation = "clone";
          target = "tank/home";
          cloneTo = "tank/home-clone";
        };
        snapshots."tank/home@action-rescan" = {
          action = "rescan";
          target = "tank/home";
        };
        snapshots.aliases = {
          operation = "clone";
          target = "tank/home";
          "snapshot-path" = "tank/home@alias-source";
          cloneTarget = "tank/home-alias-clone";
          clone = "tank/home-short-clone";
          renameTarget = "tank/home@alias-retained";
          newName = "tank/home@alias-new";
          recursive = true;
          "zfs.rollbackRecursive" = true;
          readonly = true;
        };
        snapshots."tank/home@old" = {
          target = "tank/home";
          releaseHold = "old-retention";
        };
        snapshots."/mnt/persist/@home-before-upgrade" = {
          target = "/mnt/persist/@home";
          readOnly = true;
        };
        snapshots."/mnt/persist/@home-before-clone" = {
          target = "/mnt/persist/@home";
          cloneTo = "/mnt/persist/@home-review";
          readOnly = true;
        };
        snapshots."tank/home@inventory" = {
          operation = "rescan";
          target = "tank/home";
        };
        snapshots."/mnt/persist/@home-inventory" = {
          operation = "rescan";
          target = "/mnt/persist/@home";
          readOnly = true;
        };
        snapshots.home-before-friendly = {
          operation = "rescan";
          target = "/mnt/persist/@home";
          snapshotPath = "/mnt/persist/@home-before-friendly";
        };
      };
    }
  ];
  zramTuningOnlyModuleTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        zram = {
          swapDevices = 3;
          memoryPercent = 35;
          priority = 15;
          algorithm = "lz4";
          preserveData = false;
        };
      };
    }
  ];
  nixosModuleExecuteTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        apply = {
          mode = "activation";
          execute = true;
          probeCurrent = true;
          failOnBlocked = true;
          scriptOut = "/run/disk-nix/execute.sh";
          reportOut = "/run/disk-nix/execute-report.json";
          receiptOut = "/run/disk-nix/execute-receipt.json";
        };
      };
    }
  ];
  nixosModuleHandoffAutoImportTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        apply = {
          mode = "activation";
          execute = true;
          failOnBlocked = true;
          reportOut = "/run/disk-nix/handoff-report.json";
          declarativeHandoff.autoImport = {
            enable = true;
            configurationPath = "/etc/nixos/storage.nix";
            backupDirectory = "/var/backups/disk-nix-handoff";
          };
        };
      };
    }
  ];
  nixosModuleBootModeTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        apply.mode = "boot";
      };
    }
  ];
  nixosModuleInstallModeTest = pkgs.nixos [
    self.nixosModules.default
    {
      system.stateVersion = "26.05";
      boot.loader.grub.enable = false;
      services.disk-nix = {
        enable = true;
        apply.mode = "install";
      };
    }
  ];
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
