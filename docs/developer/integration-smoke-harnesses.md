# Integration smoke harnesses

This page summarizes the destructive smoke harnesses by domain.

Use [Integration tests](integration-tests.md) for opt-in entrypoints and disk
safety policy.

## Common Rules

| Rule | Meaning |
| --- | --- |
| Destructive opt-in | Harnesses require `DISK_NIX_INTEGRATION_DESTRUCTIVE=1`. |
| Root where needed | Loop, mapper, volume, array, and filesystem harnesses require root. |
| Disposable targets | Any selected block device, export, session, map, or namespace must be safe to mutate. |
| Development binary | Set `DISK_NIX_BIN` to test a local binary without `nix run`. |
| VM selection | Use `DISK_NIX_VM_HARNESSES="..."` inside a disposable VM. |

## Harness Matrix

| Harness | Package selector | Backing target | Main proof |
| --- | --- | --- | --- |
| Loop | `loop` | Temporary backing file and loop device. | Loop inspect, grow, read-only property, backing-file mode. |
| Btrfs | `btrfs` | Temporary loop filesystem. | Label mutation, scrub, replacement, sentinel survival. |
| bcachefs | `bcachefs` | Temporary member devices. | Scrub, member replacement, rereplicate, sentinel survival. |
| bcache | `bcache` | Temporary backing and cache devices. | Cache mode, attach/detach, replacement, failed attach, rescan. |
| LUKS | `luks` | Temporary loop container. | Header label, open, close, mapper status. |
| Swap | `swap` | Temporary loop swap. | Label mutation, activation, priority, deactivation. |
| zram | `zram` | Existing generated zram state. | Property reconciliation without recreating devices. |
| LVM | `lvm` | Temporary PV/VG/LV/cache. | Cache policy, detach/reattach, replacement, thin/LV rescan. |
| MD RAID | `mdraid` | Temporary RAID1 loop devices. | Replacement, stale superblock, failed detach/reattach, partial rebuild. |
| ZFS | `zfs` | Temporary loop pool. | Property mutation, scrub, device replacement. |
| NFS | `nfs` | Operator-supplied export. | Remount, export option mutation, data-survival checks. |
| VDO | `vdo` | Existing disposable VDO volume. | Write-policy mutation and inventory refresh. |
| iSCSI | `iscsi` | Existing disposable session/LUN. | Session rescan, host-side LUN rescan, data survival. |
| Multipath | `multipath` | Existing disposable map. | Resize, path add/remove/replace, flush. |
| NVMe | `nvme` | Existing disposable controller/namespace. | Create/delete, grow, attach/detach, reconnect, identity drift. |
| Target LUN | `target-lun` | Temporary LIO target state. | LIO property, map/unmap, destroy refusal, host visibility. |
| Layered VM | `layered-vm` | Disposable VM loop disk. | Partition, LUKS, LVM, filesystem grow, failure recovery, remount. |

## Local Loop Harnesses

| Harness | Exercises | Data-safety assertion |
| --- | --- | --- |
| Loop | `losetup`, backing-file create/grow/rescan, loop read-only mutation. | Temporary file and loop device are removed. |
| Btrfs | `mkfs.btrfs`, mount, label, scrub, device replacement. | Sentinel remains readable after replacement. |
| bcachefs | `mkfs.bcachefs`, scrub, device add, rereplicate, device remove. | Sentinel remains readable after member replacement. |
| LUKS | `cryptsetup luksFormat`, open, config, close. | Mapper closes cleanly and backing loop is cleaned up. |
| Swap | `mkswap`, `swapon`, `swaplabel`, `swapoff`. | Temporary swap is deactivated and removed. |

## Cache And Volume Harnesses

| Harness | Exercises | Data-safety assertion |
| --- | --- | --- |
| LVM | PV/VG/LV setup, thin rescan, cache mode, cache detach/reattach, cache replacement. | Cached-origin ext4 sentinel survives cache lifecycle changes. |
| bcache | Backing/cache setup, cache mode, cache-set attach/detach, replacement. | Generated bcache device remains readable after replacement. |
| MD RAID | RAID1 create, member replacement, stale superblock checks, failed detach/reattach. | Array returns to healthy state after partial-rebuild checks. |
| ZFS | Pool create, pool property, scrub, vdev replacement. | Pool mountpoint remains active after replacement. |

## Runtime Harnesses

| Harness | Exercises | Notes |
| --- | --- | --- |
| zram | Algorithm, stream count, size, memory limit, compression ratio, swap priority. | Does not recreate active `/dev/zram*` devices. |
| VDO | `vdo status`, `vdostats`, write policy mutation. | Requires an existing disposable VDO volume. |

## Network And Fabric Harnesses

| Harness | Required variable | Optional destructive paths |
| --- | --- | --- |
| NFS | `DISK_NIX_NFS_SOURCE` | `DISK_NIX_NFS_DATA_SURVIVAL=1`, `DISK_NIX_NFS_EXPORT_PROPERTY=1`. |
| iSCSI | `DISK_NIX_ISCSI_TARGET` | `DISK_NIX_LUN_PATH`, `DISK_NIX_LUN_DATA_SURVIVAL=1`. |
| Multipath | `DISK_NIX_MULTIPATH_MAP` | Resize, path add/remove/replace, flush variables. |
| NVMe | `DISK_NIX_NVME_CONTROLLER` | Create/delete, grow, attach/detach, reconnect variables. |
| Target LUN | LIO target support in the runner. | Map/unmap and destroy-refusal paths. |

These harnesses do not provision real infrastructure for you. They assume the
selected export, session, map, namespace, or target state is disposable.

## Layered VM Harness

The layered harness runs inside a disposable VM and validates a full storage
stack.

| Layer | Covered behavior |
| --- | --- |
| Backing file and partition | File grow, partition grow, table reread. |
| LUKS | Open, resize, close, reopen. |
| LVM | PV/VG/LV grow and activation checks. |
| Filesystem | ext4 grow, remount, sentinel preservation. |
| Failure injection | Partial mutation, failed command report, rollback review. |
| Resume | Clean follow-up apply and final verification. |

Rollback review remains non-mutating in this harness. Recovery commands are
read-only unless a separate proven-safe rollback recipe explicitly qualifies.

## Default VM Suite

The default VM suite runs:

- loop
- Btrfs
- swap
- layered VM
- failure recovery

Other harnesses remain packaged and selectable through `DISK_NIX_VM_HARNESSES`.
Some require kernel modules, services, or lab infrastructure that the flake VM
does not provision by default.
