# Storage scope

`disk-nix` should grow toward full Linux storage awareness.

## Domains

- physical disks and partitions
- filesystem identity, labels, UUIDs, free space, usage, metadata, mountpoints
- LUKS, dm-crypt, and device-mapper tables
- LVM PVs, VGs, LVs, thin pools, snapshots, cache, and VDO
- MD RAID
- Btrfs filesystems, devices, subvolumes, snapshots, qgroups, balance, and usage
- ZFS pools, vdevs, datasets, zvols, snapshots, properties, cache, log, special
  vdevs, and pool health
- iSCSI sessions, targets, portals, and LUNs
- NFS mounts, exports, options, and server metadata where available
- multipath
- NVMe controllers and namespaces
- bcache and other cache layers
- swap, tmpfs, bind mounts, overlayfs, and loop devices

## Update operations

Creation is only one lifecycle operation. The planner should also support:

- grow and shrink where supported
- add, remove, and replace devices
- set safe pool, dataset, filesystem, and volume properties
- create and prune snapshots
- rebalance data
- convert or attach cache layers
- grow LUN-backed devices
- migrate data when direct mutation is impossible

## Current discovery coverage

The current probe layer normalizes:

- `lsblk --json --bytes --output-all` for block devices, partitions,
  filesystems, identity, usage, and mount hints
- `blkid -o export` for filesystem and block signatures, UUIDs, labels,
  PARTUUID/PARTLABEL, signature usage, versions, and block sizes
- `parted -lm` for partition table type, disk sector sizes, partition numbers,
  start/end offsets, partition sizes, names, types, and flags
- `udevadm info --export-db` for block-device udev identity, by-id/by-path
  symlinks, serials, WWNs, filesystem IDs, partition metadata, and
  device-mapper properties
- `findmnt --json --bytes` for mounted filesystems, pseudo filesystems, and
  NFS exports
- `tune2fs -l` for ext2/ext3/ext4 superblock metadata, feature flags,
  filesystem state, mount/check counters, inode and block counts, UUIDs, labels,
  and computed capacity/usage where device access is permitted
- `/proc/swaps` for active swap devices/files, active size, used/free bytes,
  swap type, and priority
- LUKS mapper status through `cryptsetup status` for active/in-use state,
  backing device, cipher, key size, key location, sector size/count, offset, and
  access mode
- Device-mapper metadata through `dmsetup info` and `dmsetup deps` for mapper
  UUIDs, major/minor numbers, open counts, segment/event counts, and backing
  dependency edges
- LVM `pvs`, `vgs`, and `lvs` JSON reports for PV/VG/LV topology, snapshots,
  thin pools, cache-like logical volumes, and VDO-like logical volumes where
  attributes expose them
- VDO `vdo status` output for VDO device path, backing storage device,
  logical/physical size, compression, deduplication, write policy, index, and
  cache settings
- VDO `vdostats --human-readable` output for runtime size, used/free capacity,
  utilization percentage, and space-saving percentage
- ZFS `zpool list -H -p`, `zpool status -P`, and `zfs list -H -p` output for
  pool capacity, health, vdev topology, data/log/cache/special/dedup roles,
  backing devices, datasets, snapshots, zvols, mountpoints, and clone origins
- Btrfs mounted filesystems through `btrfs filesystem show`, `btrfs filesystem usage -b`, `btrfs subvolume list -u`, and `btrfs qgroup show --raw -reF` for
  filesystem identity, member devices, usage, subvolumes, snapshot-like
  subvolume relationships, and qgroup referenced/exclusive usage and limits
- bcache sysfs metadata through `/sys/block/*/bcache` for backing devices,
  cache devices, cache sets, mode, state, dirty data, replacement policy,
  writeback settings, and cache relationships
- iSCSI sessions through `iscsiadm -m session -P 3` for session ids, target
  IQNs, portals, LUNs, and attached SCSI disks
- NFS mount metadata through `nfsstat -m` for server, export, protocol,
  version, transfer sizes, locking, client address, and mount options
- MD RAID arrays through `mdadm --detail --scan` and `mdadm --detail <array>`
  for array UUID, level, state, size, device counts, and member devices
- Multipath maps through `multipath -ll` for map name, WWID, dm device,
  vendor/product, policy metadata, and backing path devices
- NVMe namespaces through `nvme list -o json` for namespace path, serial,
  model, firmware, capacity, usage, LBA, and sector size

LVM probing may report `partial` when the process lacks permission to talk to
device-mapper. That should not prevent the rest of discovery from succeeding.

## Advice examples

If a user asks to shrink XFS, the planner should explain that XFS does not
support shrinking and suggest creating a new smaller filesystem, copying data,
and switching mounts.

If a user asks to remove a Btrfs device, the planner should check free metadata
and data capacity and suggest a filtered balance or temporary replacement
capacity when removal is risky.

If a user asks to destroy a ZFS dataset, the planner should recommend a
snapshot, rename, or unmount-first workflow before any destructive action.
