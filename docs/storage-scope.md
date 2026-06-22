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
- `findmnt --json --bytes` for mounted filesystems, pseudo filesystems, and
  NFS exports
- LVM `pvs`, `vgs`, and `lvs` JSON reports for PV/VG/LV topology, snapshots,
  thin pools, cache-like logical volumes, and VDO-like logical volumes where
  attributes expose them

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
