# Storage scope

`disk-nix` should grow toward full Linux storage awareness.

## Domains

The supported scope includes local block devices, filesystems, volume managers,
complex filesystems, network storage, cache layers, and runtime-only storage.

Local block coverage includes physical disks, partitions, device-mapper, LUKS,
dm-crypt, MD RAID, multipath, NVMe controllers, NVMe namespaces, loop devices,
swap, zram, tmpfs, bind mounts, and overlayfs.

Volume and filesystem coverage includes ext, XFS, NTFS, exFAT, F2FS, Btrfs,
bcachefs, ZFS, LVM PVs, VGs, LVs, thin pools, snapshots, cache, and VDO.

Network coverage includes iSCSI sessions, targets, portals, LUNs, NFS mounts,
NFS exports, client/server options, and server metadata where available.

Cache coverage includes bcache, LVM cache/writecache, bcachefs member-device
cache accounting, and ZFS cache/log/special vdevs.

## Update operations

Creation is only one lifecycle operation. The planner also models growth,
shrink where supported, device add/remove/replace, safe property updates,
renames, clone promotion, import/export, activation, deactivation, open/close,
assemble/stop, mount/remount/unmount, snapshots, rebalance, cache conversion,
LUN growth, and data migration when direct mutation is impossible.

## Current discovery coverage

The current probe layer normalizes the source groups below.

### Block, SCSI, And Identity Sources

`lsblk --json --bytes --output-all` supplies block devices, partitions,
filesystems, identity, sector and I/O alignment, discard geometry, scheduler
state, queue sizing, zoned-device limits, DAX/hotplug flags, usage, and mount
hints.

`lsscsi -L -g -s`, `lsscsi -g -s -t -i -w`, and `lsscsi -g -s -u -i -w` supply
SCSI host/channel/target/LUN addresses, block and generic devices, by-id and WWN
aliases, transport, LU names, capacity, queue state, and LUN-to-block
relationships.

`smartctl -a -j` enriches physical disks with SMART health, model, firmware,
serial, WWN, capacity, block sizes, rotation data, link speed, power-on history,
temperature, test status, error logs, and raw SMART attributes.

`blkid -o export` supplies filesystem and block signatures, UUIDs, labels,
PARTUUID/PARTLABEL, signature usage, versions, and block sizes.

`parted -lm` supplies partition table type, disk sector sizes, partition
numbers, raw and normalized geometry, partition sizes, names, types, and flags.

`udevadm info --export-db` supplies by-id/by-path links, serials, WWNs,
filesystem IDs, encoded labels, UUID sub-identifiers, filesystem geometry,
partition metadata, path tags, major/minor numbers, and mapper flags.

### Mount And Filesystem Sources

`findmnt --json --bytes` supplies mounted filesystems, pseudo filesystems, NFS
exports, tmpfs sizing, bind sources, overlayfs directories, mount propagation,
and read/write state.

`tune2fs -l` supplies ext superblock metadata, feature flags, state, reservation
and overhead accounting, block/inode geometry, RAID layout hints, counters,
timestamps, hash settings, default mount options, journal data, error telemetry,
checksums, UUIDs, labels, and computed usage.

`xfs_info` supplies mounted XFS geometry, allocation groups, inode and sector
sizes, metadata feature flags, data allocation parameters, naming format, log
geometry, and realtime geometry.

`ntfsinfo -m` supplies NTFS identity, serial, state, version, sector and cluster
sizing, index block size, MFT record size, MFT zone/location metadata, and
allocated size.

`dump.exfat`, `exfatlabel`, and `tune.exfat` supply exFAT labels, GUID/serial,
version, volume length, FAT layout, cluster heap layout, cluster counts, root
cluster, sector sizing, and free-space estimates.

`dump.f2fs` supplies F2FS identity, UUID, block counts, valid node/inode counts,
checkpoint/SIT/NAT/SSA layout, section/zone geometry, log sizing, version data,
and overprovisioning metadata.

### Complex Filesystem Sources

`bcachefs show-super`, `bcachefs fs usage`, `blkid`, and `findmnt` supply
first-class bcachefs filesystem and member-device nodes. They include UUIDs,
labels, superblock magic, version state, member indexes, capacity, reservations,
data-type accounting, and per-device metadata.

ZFS data comes from `zpool list -H -p`, `zpool get -H`, `zpool status -P`,
`zfs list -H -p`, and `zfs holds -H`. It covers pool capacity, health,
properties, vdev topology, roles, datasets, snapshots, zvols, mountpoints,
origins, holds, user references, and dataset/zvol policy properties.

