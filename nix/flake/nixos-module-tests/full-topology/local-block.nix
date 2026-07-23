{
  services.disk-nix = {
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
  };
}
