{
  services.disk-nix = {
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
