# NixOS module reference

This page is the structured reference for typed `services.disk-nix` options.

Use [NixOS module](nixos-module.md) for the quick start and apply-policy
overview.

## Full Example

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;
    apply = {
      mode = "manual";
      allowGrow = true;
      allowShrink = false;
      allowPotentialDataLoss = false;
      allowDestructive = false;
      probeCurrent = true;
      requireBackup = false;
      requireConfirmation = false;
    };
    luks.devices.cryptroot = {
      device = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
      allowDiscards = true;
    };
    filesystems.root = {
      device = "/dev/disk/by-label/nixos-root";
      fsType = "xfs";
      mountpoint = "/";
      neededForBoot = true;
      resizePolicy = "grow-only";
      desiredSize = "100%";
    };
    swaps.primary = {
      device = "/dev/disk/by-label/swap";
      priority = 5;
    };
    zram = {
      enable = true;
      operation = "rescan";
      swapDevices = 1;
      memoryPercent = 50;
      priority = 20;
      algorithm = "zstd";
    };
  };
}
```

## Generated Files

| File | Purpose |
| --- | --- |
| `/etc/disk-nix/spec.json` | Normalized planner spec with top-level `version = 1`. |
| `/etc/disk-nix/steady-state.json` | Native NixOS state derived from active declarations. |
| `/etc/disk-nix/declarative-handoff.nix` | Reviewable Nix module snippet for post-mutation steady state. |
| `/etc/disk-nix/declarative-handoff-import.patch` | Optional patch skeleton for importing the handoff module. |

## Native NixOS Surfaces

| disk-nix declaration | Native NixOS surface |
| --- | --- |
| Active filesystems and NFS mounts | `fileSystems` |
| Active swap devices | `swapDevices` |
| zram settings | `zramSwap` |
| Active LUKS mappers | `boot.initrd.luks.devices` |
| Filesystem support | `boot.supportedFilesystems` |
| LVM and thin/cache support | `services.lvm`, `boot.initrd.services.lvm` |
| MD RAID | `boot.swraid` |
| Multipath | `services.multipath` |
| ZFS pools | `boot.zfs.extraPools` |
| bcache | `boot.bcache`, `boot.initrd.services.bcache` |
| VDO-backed LVM | `services.lvm.boot.vdo.enable` |
| iSCSI | `services.openiscsi`, `boot.iscsi-initiator` |
| NFS exports | `services.nfs.server.exports` |

Raw `spec` remains available for domains whose typed NixOS options have not
been implemented.

## Steady-State JSON

| Section | Records |
| --- | --- |
| Native storage | `fileSystems`, `swapDevices`, zram, initrd LUKS, supported filesystems. |
| Network storage | NFS export lines, iSCSI service settings, boot initiator settings. |
| Active identities | Mounts, swaps, LUKS, LVM, VDO, dm, MD, multipath, ZFS, Btrfs, caches, loops, backing files, NVMe, snapshots. |
| Network identities | iSCSI targets, host-side LUN paths, NFS export path/client selectors. |
| Native services | LVM, thin, VDO, MD RAID, multipath, ZFS, bcache, NFS. |
| `lifecycleManaged` | Active disk-nix resources that still need post-mutation review. |
| `declarativeHandoff` | Native NixOS surfaces and generated artifact paths to review. |

## Handoff Auto-Import

| Option | Behavior |
| --- | --- |
| `services.disk-nix.apply.declarativeHandoff.autoImport.enable` | Applies the generated import patch after successful execution. |
| `configurationPath` | Target NixOS configuration file. |
| `backupDirectory` | Directory for pre-edit backups. |

Auto-import requires `apply.execute = true`. It skips the edit when the handoff
module is already imported.

## Tool Packages

`toolPackages` defaults to the storage tools used by probe and execution
adapters.

| Tool group | Examples |
| --- | --- |
| Core | bash wrappers, coreutils, util-linux, partitioning, `growpart`. |
| Filesystems | Btrfs, bcachefs, ext, XFS, F2FS, exFAT, ZFS. |
| Volumes and encryption | LVM, cryptsetup, MD RAID, VDO. |
| Fabrics | multipath, NFS, iSCSI, SCSI inventory, NVMe, SMART. |
| Targets and caches | `targetcli`, `tgtadm`, bcache. |

Override the list to pin site-specific tool builds or trim unused domains.

## Active vs Planner-Only

| Declaration shape | Native NixOS output | Planner spec |
| --- | --- | --- |
| Active mount | Included in `fileSystems`. | Included for planning/probing context. |
| Active swap | Included in `swapDevices`. | Included for lifecycle context. |
| Active LUKS mapper | Included in initrd LUKS devices. | Included for lifecycle context. |
| NFS export with client/options | Included in NFS export lines. | Included for lifecycle context. |
| `operation = "unmount"` | Filtered out. | Kept for reviewed imperative teardown. |
| `operation = "logout"` | Filtered out of auto-login. | Kept for reviewed imperative logout. |
| `operation = "close"` | Filtered out of initrd LUKS. | Kept for reviewed mapper close. |
| `destroy = true` | Filtered out. | Kept behind destructive policy. |
| Under-specified export | Filtered out. | Kept for review. |

For ZFS and LVM, `operation = "export"` means detach an existing local resource.
For NFS, `operation = "export"` describes an active published export.

## Duplicate Identity Rejection

The module fails evaluation when active declarations would overwrite generated
native state.

| Duplicate type | Examples |
| --- | --- |
| Mount identity | Local mountpoints, NFS mountpoints. |
| Swap identity | Concrete swap paths. |
| Mapper identity | LUKS mapper names, device-mapper maps. |
| Volume identity | VG, LV, thin pool, cache, VDO identities. |
| Pool identity | ZFS pools, datasets, zvols, snapshots. |
| Network identity | iSCSI targets, LUN host paths, NFS export path/client pairs. |
| Array/cache identity | MD arrays, multipath maps, Btrfs subvolumes/qgroups, loop targets. |

## Concrete Target Requirements

| Domain | Concrete target needed before execution |
| --- | --- |
| Swap | Local path or block path. |
| Loop | `/dev/loop*` through `target` or `path`. |
| MD RAID | `/dev/md*` array target. |
| Multipath | `mpath*` or `/dev/mapper/*`. |
| bcache | `/dev/bcache*` through `target`, `path`, or `device`. |
| LVM | Canonical `vg/lv` or `vg/pool`. |
| ZFS | Concrete `pool/name` for datasets and zvols. |
| Btrfs | Absolute subvolume path or numeric qgroup id. |
| NFS export | Local exported path and client selector. |
| LUN provider | Target IQN, LUN id, provider id, backing object, ACLs where needed. |

Logical attribute names are fine for review. They remain non-ready until a
concrete tool address is supplied.

## Filesystems

| Declaration | Native state | Planner behavior |
| --- | --- | --- |
| `device`, `fsType`, `mountpoint`, `options` | `fileSystems` entry. | Mount/remount/rescan context. |
| `operation = "mount"` | Kept active. | Reviewed mount command. |
| `operation = "remount"` | Kept active. | Reviewed remount command. |
| `operation = "unmount"` | Filtered out. | Offline-gated unmount command. |
| `preserveData = false` | No special native output. | Destructive `mkfs.*` plan. |
| `desiredSize` | Native size intent only through disk-nix. | Grow/shrink target where supported. |
| `properties` | Native options where applicable. | Label, UUID, balance, or policy mutation. |

Ext maintenance uses the explicit `device` for `resize2fs` and `e2fsck`.
XFS shrink stays manual-only migration guidance.

## Complex Filesystems

| Domain | Typed lifecycle support |
| --- | --- |
| Btrfs filesystem | Rebalance, check, repair, scrub, trim, device add/remove/replace, labels, balance filters. |
| Btrfs subvolume | Create, delete, rename, read-only property, rescan. |
| Btrfs qgroup | Create, destroy, limit, rescan. |
| bcachefs | Scrub, fsck, member resize/add/remove, rereplicate. |
| ZFS pool | Scrub, import, export, create, destroy, add, replace, remove, properties. |
| ZFS dataset | Create, destroy, rename, promote, properties, rescan. |
| ZFS zvol | Create, destroy, grow, rename, promote, properties, rescan. |
| Snapshots | ZFS/Btrfs create, clone, rename, rescan, destroy; ZFS hold/release and rollback. |

`readOnly = true` on ZFS import renders `zpool import -o readonly=on <pool>`.
Rollback remains potential-data-loss and requires explicit policy.

## Swap And zram

| Option group | Fields |
| --- | --- |
| Swap | `device`, `priority`, `operation`, `desiredSize`, `properties.label`, `properties.uuid`. |
| zram | `enable`, `operation`, `swapDevices`, `memoryPercent`, `memoryMax`, `priority`, `algorithm`. |

zram is modeled separately from persistent swap devices. It derives `zramSwap`
instead of adding `/dev/zram*` to `swapDevices`.

## LUKS

| Declaration | Behavior |
| --- | --- |
| `operation = "open"` | Preserved container open path. |
| `operation = "format"` | Destructive format path. |
| `operation = "close"` | Planner-only mapper close. |
| `desiredSize` | `cryptsetup resize` target. |
| `properties.label` | `cryptsetup config --label`. |
| `properties.subsystem` | `cryptsetup config --subsystem`. |
| `properties.uuid` | `cryptsetup luksUUID --uuid`. |
| keyslots/tokens | Add, remove, import, priority, change-key flows. |

The Nix attribute name becomes the mapper name unless `name`, `mapper`,
`mapperName`, or `target` supplies a concrete mapper identity.

## Volumes, Arrays, And Caches

| Domain | Lifecycle declarations |
| --- | --- |
| LVM PV | Create, grow, rescan, remove. |
| LVM VG | Create, grow/add PV, replace PV, remove PV, import, export, activate, deactivate, rename. |
| LVM LV | Create, grow, remove, rename, activate, deactivate, rescan. |
| Thin pool | Create, grow, remove, rescan. |
| LVM cache/writecache | Attach, detach, replace, rescan, mode/policy mutation. |
| VDO | Create, remove, grow logical/physical, start, stop, write policy. |
| MD RAID | Create, assemble, stop, grow, member add/remove/replace, rescan. |
| Multipath | Resize, path add/remove/replace, flush, rescan. |
| bcache | Attach, detach, replace, cache mode, cache-set properties, rescan. |

Active declarations enable the relevant NixOS services. Teardown and destructive
declarations stay planner-only.

## Network Storage

| Domain | Native state | Planner behavior |
| --- | --- | --- |
| NFS client mount | `fileSystems` when active. | Mount, remount, unmount, rescan. |
| NFS export | `services.nfs.server.exports` when active and complete. | Export, unexport, property updates. |
| iSCSI session | `services.openiscsi` and boot initiator portals. | Discover, login, logout, rescan, grow review. |
| Host-side LUN | No direct native state. | Attach, detach, grow, rescan. |
| Target-side LUN | No generic native state. | LIO, tgt, SCST, and generic provider handoffs. |
| NVMe namespace | No native NixOS state. | Create, delete, grow, attach, detach, rescan. |

Regular iSCSI still requires an explicit initiator name. There is no safe
implicit default for `services.openiscsi.name`.

## Common Address Fields

| Field | Meaning |
| --- | --- |
| `operation` | Lifecycle action. |
| `action` | Alias for `operation`. |
| `target` | Concrete tool address for most domains. |
| `path` | Local path, Btrfs path, NFS export path, or friendly concrete address. |
| `device` | Block device, backing file, or namespace path depending on domain. |
| `mountpoint` | Local or NFS mount target. |
| `name` | Tool object name when native tools address by name. |
| `metadata` | Provider-specific hints and stable identity data. |
| `properties` | Declarative property updates. |

## Apply Policy Fields

| Field | Purpose |
| --- | --- |
| `allowDestructive` | Permit destructive operations. |
| `allowFormat` | Permit formatting. |
| `allowShrink` | Permit supported shrink paths. |
| `allowPotentialDataLoss` | Permit reviewed rollback, removal, shrink, or detach paths. |
| `allowGrow` | Permit supported grow paths. |
| `allowOffline` | Permit offline-required maintenance. |
| `allowPropertyChanges` | Permit identity and policy property mutations. |
| `allowDeviceReplacement` | Permit replacement workflows. |
| `allowRebalance` | Permit Btrfs-style rebalance work. |
| `requireBackup` | Require backup acknowledgement. |
| `backupVerified` | Record backup verification. |
| `requireConfirmation` | Require confirmation text. |
| `requireConfirmationFile` | Require `disk-nix confirm` in a file. |
| `probeCurrent` | Compare against live topology. |
| `failOnBlocked` | Fail activation/install when policy blocks actions. |
| `execute` | Run ready commands instead of only validating. |
| `scriptOut` | Persist review shell script. |
| `reportOut` | Persist JSON report. |
| `receiptOut` | Persist receipt envelope. |
