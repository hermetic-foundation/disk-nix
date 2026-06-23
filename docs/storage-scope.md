# Storage scope

`disk-nix` should grow toward full Linux storage awareness.

## Domains

- physical disks and partitions
- filesystem identity, labels, UUIDs, free space, usage, metadata, mountpoints
- LUKS, dm-crypt, and device-mapper tables
- LVM PVs, VGs, LVs, thin pools, snapshots, cache, and VDO
- MD RAID
- Btrfs filesystems, devices, subvolumes, snapshots, qgroups, balance, and usage
- bcachefs filesystems, member devices, mounted usage, data-type accounting, and
  lifecycle topology operations
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
- rename datasets, volumes, volume groups, subvolumes, and snapshots before
  final cleanup
- promote ZFS clones after snapshot-based validation or migration
- import and export ZFS pools when moving or recovering existing storage
- import and export LVM volume groups when moving existing PV sets
- activate and deactivate LVM volumes, thin pools, snapshots, and volume groups
- open and close LUKS mappings without formatting encrypted containers
- assemble and stop existing MD RAID arrays without removing member metadata
- mount, remount, and unmount NFS client mounts without deleting remote data
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
  symlinks, serials, WWNs, filesystem IDs, encoded/safe labels, UUID
  sub-identifiers, partition geometry/table metadata, path tags, major/minor
  numbers, and device-mapper udev flags/properties
- `findmnt --json --bytes` for mounted filesystems, pseudo filesystems, NFS
  exports, tmpfs sizing and ownership options, bind source paths, overlayfs
  lower/upper/work directories, mount propagation, and read/write state
- `tune2fs -l` for ext2/ext3/ext4 superblock metadata, feature flags,
  filesystem state, mount/check counters, inode and block counts, UUIDs, labels,
  and computed capacity/usage where device access is permitted
- `xfs_info` for mounted XFS filesystem geometry, allocation group layout,
  inode size, sector size, metadata feature flags such as reflink, bigtime,
  finobt, sparse, and rmapbt, data allocation parameters, naming format, log
  geometry, and realtime extent/block counters
- `ntfsinfo -m` for NTFS volume identity, serial, state, version, sector and
  cluster sizing, index block size, MFT record size, and allocated size
- `dump.f2fs` for F2FS volume identity, UUID, block counts, valid block usage,
  segment counts, section/zone geometry, and overprovisioning metadata
- `bcachefs show-super`, `bcachefs fs usage`, `blkid`, and `findmnt` for
  first-class bcachefs filesystem and member-device nodes with
  external/internal UUIDs, labels, member-device indexes, mounted capacity,
  online reservations, data-type byte accounting, and per-device free/capacity
  metadata
- `/proc/swaps` for active swap devices/files, active size, used/free bytes,
  swap type, and priority
- `losetup --json --list` for loop device mappings, backing files or block
  devices, offsets, size limits, autoclear state, read-only state, direct I/O,
  and logical sector size
- LUKS mapper status through `cryptsetup status` for active/in-use state,
  backing device, cipher, key size, key location, sector size/count, offset, and
  access mode; LUKS header metadata through `cryptsetup luksDump` for version,
  UUID, label, data segment, keyslot, PBKDF, and token metadata
- Device-mapper metadata through `dmsetup info` and `dmsetup deps` for mapper
  names, UUIDs, major/minor numbers, open counts, segment/event counts, and
  backing dependency edges
- LVM `pvs`, `vgs`, and `lvs` JSON reports for PV/VG/LV topology, snapshots,
  thin pools, cache-like logical volumes, and VDO-like logical volumes where
  attributes expose them
- LVM `lvs --segments` JSON reports for LV segment type, segment size/start,
  physical extent ranges, and dependencies on backing PV devices or internal LVs
- VDO `vdo status` output for VDO device path, backing storage device,
  logical/physical size, compression, deduplication, write policy, index, and
  cache settings; VDO lifecycle plans can start or stop existing VDO volumes
  without recreating or removing metadata
- VDO `vdostats --human-readable` output for runtime size, used/free capacity,
  utilization percentage, and space-saving percentage; `vdostats --verbose`
  output for operating mode, recovery percentage, write policy, and detailed
  VDO block/accounting metadata
- exFAT metadata through `tune.exfat` and `dump.exfat` for volume label, GUID,
  serial, volume length, FAT/cluster offsets, cluster counts, sector sizing,
  allocated capacity, and free-space estimates
- ZFS `zpool list -H -p`, `zpool status -P`, and `zfs list -H -p` output for
  pool capacity, health, vdev topology, data/log/cache/special/dedup roles,
  backing devices, datasets, snapshots, zvols, mountpoints, clone origins, and
  snapshot user-reference counts that indicate active holds; ZFS dataset and
  zvol policy properties include compression, quota, reservation, encryption,
  key status, and volsize
- Btrfs mounted filesystems through `btrfs filesystem show`, `btrfs filesystem usage -b`, `btrfs subvolume list -u`, and `btrfs qgroup show --raw -reF` for
  filesystem identity, member devices, usage, subvolumes, snapshot-like
  subvolume relationships, and qgroup referenced/exclusive usage and limits
- bcache sysfs metadata through `/sys/block/*/bcache` for backing devices,
  cache devices, cache sets, mode, running/state, dirty data, available cache
  percentage, discard, I/O error counters, written/metadata-written accounting,
  replacement policy, priority stats, writeback delay/running/metadata
  settings, and cache relationships
- iSCSI sessions through `iscsiadm -m session -P 3` for session ids, target
  IQNs, current and persistent portals, interface identity, connection/session
  state, negotiated transfer parameters, host state, LUN SCSI coordinates, and
  attached disk state
- NFS mount metadata through `nfsstat -m` for server, export, alternate
  `target from source` records, protocol version, transport and mount
  transport, port/mount address, transfer sizes, timeout/retransmit settings,
  local locking, lookup cache, FS-Cache, age, and mount options
- MD RAID arrays through `mdadm --detail --scan` and `mdadm --detail <array>`
  for array UUID, level, state, size, device counts, and member devices
- Multipath maps through `multipath -ll` for map name, WWID, dm device,
  vendor/product, map features, hardware handler, write-protect state,
  path-group policy/priority/status, and backing path device state
- NVMe namespaces through `nvme list -o json` for namespace path, generic
  namespace path, serial, model/product, firmware, subsystem, controller,
  address, namespace id/index, capacity, usage, LBA, and sector size

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
