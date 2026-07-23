{
  pkgs,
  format,
  nixosModuleTest,
  zramTuningOnlyModuleTest,
  nixosModuleExecuteTest,
  nixosModuleHandoffAutoImportTest,
  nixosModuleBootModeTest,
  nixosModuleInstallModeTest,
  nixosModuleCollisionTest,
  nixosModuleDiskCollisionTest,
  nixosModulePartitionCollisionTest,
  nixosModuleLuksKeyslotCollisionTest,
  nixosModuleLuksTokenCollisionTest,
  nixosModuleBackingFileCollisionTest,
  nixosModuleBtrfsSubvolumeCollisionTest,
  nixosModuleBtrfsQgroupCollisionTest,
  nixosModuleDmMapCollisionTest,
  nixosModuleVdoVolumeCollisionTest,
  nixosModulePhysicalVolumeCollisionTest,
  nixosModuleLoopDeviceCollisionTest,
  nixosModuleMdRaidCollisionTest,
  nixosModuleMultipathMapCollisionTest,
  nixosModuleNvmeNamespaceCollisionTest,
  nixosModuleCacheCollisionTest,
  nixosModulePoolCollisionTest,
  nixosModuleDatasetCollisionTest,
  nixosModuleZvolCollisionTest,
  nixosModuleVolumeGroupCollisionTest,
  nixosModuleVolumeCollisionTest,
  nixosModuleThinPoolCollisionTest,
  nixosModuleLvmCacheCollisionTest,
  nixosModuleSnapshotCollisionTest,
  nixosModuleIscsiSessionCollisionTest,
  nixosModuleLunPathCollisionTest,
  ...
}:

