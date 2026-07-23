''
  .version == 1
  and .spec.filesystems.root.device == "/dev/disk/by-label/nixos-root"
  and .spec.filesystems.root.resizePolicy == "grow-only"
  and .spec.filesystems.root.desiredSize == "100%"
  and .spec.filesystems.data.device == "/dev/disk/by-label/data"
  and .spec.filesystems.data.fsType == "btrfs"
  and .spec.filesystems.data.operation == "rebalance"
  and (.spec.filesystems.data.addDevices | index("/dev/disk/by-id/nvme-btrfs-new") != null)
  and (.spec.filesystems.data.removeDevices | index("/dev/disk/by-id/nvme-btrfs-old") != null)
  and .spec.filesystems.data.replaceDevices."/dev/disk/by-id/nvme-btrfs-aging" == "/dev/disk/by-id/nvme-btrfs-replacement"
  and .spec.filesystems.data.properties.label == "bulk-data"
  and .spec.filesystems.data.properties."btrfs.balance.data" == "usage=50"
  and .spec.filesystems.data.metadata.pool == "tank"
  and .spec.filesystems.data.metadata.role == "bulk-data"
  and .spec.filesystems.scratch.operation == "check"
  and .spec.filesystems.scratch.device == "/dev/disk/by-label/scratch"
  and .spec.filesystems.scrub.operation == "scrub"
  and .spec.filesystems.scrub.device == "/dev/disk/by-label/scrub"
  and .spec.filesystems.scrub.mountpoint == "/scrub"
  and .spec.filesystems.trim.operation == "trim"
  and .spec.filesystems.trim.device == "/dev/disk/by-label/trim"
  and .spec.filesystems.remount.operation == "remount"
  and .spec.filesystems.remount.mountpoint == "/remount"
  and (.spec.filesystems.remount.options | index("discard=async") != null)
  and .spec.filesystems.localMount.operation == "mount"
  and .spec.filesystems.localMount.device == "/dev/disk/by-label/local-mount"
  and .spec.filesystems.localMount.mountpoint == "/mnt/local-mount"
  and (.spec.filesystems.localMount.options | index("noatime") != null)
  and .spec.filesystems.localUnmount.operation == "unmount"
  and .spec.filesystems.localUnmount.device == "/dev/disk/by-label/local-unmount"
  and .spec.filesystems.localUnmount.mountpoint == "/mnt/local-unmount"
  and .spec.filesystems.localRescan.operation == "rescan"
  and .spec.filesystems.localRescan.device == "/dev/disk/by-label/local-rescan"
  and .spec.filesystems.localRescan.mountpoint == "/mnt/local-rescan"
  and .spec.filesystems.actionRescan.action == "rescan"
  and .spec.filesystems.actionUnmount.action == "unmount"
  and .spec.filesystems.destroyed.destroy == true
  and .spec.filesystems.destroyed.device == "/dev/disk/by-label/destroyed"
  and .spec.filesystems.targetSizeAlias.operation == "rescan"
  and .spec.filesystems.targetSizeAlias.targetSize == "200GiB"
  and .spec.filesystems.sizeAlias.operation == "rescan"
  and .spec.filesystems.sizeAlias.size == "150GiB"
  and .spec.filesystems.runTmpfs.device == "tmpfs"
  and .spec.filesystems.runTmpfs.fsType == "tmpfs"
  and .spec.filesystems.runTmpfs.mountpoint == "/run/disk-nix-tmp"
  and (.spec.filesystems.runTmpfs.options | index("size=64M") != null)
  and .spec.filesystems.bindCache.device == "/var/cache/disk-nix"
  and .spec.filesystems.bindCache.fsType == "none"
  and .spec.filesystems.bindCache.mountpoint == "/srv/disk-nix-cache"
  and (.spec.filesystems.bindCache.options | index("bind") != null)
  and .spec.filesystems.overlayScratch.device == "overlay"
  and .spec.filesystems.overlayScratch.fsType == "overlay"
  and .spec.filesystems.overlayScratch.mountpoint == "/srv/disk-nix-overlay"
  and (.spec.filesystems.overlayScratch.options | index("lowerdir=/nix/store") != null)
  and (.spec.filesystems.overlayScratch.options | index("upperdir=/var/lib/disk-nix/overlay/upper") != null)
  and (.spec.filesystems.overlayScratch.options | index("workdir=/var/lib/disk-nix/overlay/work") != null)
  and .spec.swaps.primary.device == "/dev/disk/by-label/swap"
  and .spec.swaps.primary.operation == "format"
  and .spec.swaps.primary.desiredSize == "8GiB"
  and .spec.swaps.primary.preserveData == false
  and .spec.swaps.primary.properties.label == "swap"
  and .spec.swaps.primary.properties."swap.uuid" == "01234567-89ab-cdef-0123-456789abcdef"
  and .spec.swaps.inventory.operation == "rescan"
  and .spec.swaps.inventory.device == "/dev/disk/by-label/swap-inventory"
  and .spec.swaps.targetSizeAlias.operation == "grow"
  and .spec.swaps.targetSizeAlias.targetSize == "12GiB"
  and .spec.swaps.sizeAlias.operation == "grow"
  and .spec.swaps.sizeAlias.size == "10GiB"
  and .spec.swaps.old.operation == "destroy"
  and .spec.swaps.actionOld.action == "destroy"
  and .spec.swaps.destroyed.destroy == true
  and .spec.swaps.destroyed.device == "/dev/disk/by-label/destroyed-swap"
  and .spec.zram.enable == true
  and .spec.zram.operation == "rescan"
  and .spec.zram.swapDevices == 2
  and .spec.zram.memoryPercent == 40
  and .spec.zram.memoryMax == 8589934592
  and .spec.zram.priority == 20
  and .spec.zram.algorithm == "zstd"
  and .spec.zram.properties."zram.compression-ratio-target" == "2.0"
  and .spec.luks.devices.cryptaction.action == "open"
  and .spec.swaps.old.device == "/dev/disk/by-label/old-swap"
  and .spec.luks.devices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
  and .spec.luks.devices.cryptroot.name == "cryptroot"
  and .spec.luks.devices.cryptroot.operation == "grow"
  and .spec.luks.devices.cryptroot.desiredSize == "100%"
  and .spec.luks.devices.cryptroot.properties.label == "cryptroot"
  and .spec.luks.devices.cryptroot.properties."luks.subsystem" == "nixos"
  and .spec.luks.devices.cryptTargetSize.operation == "grow"
  and .spec.luks.devices.cryptTargetSize.target == "cryptTargetSizeMapper"
  and .spec.luks.devices.cryptTargetSize.targetSize == "90%"
  and .spec.luks.devices.cryptSize.operation == "grow"
  and .spec.luks.devices.cryptSize.size == "80%"
  and .spec.luks.devices.cryptold.destroy == true
  and .spec.luks.devices.cryptold.device == "/dev/disk/by-partuuid/old-luks"
  and .spec.luks.devices.cryptarchive.operation == "open"
  and .spec.luks.devices.cryptarchive.device == "/dev/disk/by-id/archive-luks"
  and .spec.luks.devices.cryptclosed.operation == "close"
  and .spec.luks.devices.cryptclosed.device == "/dev/disk/by-id/closed-luks"
  and .spec.filesystems.shared.device == "nas.example.com:/srv/shared"
  and .spec.filesystems.shared.mountpoint == "/srv/shared"
  and .spec.filesystems.shared.fsType == "nfs4"
  and (.spec.filesystems.shared.options | index("x-systemd.automount") != null)
  and (.spec.filesystems | has("/srv/old") | not)
  and .spec.nfs.mounts.shared.source == "nas.example.com:/srv/shared"
  and .spec.nfs.mounts.shared.mountpoint == "/srv/shared"
  and .spec.nfs.mounts.shared.operation == "mount"
  and .spec.nfs.mounts.shared.metadata.server == "nas.example.com"
  and .spec.nfs.mounts.shared.metadata.export == "/srv/shared"
  and .spec.nfs.mounts."/srv/tuned".operation == "remount"
  and (.spec.nfs.mounts."/srv/tuned".options | index("ro") != null)
  and .spec.nfs.mounts."/srv/action".action == "remount"
  and .spec.nfs.mounts."/srv/inventory".operation == "rescan"
  and .spec.nfs.mounts."/srv/inventory".source == "nas.example.com:/srv/inventory"
  and .spec.nfs.mounts."/srv/old".source == "nas.example.com:/srv/old"
  and .spec.nfs.mounts."/srv/old".operation == "unmount"

''
