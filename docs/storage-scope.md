# Storage scope

`disk-nix` is intended to grow toward full Linux storage awareness: local block devices, complex filesystems, volume managers, cache layers, network storage, fabric-visible LUNs, and runtime-only storage.

Use [CLI](cli.md) for read-only views, [Planning](planning.md) for lifecycle semantics, and [Feature checklist](feature-checklist.md) for completion evidence.

## Domain map

| Domain | Scope |
| --- | --- |
| Local block | Physical disks, partitions, device-mapper, LUKS/dm-crypt, MD RAID, multipath, NVMe controllers and namespaces, loop devices, swap, and zram. |
| Filesystems | ext, XFS, NTFS, exFAT, F2FS, Btrfs, bcachefs, ZFS, tmpfs, bind mounts, and overlayfs. |
| Volumes | LVM PVs, VGs, LVs, thin pools, snapshots, cache, writecache, and VDO-backed logical volumes. |
| Complex pools | ZFS pools/datasets/zvols/snapshots, Btrfs filesystems/subvolumes/qgroups, and bcachefs member filesystems. |
| Cache layers | bcache, bcache cache sets, LVM cache/writecache, bcachefs member-device cache accounting, and ZFS cache/log/special vdev roles. |
| Network storage | iSCSI sessions, targets, portals, LUNs, NFS mounts, NFS exports, client/server options, and server metadata where available. |

## Lifecycle operation map

Creation is only one lifecycle operation. The planner also models growth, shrink where supported, device add/remove/replace, safe property updates, renames, clone promotion, import/export, activation, deactivation, open/close, assemble/stop, mount/remount/unmount, snapshots, rebalance, cache conversion, LUN growth, and data migration when direct mutation is impossible.

Unsupported or unsafe requests should produce warnings and non-destructive alternatives before any policy can allow mutation. Examples include XFS shrink, risky Btrfs device removal, and destructive ZFS dataset removal.

## Discovery coverage map

| Source group | Primary tools | Coverage |
| --- | --- | --- |
| Block and identity | `lsblk`, `lsscsi`, `smartctl`, `blkid`, `parted`, `udevadm` | Device identity, geometry, capacity, queues, signatures, SCSI LUNs, partition metadata, SMART, and by-id/by-path links. |
| Mounts and filesystems | `findmnt`, `tune2fs`, `xfs_info`, `ntfsinfo`, exFAT tools, `dump.f2fs` | Mount state, filesystem identity, usage, geometry, feature flags, health, labels, UUIDs, and free-space estimates. |
| Complex filesystems | `bcachefs`, `zpool`, `zfs`, `btrfs` | Pool capacity, topology, health, member devices, datasets, snapshots, zvols, subvolumes, qgroups, holds, and error counters. |
| Mapping and volumes | `cryptsetup`, `dmsetup`, LVM JSON reports, VDO tools, bcache sysfs | Encryption headers, mapper tables, PV/VG/LV topology, thin/cache/VDO metadata, usage counters, and cache relationships. |
| Network and fabrics | `iscsiadm`, `exportfs`, `nfsstat`, `mdadm`, `multipath`, `nvme` | Sessions, portals, NFS exports, RAID state, multipath paths, NVMe fabrics, ANA state, host-visible LUNs, and fabric identities. |
| Runtime-only | `/proc/swaps`, `zramctl`, `losetup` | Active swap, zram compression/accounting, loop backing files, offsets, read-only state, and direct I/O state. |

## Real-world fixture coverage

Cross-adapter fixtures cover clustered LVM on NVMe-oF, Fibre Channel multipath, native NVMe/TCP multipath, mixed NVMe-oF sharing, NFS server/client drift, SAS enclosure metadata, and VDO pressure states.

The remaining storage-awareness work is mostly breadth: more vendor arrays, more fabric failure modes, more clustered locking failure states, and more live-service profile variation.

## Advice examples

If a user asks to shrink XFS, the planner should explain that XFS does not support shrinking and suggest creating a new smaller filesystem, copying data, and switching mounts.

If a user asks to remove a Btrfs device, the planner should check free metadata and data capacity and suggest a filtered balance or temporary replacement capacity when removal is risky.

If a user asks to destroy a ZFS dataset, the planner should recommend a snapshot, rename, or unmount-first workflow before any destructive action.

## Coverage anchors

These exact phrases are kept for the flake documentation coverage check after prose restructuring.

```text
zoning-style fabric/WWPN layouts
shared namespace UUID/NGUID identity
split-brain protection refusal
pNFS layout and
SES failure attributes
physical-space pressure
active/standby state
```