{
  formatting = format.check;
  nixosModule = nixosModuleTest.config.system.build.toplevel;
  nixosModuleSpec =
    pkgs.runCommand "disk-nix-nixos-module-spec-check" { nativeBuildInputs = [ pkgs.jq ]; }
      ''
          spec=${nixosModuleTest.config.environment.etc."disk-nix/spec.json".source}
          jq -e '
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
            and .spec.iscsi.initiatorName == "iqn.2026-06.example:host"
            and (.spec.iscsi | has("discoverPortal") | not)
            and (.spec.iscsi.boot | has("discoverPortal") | not)
            and .spec.iscsi.boot.target == "iqn.2026-06.example:storage.root"
            and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".operation == "grow"
            and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
            and .spec.iscsi.sessions."iqn.2026-06.example:storage.alias".targetSize == "3TiB"
            and .spec.iscsi.sessions."iqn.2026-06.example:storage.login".operation == "login"
            and .spec.iscsi.sessions."iqn.2026-06.example:storage.logout".operation == "logout"
            and .spec.iscsi.sessions."iqn.2026-06.example:storage.rescan".operation == "rescan"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.root".portal == "192.0.2.10:3260"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.root".desiredSize == "2TiB"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.alias".targetSize == "3TiB"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.login".operation == "login"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.login".portal == "192.0.2.10:3260"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.logout".operation == "logout"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.logout".portal == "192.0.2.11:3260"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.rescan".operation == "rescan"
            and .spec.iscsiSessions."iqn.2026-06.example:storage.rescan".portal == "192.0.2.10:3260"
            and .spec.luns."iqn.2026-06.example:storage/root:0".lun == 0
            and .spec.luns."iqn.2026-06.example:storage/root:0".device == "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
            and (.spec.luns."iqn.2026-06.example:storage/root:0".devices | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
            and .spec.luns."iqn.2026-06.example:storage/new:2".operation == "attach"
            and .spec.luns."iqn.2026-06.example:storage/new:2".device == "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-2"
            and .spec.luns."iqn.2026-06.example:storage/old:3".operation == "detach"
            and (.spec.luns."iqn.2026-06.example:storage/old:3".devices | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") != null)
            and .spec.luns."iqn.2026-06.example:storage/rescan:4".operation == "rescan"
            and (.spec.luns."iqn.2026-06.example:storage/rescan:4".paths | index("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-4") != null)
            and .spec.nvmeNamespaces.rootNamespace.operation == "create"
            and .spec.nvmeNamespaces.rootNamespace.path == "/dev/nvme0"
            and .spec.nvmeNamespaces.rootNamespace.desiredSize == "100G"
            and .spec.nvmeNamespaces.rootNamespace.namespaceId == "4"
            and .spec.nvmeNamespaces.rootNamespace.controllers == "0x1"
            and .spec.nvmeNamespaces."/dev/nvme1".operation == "rescan"
            and .spec.nvmeNamespaces."/dev/nvme2".nsid == "7"
            and .spec.nvmeNamespaces."/dev/nvme2".controllerId == "0x2"
            and .spec.nvmeNamespaces."/dev/nvme3".namespaceId == "8"
            and .spec.nvmeNamespaces."/dev/nvme3".controller == "0x3"
            and .spec.exports.share.operation == "export"
            and .spec.exports.share.path == "/srv/share"
            and .spec.exports.share.client == "192.0.2.0/24"
            and .spec.exports.share.options == "rw,sync,no_subtree_check"
            and .spec.exports."/srv/inventory".operation == "rescan"
            and .spec.exports."/srv/old-share".operation == "unexport"
            and .spec.exports."/srv/old-share".client == "192.0.2.55"
            and .spec.partitions.root.operation == "grow"
            and .spec.partitions.root.device == "/dev/disk/by-id/nvme-root"
            and .spec.partitions.root.number == "2"
            and .spec.partitions.root.endOffset == "100%"
            and .spec.partitions.dataTable.operation == "rescan"
            and .spec.partitions.dataTable.device == "/dev/disk/by-id/nvme-data"
            and .spec.btrfsSubvolumes."/mnt/persist/@home".operation == "create"
            and .spec.btrfsSubvolumes."/mnt/persist/@home".path == "/mnt/persist/@home"
            and .spec.btrfsSubvolumes."/mnt/persist/@inventory".operation == "rescan"
            and .spec.btrfsSubvolumes."/mnt/persist/@inventory".path == "/mnt/persist/@inventory"
            and .spec.btrfsSubvolumes."/mnt/persist/@old-name".operation == "rename"
            and .spec.btrfsSubvolumes."/mnt/persist/@old-name".renameTo == "/mnt/persist/@new-name"
            and .spec.btrfsQgroups."0/257".target == "/mnt/persist"
            and .spec.btrfsQgroups."0/257".properties.limit == "25GiB"
            and .spec.btrfsQgroups."0/258".operation == "rescan"
            and .spec.btrfsQgroups."0/258".target == "/mnt/persist"
            and .spec.volumes.scratch.operation == "create"
            and .spec.volumes.scratch.target == "vg0/scratch"
            and .spec.volumes.scratch.desiredSize == "10GiB"
            and .spec.volumes."vg0/size-alias".size == "12GiB"
            and .spec.volumes."vg0/reporting".operation == "rescan"
            and .spec.datasets."tank/home".operation == "create"
            and .spec.datasets."tank/inventory".operation == "rescan"
            and .spec.vdoVolumes.archiveLifecycle.target == "archive"
            and .spec.vdoVolumes.archiveLifecycle.operation == "grow"
            and .spec.vdoVolumes.archiveLifecycle.desiredSize == "4TiB"
            and .spec.vdoVolumes.archiveLifecycle.physicalSize == "6TiB"
            and .spec.vdoVolumes.archiveLifecycle.properties.writePolicy == "sync"
            and .spec.vdoVolumes.archiveLifecycle.properties.compression == "enabled"
            and .spec.vdoVolumes.archiveLifecycle.properties.deduplication == "disabled"
            and .spec.vdoVolumes.warmArchive.target == "warm-archive"
            and .spec.vdoVolumes.warmArchive.operation == "start"
            and .spec.vdoVolumes.coldArchive.target == "cold-archive"
            and .spec.vdoVolumes.coldArchive.operation == "stop"
            and .spec.vdoVolumes.refreshArchive.target == "refresh-archive"
            and .spec.vdoVolumes.refreshArchive.operation == "rescan"
            and .spec.physicalVolumes.nvmePvGrow.operation == "grow"
            and .spec.physicalVolumes.nvmePvGrow.path == "/dev/disk/by-id/nvme-pv-grow"
            and .spec.physicalVolumes."/dev/disk/by-id/nvme-pv-refresh".operation == "rescan"
            and .spec.luksKeyslots."cryptroot:1".operation == "add-key"
            and .spec.luksKeyslots."cryptroot:1".device == "/dev/disk/by-id/root-luks"
            and .spec.luksKeyslots."cryptroot:1".keySlot == "1"
            and .spec.luksKeyslots."cryptroot:1".newKeyFile == "/run/keys/root-new"
            and .spec.luksKeyslots."cryptroot:2".operation == "remove-key"
            and .spec.luksKeyslots."cryptroot:2".device == "/dev/disk/by-id/root-luks"
            and .spec.luksKeyslots."cryptroot:2".keySlot == "2"
            and .spec.luksKeyslots."cryptroot:3"."key-slot" == "3"
            and .spec.luksKeyslots."cryptroot:3"."new-key-file" == "/run/keys/root-new-alias"
            and .spec.luksKeyslots."cryptroot:4".slot == "4"
            and .spec.luksTokens."cryptroot:0".operation == "import-token"
            and .spec.luksTokens."cryptroot:0".device == "/dev/disk/by-id/root-luks"
            and .spec.luksTokens."cryptroot:0".tokenId == "0"
            and .spec.luksTokens."cryptroot:0".tokenFile == "/run/keys/root-token.json"
            and .spec.luksTokens."cryptroot:1".operation == "remove-token"
            and .spec.luksTokens."cryptroot:1".device == "/dev/disk/by-id/root-luks"
            and .spec.luksTokens."cryptroot:1".tokenId == "1"
            and .spec.luksTokens."cryptroot:2".token == "2"
            and .spec.luksTokens."cryptroot:2"."token-file" == "/run/keys/root-token-alias.json"
            and .spec.luksTokens."cryptroot:3"."token-id" == "3"
            and .spec.zvols."tank/vm/root".operation == "grow"
            and .spec.zvols."tank/vm/root".desiredSize == "80GiB"
            and .spec.zvols."tank/vm/inventory".operation == "rescan"
            and .spec.thinPools.primaryPool.operation == "grow"
            and .spec.thinPools.primaryPool.path == "vg0/thinpool"
            and .spec.thinPools.primaryPool.desiredSize == "500GiB"
            and .spec.thinPools."vg0/newthin".operation == "create"
            and .spec.thinPools."vg0/newthin".desiredSize == "100GiB"
            and .spec.thinPools."vg0/reporting".operation == "rescan"
            and .spec.lvmSnapshots."vg0/root-snap".operation == "snapshot"
            and .spec.lvmSnapshots."vg0/root-snap".target == "vg0/root"
            and .spec.lvmSnapshots."vg0/root-snap".desiredSize == "20GiB"
            and .spec.lvmSnapshots."vg0/root-inspect".operation == "rescan"
            and .spec.lvmCaches."vg0/root".operation == "create"
            and .spec.lvmCaches."vg0/root".device == "vg0/root-cache"
            and .spec.lvmCaches."vg0/root".properties."lvm.cache-mode" == "writethrough"
            and .spec.lvmCaches."vg0/archive".operation == "rescan"
            and .spec.volumes."vg0/archive".operation == "deactivate"
            and .spec.loopDevices.rootImage.operation == "create"
            and .spec.loopDevices.rootImage.path == "/dev/loop7"
            and .spec.loopDevices.rootImage.device == "/var/lib/images/root.img"
            and .spec.loopDevices."/dev/loop10".operation == "rescan"
            and .spec.backingFiles."/var/lib/images/new.img".operation == "create"
            and .spec.backingFiles."/var/lib/images/new.img".desiredSize == "8GiB"
            and .spec.backingFiles."/var/lib/images/root.img".operation == "grow"
            and .spec.backingFiles."/var/lib/images/root.img".desiredSize == "16GiB"
            and .spec.backingFiles.inventoryImage.operation == "rescan"
            and .spec.backingFiles.inventoryImage.path == "/var/lib/images/inventory.img"
            and .spec.dmMaps.cryptroot.operation == "rescan"
            and .spec.dmMaps.cryptroot.target == "/dev/mapper/cryptroot"
            and .spec.dmMaps.cryptswap.operation == "rename"
            and .spec.dmMaps.cryptswap.target == "/dev/mapper/cryptswap"
            and .spec.dmMaps.cryptswap.renameTo == "cryptswap-retired"
            and .spec.dmMaps.oldmap.operation == "destroy"
            and .spec.dmMaps.oldmap.target == "/dev/mapper/oldmap"
            and .spec.mdRaids.root.target == "/dev/md/root"
            and .spec.mdRaids.root.raidLevel == "1"
            and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-a") != null)
            and (.spec.mdRaids.root.devices | index("/dev/disk/by-id/nvme-md-b") != null)
            and (.spec.mdRaids.root.addDevices | index("/dev/disk/by-id/nvme-md-spare") != null)
            and .spec.mdRaids.root.replaceDevices."/dev/disk/by-id/nvme-md-aging" == "/dev/disk/by-id/nvme-md-replacement"
            and .spec.mdRaids.existing.operation == "assemble"
            and .spec.mdRaids.existing.target == "/dev/md/existing"
            and (.spec.mdRaids.existing.devices | index("/dev/disk/by-id/existing-md-a") != null)
            and .spec.mdRaids.oldroot.operation == "stop"
            and .spec.mdRaids.oldroot.target == "/dev/md/oldroot"
            and .spec.mdRaids.inventory.operation == "rescan"
            and .spec.multipathMaps.mpatha.target == "mpatha"
            and (.spec.multipathMaps.mpatha.addDevices | index("/dev/sdb") != null)
            and .spec.multipathMaps.mpatha.replaceDevices."/dev/sdc" == "/dev/sdd"
            and .spec.multipathMaps.mpathb.operation == "rescan"
            and .spec.multipathMaps.mpathb.target == "mpathb"
            and .spec.multipathMaps.mpathOld.operation == "destroy"
            and .spec.multipathMaps.mpathOld.target == "mpath-old"
            and .spec.caches."tank/l2arc0".cacheSetUuid == "11111111-2222-3333-4444-555555555555"
            and (.spec.caches."/dev/bcache0".addDevices | index("cache-set-uuid") != null)
            and .spec.caches."/dev/bcache0".cacheSetUuid == "cache-set-uuid"
            and .spec.caches."/dev/bcache0".operation == "rescan"
            and .spec.caches."/dev/bcache0".properties."bcache.cache-mode" == "writethrough"
            and .spec.caches."/dev/bcache0".properties."bcache.set-journal-delay-ms" == "100"
            and .spec.pools.vault.operation == "import"
            and .spec.pools.vault.readOnly == true
            and .spec.pools.archiveImport.readonly == true
            and .spec.pools.moveme.operation == "export"
            and .spec.volumeGroups.importvg.operation == "import"
            and .spec.volumeGroups.exportvg.operation == "export"
            and .spec.volumeGroups.activevg.operation == "activate"
            and .spec.volumeGroups.refreshvg.operation == "rescan"
            and .spec.volumeGroups.actionvg.action == "rescan"
            and .spec.datasets."tank/home-review".operation == "promote"
            and .spec.datasets."tank/legacy-alias".renameTarget == "tank/legacy-alias-staged"
            and .spec.datasets."tank/legacy-short".newName == "tank/legacy-short-staged"
            and .spec.snapshots."tank/home@before-upgrade".target == "tank/home"
            and .spec.snapshots."tank/home@before-upgrade".hold == "disk-nix-retain"
            and .spec.snapshots."tank/home@before-upgrade".rollback == true
        and .spec.snapshots."tank/home@before-upgrade".cloneTo == "tank/home-review"
        and .spec.snapshots."tank/home@before-upgrade".renameTo == "tank/home@before-upgrade-retained"
        and .spec.snapshots."tank/home@before-upgrade".recursiveRollback == true
        and .spec.snapshots."tank/home@clone-only".operation == "clone"
        and .spec.snapshots."tank/home@clone-only".cloneTo == "tank/home-clone"
        and .spec.snapshots."tank/home@action-rescan".action == "rescan"
        and .spec.snapshots.aliases.operation == "clone"
        and .spec.snapshots.aliases."snapshot-path" == "tank/home@alias-source"
        and .spec.snapshots.aliases.cloneTarget == "tank/home-alias-clone"
        and .spec.snapshots.aliases.clone == "tank/home-short-clone"
        and .spec.snapshots.aliases.renameTarget == "tank/home@alias-retained"
        and .spec.snapshots.aliases.newName == "tank/home@alias-new"
        and .spec.snapshots.aliases.recursive == true
        and .spec.snapshots.aliases."zfs.rollbackRecursive" == true
        and .spec.snapshots.aliases.readonly == true
        and .spec.datasets."tank/legacy".renameTo == "tank/legacy-staged"
            and .spec.snapshots."tank/home@old".releaseHold == "old-retention"
            and .spec.snapshots."/mnt/persist/@home-before-upgrade".target == "/mnt/persist/@home"
            and .spec.snapshots."/mnt/persist/@home-before-upgrade".readOnly == true
            and .spec.snapshots."/mnt/persist/@home-before-clone".target == "/mnt/persist/@home"
            and .spec.snapshots."/mnt/persist/@home-before-clone".cloneTo == "/mnt/persist/@home-review"
            and .spec.snapshots."/mnt/persist/@home-before-clone".readOnly == true
            and .spec.snapshots."tank/home@inventory".operation == "rescan"
            and .spec.snapshots."/mnt/persist/@home-inventory".operation == "rescan"
            and .spec.snapshots."/mnt/persist/@home-inventory".readOnly == true
            and .spec.snapshots."home-before-friendly".operation == "rescan"
            and .spec.snapshots."home-before-friendly".target == "/mnt/persist/@home"
            and .spec.snapshots."home-before-friendly".snapshotPath == "/mnt/persist/@home-before-friendly"
            and .apply.mode == "activation"
            and .apply.allowGrow == true
            and .apply.allowOffline == false
            and .apply.probeCurrent == true
            and .apply.allowDeviceReplacement == true
            and .apply.allowRebalance == true
            and .apply.allowPotentialDataLoss == false
            and .apply.requireBackup == false
            and .apply.backupVerified == false
            and .apply.requireConfirmation == false
            and .apply.confirmation == false
            and .apply.requireConfirmationFile == "/run/disk-nix/confirm"
            and .apply.failOnBlocked == false
            and .apply.scriptOut == "/run/disk-nix/apply.sh"
            and .apply.reportOut == "/run/disk-nix/apply-report.json"
            and .apply.receiptOut == "/run/disk-nix/apply-receipt.json"
          ' "$spec"
          applyScript='${nixosModuleTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
          grep -- 'validate' "$applyScript"
          grep -- '--probe-current' "$applyScript"
          grep -- '--script-out' "$applyScript"
          grep -- '/run/disk-nix/apply.sh' "$applyScript"
          grep -- '--report-out' "$applyScript"
          grep -- '/run/disk-nix/apply-report.json' "$applyScript"
          grep -- '--receipt-out' "$applyScript"
          grep -- '/run/disk-nix/apply-receipt.json' "$applyScript"
          printf '%s\n' ${pkgs.lib.escapeShellArgs (map toString nixosModuleTest.config.systemd.services.disk-nix-plan.path)} > service-paths
          grep -- 'bcachefs-tools-' service-paths
          grep -- 'btrfs-progs-' service-paths
          grep -- 'dosfstools-' service-paths
          grep -- 'exfatprogs-' service-paths
          grep -- 'f2fs-tools-' service-paths
          grep -- 'lvm2-' service-paths
          grep -- 'lsscsi-' service-paths
          grep -- 'ntfs3g-' service-paths
          grep -- 'open-iscsi-' service-paths
          grep -- 'smartmontools-' service-paths
          grep -- 'targetcli-fb-' service-paths
          grep -- 'tgt-' service-paths
          grep -- 'util-linux-' service-paths
          grep -- 'zfs-user-' service-paths
          swapDevices=${
            pkgs.lib.escapeShellArg (
              builtins.toJSON (map (swap: { inherit (swap) device; }) nixosModuleTest.config.swapDevices)
            )
          }
          printf '%s\n' "$swapDevices" > swap-devices
          jq -e '
            length == 4
            and any(.[]; .device == "/dev/disk/by-label/swap")
            and any(.[]; .device == "/dev/disk/by-label/swap-inventory")
            and any(.[]; .device == "/dev/disk/by-label/swap-target-size")
            and any(.[]; .device == "/dev/disk/by-label/swap-size")
            and all(.[]; .device != "/dev/disk/by-label/action-old-swap")
            and all(.[]; .device != "/dev/disk/by-label/destroyed-swap")
          ' swap-devices
          luksDevices=${
            pkgs.lib.escapeShellArg (
              builtins.toJSON (
                pkgs.lib.mapAttrs (_: luks: {
                  inherit (luks) device;
                }) nixosModuleTest.config.boot.initrd.luks.devices
              )
            )
          }
          printf '%s\n' "$luksDevices" > luks-devices
          jq -e '
            has("cryptroot")
            and .cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
            and has("cryptTargetSizeMapper")
            and .cryptTargetSizeMapper.device == "/dev/disk/by-id/target-size-luks"
            and (has("cryptTargetSize") | not)
            and has("cryptSize")
            and .cryptSize.device == "/dev/disk/by-id/size-luks"
            and has("cryptarchive")
            and .cryptarchive.device == "/dev/disk/by-id/archive-luks"
            and (has("cryptold") | not)
            and (has("cryptclosed") | not)
          ' luks-devices
          fileSystems=${
            pkgs.lib.escapeShellArg (
              builtins.toJSON (
                pkgs.lib.mapAttrs (_: fs: {
                  inherit (fs) device fsType;
                }) nixosModuleTest.config.fileSystems
              )
            )
          }
          printf '%s\n' "$fileSystems" > file-systems
          jq -e '
            has("/srv/shared")
            and ."/srv/shared".device == "nas.example.com:/srv/shared"
            and has("/srv/tuned")
            and ."/srv/tuned".device == "nas.example.com:/srv/tuned"
            and ."/srv/tuned".fsType == "nfs4"
            and has("/mnt/local-mount")
            and ."/mnt/local-mount".device == "/dev/disk/by-label/local-mount"
            and ."/mnt/local-mount".fsType == "xfs"
            and (has("/mnt/local-unmount") | not)
            and has("/mnt/local-rescan")
            and ."/mnt/local-rescan".device == "/dev/disk/by-label/local-rescan"
            and ."/mnt/local-rescan".fsType == "xfs"
            and has("/mnt/action-rescan")
            and ."/mnt/action-rescan".device == "/dev/disk/by-label/action-rescan"
            and ."/mnt/action-rescan".fsType == "xfs"
            and (has("/mnt/action-unmount") | not)
            and (has("/mnt/teardown-only") | not)
            and (has("/mnt/destroyed") | not)
            and has("/srv/action")
            and ."/srv/action".device == "nas.example.com:/srv/action"
            and ."/srv/action".fsType == "nfs4"
            and has("/run/disk-nix-tmp")
            and ."/run/disk-nix-tmp".device == "tmpfs"
            and ."/run/disk-nix-tmp".fsType == "tmpfs"
            and has("/srv/disk-nix-cache")
            and ."/srv/disk-nix-cache".device == "/var/cache/disk-nix"
            and ."/srv/disk-nix-cache".fsType == "none"
            and has("/srv/disk-nix-overlay")
            and ."/srv/disk-nix-overlay".device == "overlay"
            and ."/srv/disk-nix-overlay".fsType == "overlay"
            and (has("/srv/old") | not)
          ' file-systems
          supportedFilesystems=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleTest.config.boot.supportedFilesystems)}
          printf '%s\n' "$supportedFilesystems" > supported-filesystems
          jq -e '
            .xfs == true
            and .btrfs == true
            and .bcachefs == true
            and .f2fs == true
            and .tmpfs == true
            and .overlay == true
            and .nfs4 == true
            and .zfs == true
            and (has("jfs") | not)
          ' supported-filesystems
          nativeStorage=${
            pkgs.lib.escapeShellArg (
              builtins.toJSON {
                lvm = nixosModuleTest.config.services.lvm.enable;
                lvmInitrd = nixosModuleTest.config.boot.initrd.services.lvm.enable;
                lvmThin = nixosModuleTest.config.services.lvm.boot.thin.enable;
                lvmVdo = nixosModuleTest.config.services.lvm.boot.vdo.enable;
                swraid = nixosModuleTest.config.boot.swraid.enable;
                mdadmConf = nixosModuleTest.config.boot.swraid.mdadmConf;
                multipath = nixosModuleTest.config.services.multipath.enable;
                zfsExtraPools = nixosModuleTest.config.boot.zfs.extraPools;
                zfsForceImportRoot = nixosModuleTest.config.boot.zfs.forceImportRoot;
                bcache = nixosModuleTest.config.boot.bcache.enable;
                bcacheInitrd = nixosModuleTest.config.boot.initrd.services.bcache.enable;
                zram = nixosModuleTest.config.zramSwap.enable;
                zramSwapDevices = nixosModuleTest.config.zramSwap.swapDevices;
                zramMemoryPercent = nixosModuleTest.config.zramSwap.memoryPercent;
                zramMemoryMax = nixosModuleTest.config.zramSwap.memoryMax;
                zramPriority = nixosModuleTest.config.zramSwap.priority;
                zramAlgorithm = nixosModuleTest.config.zramSwap.algorithm;
                openIscsiDiscoverPortal = nixosModuleTest.config.services.openiscsi.discoverPortal;
                bootIscsiDiscoverPortal = nixosModuleTest.config.boot.iscsi-initiator.discoverPortal;
              }
            )
          }
          printf '%s\n' "$nativeStorage" > native-storage
          jq -e '
            .lvm == true
            and .lvmInitrd == true
            and .lvmThin == true
            and .lvmVdo == true
            and .swraid == true
            and (.mdadmConf | test("^PROGRAM .*/bin/true$"))
            and .multipath == true
            and (.zfsExtraPools | index("tank") != null)
            and (.zfsExtraPools | index("mnt") == null)
            and .zfsForceImportRoot == false
            and .bcache == true
            and .bcacheInitrd == true
            and .zram == true
            and .zramSwapDevices == 2
            and .zramMemoryPercent == 40
            and .zramMemoryMax == 8589934592
            and .zramPriority == 20
            and .zramAlgorithm == "zstd"
            and .openIscsiDiscoverPortal == "192.0.2.10:3260"
            and .bootIscsiDiscoverPortal == "192.0.2.10:3260"
          ' native-storage
          steadyState=${
            pkgs.lib.escapeShellArg (
              builtins.readFile nixosModuleTest.config.environment.etc."disk-nix/steady-state.json".source
            )
          }
          printf '%s\n' "$steadyState" > steady-state
          jq -e '
            .version == 1
            and .fileSystems."/srv/tuned".device == "nas.example.com:/srv/tuned"
            and .fileSystems."/srv/tuned".fsType == "nfs4"
            and .fileSystems."/mnt/local-mount".device == "/dev/disk/by-label/local-mount"
            and .fileSystems."/mnt/local-mount".fsType == "xfs"
            and (.fileSystems | has("/mnt/local-unmount") | not)
            and (.fileSystems | has("/srv/old") | not)
            and (.swapDevices | length == 4)
            and (.swapDevices | any(.device == "/dev/disk/by-label/swap"))
            and (.swapDevices | all(.device != "/dev/disk/by-label/destroyed-swap"))
            and .luksDevices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
            and (.luksDevices | has("cryptclosed") | not)
            and .zramSwap.enable == true
            and .zramSwap.swapDevices == 2
            and .zramSwap.memoryMax == 8589934592
            and (.supportedFilesystems | index("xfs") != null)
            and (.supportedFilesystems | index("nfs4") != null)
            and (.supportedFilesystems | index("zfs") != null)
            and (.nfsExports | index("/srv/share 192.0.2.0/24(rw,sync,no_subtree_check)") != null)
            and (.nfsExports | all(. | contains("/srv/old-share") | not))
            and (.storageIdentities.filesystemMountpoints | index("/mnt/local-mount") != null)
            and (.storageIdentities.filesystemMountpoints | index("/mnt/local-unmount") == null)
            and (.storageIdentities.swapDevices | index("/dev/disk/by-label/swap") != null)
            and (.storageIdentities.swapDevices | index("/dev/disk/by-label/destroyed-swap") == null)
            and (.storageIdentities.physicalVolumes | index("/dev/disk/by-id/nvme-pv-grow") != null)
            and (.storageIdentities.volumes | index("vg0/scratch") != null)
            and (.storageIdentities.volumes | index("vg0/archive") == null)
            and (.storageIdentities.thinPools | index("vg0/thinpool") != null)
            and (.storageIdentities.lvmCaches | index("vg0/root") != null)
            and (.storageIdentities.vdoVolumes | index("archive") != null)
            and (.storageIdentities.vdoVolumes | index("cold-archive") == null)
            and (.storageIdentities.luksKeyslots | index("/dev/disk/by-id/root-luks keyslot 1") != null)
            and (.storageIdentities.luksTokens | index("/dev/disk/by-id/root-luks token 0") != null)
            and (.storageIdentities.backingFiles | index("/var/lib/images/root.img") != null)
            and (.storageIdentities.loopDevices | index("/dev/loop7") != null)
            and (.storageIdentities.dmMaps | index("/dev/mapper/cryptroot") != null)
            and (.storageIdentities.mdRaids | index("/dev/md/root") != null)
            and (.storageIdentities.mdRaids | index("/dev/md/oldroot") == null)
            and (.storageIdentities.multipathMaps | index("mpatha") != null)
            and (.storageIdentities.pools | index("vault") != null)
            and (.storageIdentities.pools | index("moveme") == null)
            and (.storageIdentities.datasets | index("tank/home") != null)
            and (.storageIdentities.zvols | index("tank/vm/root") != null)
            and (.storageIdentities.btrfsSubvolumes | index("/mnt/persist/@home") != null)
            and (.storageIdentities.btrfsQgroups | index("0/257 /mnt/persist") != null)
            and (.storageIdentities.snapshots | index("tank/home@before-upgrade") != null)
            and (.storageIdentities.caches | index("tank/l2arc0") != null)
            and (.storageIdentities.nvmeNamespaces | index("/dev/nvme0 nsid 4") != null)
            and (.networkStorage.iscsiSessionTargets | index("iqn.2026-06.example:storage.root") != null)
            and (.networkStorage.iscsiSessionTargets | index("iqn.2026-06.example:storage.logout") == null)
            and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
            and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
            and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") == null)
            and (.networkStorage.nfsExportSelectors | index("/srv/share 192.0.2.0/24") != null)
            and (.networkStorage.nfsExportSelectors | index("/srv/old-share 192.0.2.55") == null)
            and .lifecycleManaged.filesystems."/mnt/local-mount".operation == "mount"
            and .lifecycleManaged.filesystems."/mnt/local-mount".identity == "/mnt/local-mount"
            and (.lifecycleManaged.filesystems | has("/mnt/local-unmount") | not)
            and .lifecycleManaged.swapDevices."/dev/disk/by-label/swap".operation == "format"
            and .lifecycleManaged.swapDevices."/dev/disk/by-label/swap".desiredSize == "8GiB"
            and (.lifecycleManaged.swapDevices | has("/dev/disk/by-label/destroyed-swap") | not)
            and .lifecycleManaged.physicalVolumes."/dev/disk/by-id/nvme-pv-grow".operation == "grow"
            and .lifecycleManaged.volumes."vg0/scratch".operation == "create"
            and .lifecycleManaged.volumes."vg0/scratch".desiredSize == "10GiB"
            and (.lifecycleManaged.volumes | has("vg0/archive") | not)
            and .lifecycleManaged.thinPools."vg0/thinpool".operation == "grow"
            and .lifecycleManaged.lvmCaches."vg0/root".operation == "create"
            and .lifecycleManaged.vdoVolumes.archive.operation == "grow"
            and .lifecycleManaged.vdoVolumes.archive.desiredSize == "4TiB"
            and (.lifecycleManaged.vdoVolumes | has("cold-archive") | not)
            and .lifecycleManaged.luksKeyslots."/dev/disk/by-id/root-luks keyslot 1".operation == "add-key"
            and (.lifecycleManaged.luksKeyslots | has("/dev/disk/by-id/root-luks keyslot 2") | not)
            and .lifecycleManaged.btrfsQgroups."0/257 /mnt/persist".identity == "0/257 /mnt/persist"
            and .lifecycleManaged.snapshots."tank/home@before-upgrade".operation == "create"
            and .lifecycleManaged.luns."/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0".operation == "grow"
            and (.lifecycleManaged.luns | has("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") | not)
            and .lifecycleManaged.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
            and (.lifecycleManaged.iscsiSessions | has("iqn.2026-06.example:storage.logout") | not)
            and .lifecycleManaged.nfsExports."/srv/share 192.0.2.0/24".operation == "export"
            and (.lifecycleManaged.nfsExports | has("/srv/old-share 192.0.2.55") | not)
            and .iscsi.openiscsi.enable == true
            and .iscsi.openiscsi.discoverPortal == "192.0.2.10:3260"
            and .iscsi.bootInitiator.enable == true
            and .iscsi.bootInitiator.discoverPortal == "192.0.2.10:3260"
            and (.declarativeHandoff.fileSystems | index("/mnt/local-mount") != null)
            and (.declarativeHandoff.fileSystems | index("/mnt/local-unmount") == null)
            and (.declarativeHandoff.swapDevices | index("/dev/disk/by-label/swap") != null)
            and (.declarativeHandoff.swapDevices | index("/dev/disk/by-label/destroyed-swap") == null)
            and (.declarativeHandoff.luksDevices | index("cryptroot") != null)
            and (.declarativeHandoff.luksDevices | index("cryptclosed") == null)
            and (.declarativeHandoff.nfsExports | index("/srv/share 192.0.2.0/24") != null)
            and (.declarativeHandoff.nfsExports | index("/srv/old-share 192.0.2.55") == null)
            and (.declarativeHandoff.iscsiSessions | index("iqn.2026-06.example:storage.root") != null)
            and (.declarativeHandoff.iscsiSessions | index("iqn.2026-06.example:storage.logout") == null)
            and .declarativeHandoff.iscsiBoot == true
            and .declarativeHandoff.nixModule == "/etc/disk-nix/declarative-handoff.nix"
            and .declarativeHandoff.importPatch == "/etc/disk-nix/declarative-handoff-import.patch"
            and .declarativeHandoff.autoImport.enabled == false
            and .declarativeHandoff.autoImport.configurationPath == "/etc/nixos/configuration.nix"
            and .declarativeHandoff.autoImport.backupDirectory == "/var/backups/disk-nix"
            and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/spec.json") != null)
            and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/steady-state.json") != null)
            and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/declarative-handoff.nix") != null)
            and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/declarative-handoff-import.patch") != null)
            and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply.sh") != null)
            and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply-report.json") != null)
            and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply-receipt.json") != null)
            and .nativeServices.lvm == true
            and .nativeServices.lvmThin == true
            and .nativeServices.lvmVdo == true
            and .nativeServices.mdraid == true
            and .nativeServices.multipath == true
            and .nativeServices.bcache == true
            and .nativeServices.nfsServer == true
            and (.nativeServices.zfsExtraPools | index("tank") != null)
            and (.nativeServices.zfsExtraPools | index("moveme") == null)
            and (.nativeServices.zfsExtraPools | index("mnt") == null)
          ' steady-state
          handoffNix=${nixosModuleTest.config.environment.etc."disk-nix/declarative-handoff.nix".source}
          grep -F -- 'Generated by services.disk-nix' "$handoffNix"
          grep -F -- 'This file is not imported by default' "$handoffNix"
          grep -F -- 'fileSystems = {' "$handoffNix"
          grep -F -- '"/mnt/local-mount" = {' "$handoffNix"
          grep -F -- 'swapDevices = [' "$handoffNix"
          grep -F -- 'zramSwap = {' "$handoffNix"
          grep -F -- 'luks = {' "$handoffNix"
          grep -F -- 'devices = {' "$handoffNix"
          grep -F -- 'supportedFilesystems = [' "$handoffNix"
          grep -F -- 'openiscsi = {' "$handoffNix"
          grep -F -- 'extraPools = [' "$handoffNix"
          handoffPatch=${
            nixosModuleTest.config.environment.etc."disk-nix/declarative-handoff-import.patch".source
          }
          grep -F -- 'Generated by services.disk-nix' "$handoffPatch"
          grep -F -- 'This patch is intentionally not applied by default' "$handoffPatch"
          grep -F -- 'imports = [' "$handoffPatch"
          grep -F -- '/etc/disk-nix/declarative-handoff.nix' "$handoffPatch"
          printf '%s\n' ${pkgs.lib.escapeShellArg nixosModuleTest.config.services.nfs.server.exports} > nfs-exports
          grep -- '/srv/share 192.0.2.0/24(rw,sync,no_subtree_check)' nfs-exports
          ! grep -- '/srv/old-share' nfs-exports
          tuningOnlySpec=${zramTuningOnlyModuleTest.config.environment.etc."disk-nix/spec.json".source}
          jq -e '
            .spec.zram.swapDevices == 3
            and .spec.zram.memoryPercent == 35
            and .spec.zram.priority == 15
            and .spec.zram.algorithm == "lz4"
            and .spec.zram.preserveData == false
            and ((.spec.zram.enable // false) == false)
          ' "$tuningOnlySpec"
          tuningOnlyNative=${
            pkgs.lib.escapeShellArg (
              builtins.toJSON {
                zram = zramTuningOnlyModuleTest.config.zramSwap.enable;
              }
            )
          }
          printf '%s\n' "$tuningOnlyNative" > tuning-only-native-storage
          jq -e '.zram == false' tuning-only-native-storage
          touch "$out"
      '';
  nixosModuleExecute =
    pkgs.runCommand "disk-nix-nixos-module-execute-check" { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        spec=${nixosModuleExecuteTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '
          .apply.mode == "activation"
          and .apply.failOnBlocked == true
          and .apply.probeCurrent == true
          and has("apply")
          and (.apply | has("execute") | not)
        ' "$spec"
        applyScript='${nixosModuleExecuteTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$applyScript"
        grep -- '--execute' "$applyScript"
        grep -- '--probe-current' "$applyScript"
        grep -- '--script-out' "$applyScript"
        grep -- '/run/disk-nix/execute.sh' "$applyScript"
        grep -- '--report-out' "$applyScript"
        grep -- '/run/disk-nix/execute-report.json' "$applyScript"
        grep -- '--receipt-out' "$applyScript"
        grep -- '/run/disk-nix/execute-receipt.json' "$applyScript"
        touch "$out"
      '';
  nixosModuleHandoffAutoImport =
    pkgs.runCommand "disk-nix-nixos-module-handoff-auto-import-check"
      { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        spec=${nixosModuleHandoffAutoImportTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '
          .apply.mode == "activation"
          and .apply.failOnBlocked == true
          and (.apply | has("execute") | not)
          and (.apply | has("declarativeHandoff") | not)
        ' "$spec"
        steadyState=${
          pkgs.lib.escapeShellArg (
            builtins.readFile
              nixosModuleHandoffAutoImportTest.config.environment.etc."disk-nix/steady-state.json".source
          )
        }
        printf '%s\n' "$steadyState" > steady-state
        jq -e '
          .declarativeHandoff.autoImport.enabled == true
          and .declarativeHandoff.autoImport.configurationPath == "/etc/nixos/storage.nix"
          and .declarativeHandoff.autoImport.backupDirectory == "/var/backups/disk-nix-handoff"
        ' steady-state
        applyScript='${nixosModuleHandoffAutoImportTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$applyScript"
        grep -- '--execute' "$applyScript"
        grep -F -- 'config_path=/etc/nixos/storage.nix' "$applyScript"
        grep -F -- 'backup_dir=/var/backups/disk-nix-handoff' "$applyScript"
        grep -F -- 'handoff_module=/etc/disk-nix/declarative-handoff.nix' "$applyScript"
        grep -F -- 'import_patch=/etc/disk-nix/declarative-handoff-import.patch' "$applyScript"
        grep -F -- 'grep -F -q "$handoff_module" "$config_path"' "$applyScript"
        grep -F -- 'cp --preserve=mode,ownership,timestamps "$config_path" "$backup_path"' "$applyScript"
        grep -F -- 'patch --forward --backup --input="$import_patch" "$config_path"' "$applyScript"
        touch "$out"
      '';
  nixosModuleApplyModes =
    pkgs.runCommand "disk-nix-nixos-module-apply-modes-check" { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        bootWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.warnings)}
        installWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.warnings)}
        ! printf '%s\n' "$bootWarnings" | grep -- 'apply.mode = \\"boot\\" is reserved'
        ! printf '%s\n' "$installWarnings" | grep -- 'apply.mode = \\"install\\" is reserved'
        bootSpec=${nixosModuleBootModeTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '.apply.mode == "boot"' "$bootSpec"
        bootScript='${nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$bootScript"
        bootWantedBy=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.wantedBy)}
        printf '%s\n' "$bootWantedBy" | jq -e 'index("multi-user.target") != null'
        bootWants=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.wants)}
        printf '%s\n' "$bootWants" | jq -e 'index("systemd-udev-settle.service") != null'
        bootAfter=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.after)}
        printf '%s\n' "$bootAfter" | jq -e 'index("local-fs.target") != null and index("systemd-udev-settle.service") != null'
        bootBefore=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.before)}
        printf '%s\n' "$bootBefore" | jq -e 'index("multi-user.target") != null'
        installSpec=${nixosModuleInstallModeTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '.apply.mode == "install"' "$installSpec"
        installScript='${nixosModuleInstallModeTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$installScript"
        installWantedBy=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.systemd.services.disk-nix-plan.wantedBy)}
        printf '%s\n' "$installWantedBy" | jq -e 'index("multi-user.target") != null'
        touch "$out"
      '';
  nixosModuleAssertions = pkgs.runCommand "disk-nix-nixos-module-assertions-check" { } ''
    collisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleCollisionTest.config.system.build.toplevel).success))}
    diskCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDiskCollisionTest.config.system.build.toplevel).success))}
    partitionCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePartitionCollisionTest.config.system.build.toplevel).success))}
    luksKeyslotCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLuksKeyslotCollisionTest.config.system.build.toplevel).success))}
    luksTokenCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLuksTokenCollisionTest.config.system.build.toplevel).success))}
    backingFileCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBackingFileCollisionTest.config.system.build.toplevel).success))}
    btrfsSubvolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBtrfsSubvolumeCollisionTest.config.system.build.toplevel).success))}
    btrfsQgroupCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBtrfsQgroupCollisionTest.config.system.build.toplevel).success))}
    dmMapCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDmMapCollisionTest.config.system.build.toplevel).success))}
    vdoVolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVdoVolumeCollisionTest.config.system.build.toplevel).success))}
    physicalVolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePhysicalVolumeCollisionTest.config.system.build.toplevel).success))}
    loopDeviceCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLoopDeviceCollisionTest.config.system.build.toplevel).success))}
    mdRaidCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleMdRaidCollisionTest.config.system.build.toplevel).success))}
    multipathMapCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleMultipathMapCollisionTest.config.system.build.toplevel).success))}
    nvmeNamespaceCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleNvmeNamespaceCollisionTest.config.system.build.toplevel).success))}
    cacheCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleCacheCollisionTest.config.system.build.toplevel).success))}
    poolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePoolCollisionTest.config.system.build.toplevel).success))}
    datasetCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDatasetCollisionTest.config.system.build.toplevel).success))}
    zvolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleZvolCollisionTest.config.system.build.toplevel).success))}
    volumeGroupCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVolumeGroupCollisionTest.config.system.build.toplevel).success))}
    volumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVolumeCollisionTest.config.system.build.toplevel).success))}
    thinPoolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleThinPoolCollisionTest.config.system.build.toplevel).success))}
    lvmCacheCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLvmCacheCollisionTest.config.system.build.toplevel).success))}
    snapshotCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleSnapshotCollisionTest.config.system.build.toplevel).success))}
    iscsiSessionCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleIscsiSessionCollisionTest.config.system.build.toplevel).success))}
    lunPathCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLunPathCollisionTest.config.system.build.toplevel).success))}
    test "$collisionSuccess" = false
    test "$diskCollisionSuccess" = false
    test "$partitionCollisionSuccess" = false
    test "$luksKeyslotCollisionSuccess" = false
    test "$luksTokenCollisionSuccess" = false
    test "$backingFileCollisionSuccess" = false
    test "$btrfsSubvolumeCollisionSuccess" = false
    test "$btrfsQgroupCollisionSuccess" = false
    test "$dmMapCollisionSuccess" = false
    test "$vdoVolumeCollisionSuccess" = false
    test "$physicalVolumeCollisionSuccess" = false
    test "$loopDeviceCollisionSuccess" = false
    test "$mdRaidCollisionSuccess" = false
    test "$multipathMapCollisionSuccess" = false
    test "$nvmeNamespaceCollisionSuccess" = false
    test "$cacheCollisionSuccess" = false
    test "$poolCollisionSuccess" = false
    test "$datasetCollisionSuccess" = false
    test "$zvolCollisionSuccess" = false
    test "$volumeGroupCollisionSuccess" = false
    test "$volumeCollisionSuccess" = false
    test "$thinPoolCollisionSuccess" = false
    test "$lvmCacheCollisionSuccess" = false
    test "$snapshotCollisionSuccess" = false
    test "$iscsiSessionCollisionSuccess" = false
    test "$lunPathCollisionSuccess" = false
    touch "$out"
  '';
}