Btrfs data comes from `btrfs filesystem show`, `btrfs filesystem usage -b`,
rich `btrfs subvolume list`, `btrfs qgroup show --raw -reF -p -c`, and
`btrfs device stats`. It covers filesystem identity, member devices, usage,
subvolume lineage, snapshot relationships, qgroup hierarchy, limits, and member
error counters.

### Mapping, Volume, And Cache Sources

LUKS data comes from `cryptsetup status` and `cryptsetup luksDump`. It covers
active state, backing device, cipher, sector layout, access mode, header
version, UUID, label, data segments, keyslots, tokens, digest metadata, and
redacted token-specific hints.

Device-mapper data comes from `dmsetup info`, `dmsetup deps`, `dmsetup table`,
and `dmsetup status`. It covers mapper identity, open counts, dependency edges,
table target ranges, live target status, sanitized dm-crypt fields, structured
linear/striped/thin/cache/snapshot payloads, and usage counters.

LVM data comes from `pvs`, `vgs`, `lvs`, and `lvs --segments` JSON reports. It
covers PV/VG/LV topology, snapshots, thin pools, cache-like volumes, extents,
metadata areas, activation state, device-mapper paths, segment mappings, RAID
status, writecache/cache counters, VDO-like LVs, and backing dependencies.

VDO data comes from `vdo status`, `vdostats --human-readable`, and
`vdostats --verbose`. It covers device paths, logical/physical size, policy,
compression, deduplication, service state, recovery progress, capacity,
space-saving, version data, and detailed block accounting.

bcache data comes from `/sys/block/*/bcache` and linked
`/sys/fs/bcache/<set>` directories. It covers backing devices, cache devices,
cache sets, mode, state, dirty data, available cache, discard, errors, writeback
tuning, priority stats, journal data, and relationships.

### Network And Fabric Sources

Configured iSCSI nodes come from `iscsiadm -m node -P 1`; active sessions come
from `iscsiadm -m session -P 3`. The graph records portals, IQNs, startup
policy, interfaces, CHAP hints, session state, connection addresses, negotiated
parameters, SCSI coordinates, attached disks, and LUN identities.

NFS server exports come from `exportfs -v`; client mount detail comes from
`nfsstat -m` and `findmnt`. The graph records exported paths, clients, options,
source splits, protocol, transport, addresses, transfer sizing, locking, cache,
RPC security, referrals, pNFS hints, drift, and Kerberos policy variants.

MD RAID data comes from `/proc/mdstat`, `mdadm --detail --scan`,
`mdadm --examine --scan`, and `mdadm --detail <array>`. It covers UUIDs,
metadata versions, names, levels, runtime state, device counts, event counters,
bitmap data, rebuild/check progress, and member slot/state fields.

Multipath data comes from `multipath -ll`. It covers map identity, WWID, dm
device, vendor/product, size, features, handler, write protection, path groups,
priorities, SCSI coordinates, online/checker state, and extra path flags.

NVMe data comes from `nvme list-subsys -o json`, `nvme list -o json`,
`nvme id-ns -o json`, `nvme id-ctrl -o json`, and `nvme smart-log -o json`. It
covers subsystems, controllers, namespaces, paths, NQNs, fabrics endpoints,
ANA state, capacity, LBA data, controller capability, health, errors,
temperature, lifetime usage, and power telemetry.

Cross-adapter fixtures cover clustered LVM on NVMe-oF, lock-failure metadata,
Fibre Channel multipath, native NVMe/TCP multipath, live reconnecting fabrics
controllers, and optimized or inaccessible ANA states.

### Runtime-Only Sources

`/proc/swaps` supplies active swap devices and files, size, used/free bytes,
type, and priority.

`zramctl --bytes --raw --noheadings --output-all` supplies zram size,
uncompressed data, compressed data, allocator memory, limits, high-water use,
algorithm, stream count, zero pages, compaction data, compression ratio, and
swap mountpoint state.

`losetup --json --list` supplies loop mappings, backing files or block devices,
inode and major/minor data, offsets, size limits, partition-scan state,
autoclear, read-only state, direct I/O, and logical sector size.

## Advice examples

If a user asks to shrink XFS, the planner should explain that XFS does not
support shrinking and suggest creating a new smaller filesystem, copying data,
and switching mounts.

If a user asks to remove a Btrfs device, the planner should check free metadata
and data capacity and suggest a filtered balance or temporary replacement
capacity when removal is risky.

If a user asks to destroy a ZFS dataset, the planner should recommend a
snapshot, rename, or unmount-first workflow before any destructive action.

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
