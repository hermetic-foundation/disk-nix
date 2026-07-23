{
  services.disk-nix = {
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
  };
}
