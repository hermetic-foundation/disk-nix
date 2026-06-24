# CLI

`disk-nix` exposes human-readable commands for interactive inspection and JSON
commands for automation. JSON is the intended stable interface; text output is
presentation over the same storage model.

## Discovery

Use topology discovery to see the full probed graph:

```sh
disk-nix topology
disk-nix topology --json
```

`topology` reports node and edge counts plus probe adapter status. The JSON
form emits the complete `StorageGraph`:

```json
{
  "nodes": [],
  "edges": []
}
```

Nodes include:

- `id`
- `kind`
- `name`
- `path`
- `sizeBytes`
- `usage`
- `identity`
- `properties`

Edges include:

- `from`
- `to`
- `relationship`

The graph can represent block devices, partitions, filesystems, mountpoints,
swap, LUKS, device-mapper, LVM, VDO, MD RAID, Btrfs, ZFS, exFAT, NTFS, iSCSI,
LUNs, NFS, bcache, multipath, NVMe namespaces, and loop devices. Nodes are
merged by id when multiple probe adapters report complementary information.
exFAT probing uses `tune.exfat` and `dump.exfat` when available to add label,
GUID, serial, tool version, sector, cluster, size, used-cluster, and free-space
metadata beyond generic `blkid` fields. NTFS probing uses `ntfsinfo -m` when
available to add volume name/state/version, serial, sector/cluster sizing,
index block size, MFT record size, and allocated size. F2FS probing uses
`dump.f2fs` when available to add volume name, UUID, user/valid block counts,
checkpoint/SIT/NAT/SSA segment layout, section/zone geometry, log sizing,
version metadata, overprovisioning, and computed usage. bcachefs probing uses
`bcachefs show-super` and
`bcachefs fs usage` when available to add external/internal UUIDs, labels,
superblock magic, version and upgrade state, member-device indexes, mounted
capacity, filesystem data-type byte accounting, and per-device free/capacity,
superblock, journal, btree, user, and cached metadata.

## Probe Status

Probe status explains what data was available on the current host:

```sh
disk-nix probe-status
disk-nix probe-status --json
```

Each adapter reports one of:

- `available`: the adapter ran and returned usable data
- `partial`: the adapter ran but some commands or objects were inaccessible
- `unavailable`: the required command, service, kernel surface, or data source
  was not present
- `failed`: the adapter unexpectedly failed

Unavailable or partial adapters are not fatal. They mean the graph is degraded
for that storage domain. For example, a host without `zpool` can still report
regular block devices, but it cannot report ZFS pool and dataset details.

## Shell Completions

The Nix package installs bash, zsh, and fish completion files under the usual
share directories. The CLI can also emit completion scripts directly:

```sh
disk-nix completions bash
disk-nix completions zsh
disk-nix completions fish
```

Completion output is generated from the same Clap command definition used by
the binary, so new subcommands and flags are reflected in both packaged and
manual completions.

## Manpage

The Nix package installs a generated `disk-nix(1)` manpage. The CLI can also
emit the roff source directly:

```sh
disk-nix manpage
```

The manpage is generated from the same Clap command definition as `--help`, so
packaged documentation tracks the supported subcommands and flags.

## Spec Schema

`schema` emits a JSON Schema-style contract for desired storage specs:

```sh
disk-nix schema
```

The schema describes both direct planner specs and the NixOS module wrapper
shape with top-level `spec` and `apply` objects. It includes the planner's
filesystem fields, including filesystem `operation`, `device`, mount
`options`, `properties`, `metadata`, `neededForBoot`, `destroy`, and Btrfs
device-membership update fields. It also includes disk and partition lifecycle
collections, swap, LUKS, LUKS keyslots/tokens, NFS mount wrappers with
planner-only `metadata`, iSCSI discovery/session/boot wrappers, Btrfs
subvolume, VDO, LVM physical volume, LVM thin pool, LVM snapshot, LVM cache,
loop-device, MD RAID, multipath, NVMe namespace, and zvol lifecycle
declarations, higher-layer lifecycle collections, snapshot declarations
including Btrfs `readOnly` snapshot intent, supported operation names, apply
policy fields, and NixOS activation helper fields such as `probeCurrent`,
`failOnBlocked`, `scriptOut`, and `reportOut`.
Planner-compatible aliases such as `number`, `startOffset`, `endOffset`, and
`raidLevel` are included for editor completion and validation parity.
The Nix package installs the same generated schema at
`share/disk-nix/schema/disk-nix-spec.schema.json`.

## Focused Views

Focused commands filter the graph for common workflows:

```sh
disk-nix devices
disk-nix partitions
disk-nix filesystems
disk-nix complex-filesystems
disk-nix zfs
disk-nix volumes
disk-nix pools
disk-nix snapshots
disk-nix mappings
disk-nix encryption
disk-nix cache
disk-nix lvm
disk-nix vdo
disk-nix multipath
disk-nix nvme
disk-nix raid
disk-nix loop
disk-nix swap
disk-nix iscsi
disk-nix nfs
disk-nix mounts
disk-nix network-storage
disk-nix ids
disk-nix usage
```

Every focused view accepts `--json`:

```sh
disk-nix devices --json
disk-nix partitions --json
disk-nix filesystems --json
disk-nix complex-filesystems --json
disk-nix zfs --json
disk-nix volumes --json
disk-nix pools --json
disk-nix snapshots --json
disk-nix mappings --json
disk-nix encryption --json
disk-nix cache --json
disk-nix lvm --json
disk-nix vdo --json
disk-nix multipath --json
disk-nix nvme --json
disk-nix raid --json
disk-nix loop --json
disk-nix swap --json
disk-nix iscsi --json
disk-nix nfs --json
disk-nix mounts --json
disk-nix network-storage --json
disk-nix ids --json
disk-nix usage --json
```

The JSON form returns a `StorageGraph` subgraph. Edges are preserved when both
endpoints are included in the filtered node set.

Use these commands for:

- `devices`: disks, partitions, dm devices, LVM objects, VDO, RAID, zvols,
  cache devices, multipath devices, NVMe namespaces, loop devices, and swap,
  including model/vendor, transport, rotational, NVMe model/firmware/namespace
  geometry, partition table/number, filesystem type, loop
  backing/offset/autoclear metadata, multipath path host/major-minor/state
  details, MD RAID member number/major/minor/raid-device/state, active swap
  state/type/priority, and udev by-id/by-path links, encoded labels,
  filesystem UUID sub-identifiers,
  partition table metadata, major/minor numbers, and device-mapper flags when
  probes expose them
- `partitions`: partition nodes with size, PARTUUID, path, filesystem type,
  `blkid` signature details, partition number, start/end geometry, type/name,
  and flags when probes expose them
- `filesystems`: regular filesystems, Btrfs filesystems/subvolumes/snapshots,
  bcachefs filesystems, ZFS datasets/snapshots, and NFS exports, with selected
  filesystem metadata details such as `blkid` version/block-size/usage, exFAT
  GUID/serial, volume length, FAT and cluster-heap layout, root cluster, and
  raw plus derived cluster geometry, NTFS volume identity, version, cluster
  sizing, and MFT record sizing, F2FS block usage, valid inode/node counts, segment layout,
  section/zone geometry, log sizing, version, and overprovisioning metadata,
  XFS source, allocation-group, inode, data, naming format, log type/sizing,
  realtime type/geometry, and metadata feature details,
  bcachefs external/internal UUID, superblock magic, version/upgrade state,
  member-device, mounted usage, and filesystem/member data-type byte
  accounting, Btrfs Data/Metadata/System allocation profiles and byte counts,
  and ext state/features, reserved and overhead block accounting, block/inode
  group geometry, mount/check counters, timestamps, directory hash settings,
  default mount options, lifetime writes, journal identity, and checksum
  metadata when probes expose them
- `complex-filesystems`: Btrfs, bcachefs, and ZFS pools, vdevs, datasets,
  zvols, subvolumes, snapshots, qgroups, and member devices, including size,
  used/free capacity, utilization, backing/member counts, allocation profiles,
  qgroup limits, bcachefs superblock and member accounting, ZFS health/vdev
  state, and ZFS compression/quota/reservation/encryption properties when
  probes expose them
- `zfs`: ZFS pools, vdevs, datasets, snapshots, and zvols, including pool
  health/state, status/action advisories, scan/error summaries, vdev roles and
  error counters, dataset compression, quota, reservation, encryption, key
  status, snapshot user references, zvol volume size, origin, and
  pool/dataset/snapshot child relationships when `zpool` and `zfs` probes
  expose them
- `volumes`: logical storage objects such as LVM, Btrfs, bcachefs, ZFS, zvols,
  LUNs, and exports, including LVM origin/pool/data metadata, activation state,
  role, layout, health, tags, thin-pool fullness behavior, metadata size, and
  cache or writecache status, MD RAID level/state, iSCSI attached disks, NFS
  server/export details, and ZFS zvol `volsize` when reported by `zfs list`
- `pools`: storage pools and grouping layers such as LVM volume groups, thin
  pools, Btrfs filesystems/qgroups, bcachefs filesystems, ZFS pools/vdevs, and
  MD RAID arrays,
  including ZFS health/vdev role/state/error counters, LVM extent and PV/LV
  counts, Btrfs qgroup limits, and MD RAID metadata version, name, level,
  state, device counts, and event counters where probes expose them
- `snapshots`: snapshot objects across LVM, Btrfs, and ZFS, including known
  source relationships, LVM origin/pool/data metadata, Btrfs subvolume IDs,
  generation, top-level, and parent UUIDs, and ZFS user-reference, compression,
  and encryption details
- `mappings`: encryption headers/keyslots/tokens, device-mapper, LVM, VDO,
  RAID, multipath, and cache layers, including LUKS active/keyslot/token
  counts, keyslot priorities/ciphers/PBKDF cost metadata, token-to-keyslot
  bindings, header area/epoch/flag details, data-segment cipher/offset/length
  details, dm name/UUID, major/minor numbers, open/segment counters, LVM
  segment data/metadata device mappings,
  thin-pool discard/zeroing/transaction details, cache segment policy/settings,
  VDO segment compression/dedup/write-policy details, multipath WWID/size/path
  state, VDO backing device, logical/physical size, mode, configured and active
  write policy, index/cache sizing, data-reduction settings, and block
  accounting, loop backing/offset/read-only/direct-I/O settings, and bcache
  role/cache-set/tuning details such as label, state, running flag, available
  cache percentage, cache mode, discard, I/O errors, written/metadata-written
  accounting, readahead, sequential cutoff, priority stats, writeback delay,
  and writeback rate when probes expose them
- `encryption`: LUKS/dm-crypt mappings and header metadata, including cipher,
  active/in-use state, keyslot/token counts and ids, LUKS version, epoch,
  metadata/keyslot area sizes, flags, subsystem, keyslot priority/cipher/PBKDF
  details, token-to-keyslot bindings, and data-segment details
- `cache`: bcache devices/cache sets, LVM cache/writecache metadata, bcachefs
  member-device cache accounting, and ZFS cache vdevs, including cache mode,
  policy, dirty/writeback data, cache-set identity, state/running flags,
  available cache percentage, discard, I/O errors, written/metadata-written
  accounting, priority stats, and vdev state
- `lvm`: LVM physical volumes, volume groups, logical volumes, segments, thin
  pools, snapshots, and cache/writecache layers, including data and metadata
  percentages, active state, extent/PV/LV counts, origin/pool relationships,
  thin-pool fullness behavior, segment device mappings, cache policy, health,
  tags, and backing/member counts when `pvs`, `vgs`, `lvs`, or `dmsetup`
  expose them
- `vdo`: native VDO volumes and LVM VDO segment metadata, including backing
  device, logical and physical size, status/stat counters, operating mode,
  recovery progress, configured and active write policy, index/cache sizing,
  compression, deduplication, version/release data, and block accounting when
  probes expose them
- `multipath`: multipath maps and their backing paths, including WWID, dm
  device, vendor/product, size, features, hardware handler, write protection,
  path count, host path, major/minor, path-group policy, priority, group
  status, and path state when `multipath -ll` exposes them
- `nvme`: NVMe namespaces, including path, serial, model, firmware, namespace
  index/id, generic namespace path, subsystem, controller, controller id,
  transport, address, namespace capacity, LBA format, maximum LBA, sector size,
  physical size, used bytes, free bytes, and utilization when `nvme list -o json` exposes them
- `raid`: MD RAID arrays and member devices, including array UUID, metadata
  version, level, state, size, raid, total, array, active, working, failed,
  spare, and degraded device counts, event counters, chunk/layout details,
  consistency policy, rebuild, resync, and check progress, intent bitmap,
  timestamps, and per-member number, major/minor, raid-device, and state when
  `mdadm --detail` exposes them
- `loop`: loop devices and backing files/devices, including backing path,
  offset, size limit, logical sector size, major/minor, autoclear,
  read-only, and direct-I/O settings when `losetup --json` exposes them
- `swap`: active swap devices and files, including type, priority, active
  state, size, used bytes, free bytes, utilization, and backing relationship
  when `/proc/swaps` exposes them
- `iscsi`: iSCSI sessions, targets, and LUNs, including current and persistent
  portals, target portal group tag, connection/session state, connection
  CID/local/peer addresses, interface identity, negotiated transfer parameters,
  target IQNs, LUN sizes, SCSI host/channel/id coordinates, attached disk
  state, session to target imports, target-contained LUN counts, and
  LUN-to-block-device backing relationships when
  `iscsiadm --mode session -P 3` exposes them
- `nfs`: NFS exports and client mounts, including source, server/export split,
  NFS protocol version, transport and mount transport, security flavor,
  client/server addresses, port/mount address, read/write transfer sizes,
  timeout/retransmit settings, local locking, lookup cache, FS-Cache,
  capability flags, transfer multipliers, directory transfer/block sizing,
  RPC security flavor identifiers, age, and export-to-client mount
  relationships when `findmnt` or NFS mount probes expose them
- `mounts`: local mountpoints and NFS mounts, including mount source,
  read/write state, bind indicators, tmpfs sizing/mode metadata, and overlayfs
  lower/upper/work directory options when `findmnt` reports them
- `network-storage`: iSCSI sessions, iSCSI targets, LUNs, NFS exports, and NFS
  mounts, including iSCSI current and persistent portals, connection/session
  state, interface identity, negotiated transfer parameters, SCSI coordinates,
  attached disk state, plus NFS mount source, server/export, protocol,
  security, client/server address, mount transport, cache, timeout, and
  transfer-size details when probes expose them
- `ids`: nodes with UUID, PARTUUID, label, serial, or WWN identity fields
- `usage`: nodes with size, used, free, allocated, utilization, or selected
  metadata detail data, including bcache role/set/state, cache mode,
  replacement policy, available cache percentage, dirty data, I/O errors,
  writeback percentage, `blkid` signature
  details, ext superblock state, block/inode geometry, reservation, mount/check,
  and journal details, LVM layout, health, thin/cache/writecache
  status, NTFS volume geometry and MFT record sizing, F2FS block usage,
  valid inode/node counts, segment layout, section/zone geometry, log sizing,
  bcachefs filesystem and member-device capacity plus data-type accounting,
  Btrfs allocation class profiles and byte counts, VDO backing, logical/physical
  size, data-reduction, cache/index, and block-accounting details, NVMe namespace
  details, loop mapping details, and active swap
  state/type/priority when probed

## Inspect

`inspect` finds nodes by id, path, name, UUID, PARTUUID, label, serial, WWN, or
property key/value:

```sh
disk-nix inspect /dev/nvme0n1
disk-nix inspect /
disk-nix inspect tank/home
disk-nix inspect 7420d5e2-2f0f-4709-a1d1-61a9116412f8
```

The text form prints identity fields, properties, and direct relationships for
matched nodes. The JSON form returns a subgraph containing matched nodes, direct
neighbor nodes, and the relationship edges between them:

```sh
disk-nix inspect / --json
```

This is the preferred machine-readable query surface for drilling into one
device, filesystem, pool, dataset, LUN, mount, or mapping layer.

## Capabilities

Capabilities describe modeled operation support and safety classes:

```sh
disk-nix capabilities
disk-nix capabilities --json
```

The matrix includes local block layers, complex filesystems, cache layers, NFS
exports and client mounts, iSCSI sessions, LUNs, safe property updates,
ZFS/Btrfs/LVM snapshots, and topology updates for ZFS pools, LVM volume
groups, MD RAID, multipath, Btrfs, NVMe namespaces, and cache devices.

The JSON records include:

- `nodeKind`
- `operation`
- `risk`
- `advice`

Risk classes are:

- `safe`
- `online`
- `offline-required`
- `reversible`
- `potential-data-loss`
- `destructive`
- `irreversible`
- `unsupported`

Capabilities are not a promise that an operation can run on the current host.
They are the planner's storage-domain model. The apply policy still decides
whether a planned action may proceed.

## Planning

Plan desired changes from a JSON spec:

```sh
disk-nix plan --spec ./examples/simple-root.json
disk-nix plan --spec ./examples/lifecycle-update.json
disk-nix plan --spec ./examples/simple-root.json --json
disk-nix plan --spec ./examples/simple-root.json --probe-current --json
```

The planner accepts either a direct storage spec or the NixOS module wrapper
written to `/etc/disk-nix/spec.json`.

Plan JSON includes:

- `summary.actionCount`
- `summary.offlineRequiredCount`
- `summary.destructiveCount`
- `summary.potentialDataLossCount`
- `summary.unsupportedCount`
- `topologyComparison` when `--probe-current` is set
- `actions`

Each action includes the target id, operation, risk class, destructive flag,
typed context, and optional advice with non-destructive alternatives.
With `--probe-current`, the CLI also probes the current host and adds
`topologyComparison`, including matched target counts, missing target counts,
size diagnostics, filesystem type conflicts, and already-satisfied property or
size checks. The comparison is advisory and does not mutate storage.

## Apply Evaluation

Apply defaults to policy evaluation and dry-run command planning:

```sh
disk-nix apply --spec ./examples/lifecycle-update.json
disk-nix apply --spec ./examples/lifecycle-update.json --json
disk-nix apply --spec ./examples/lifecycle-update.json --probe-current --json
disk-nix apply --spec ./examples/simple-root.json --script-out ./disk-nix-apply.sh
disk-nix apply --spec ./examples/lifecycle-update.json --report-out ./apply-report.json
disk-nix validate --spec ./examples/lifecycle-update.json --json
```

The report includes:

- `status`
- `apply.policy`
- `apply.allowedCount`
- `apply.blockedCount`
- `apply.blockedSummary`
- `apply.blocked`
- `topologyComparison` when `--probe-current` is set
- `commandSummary`
- `commandPlan`
- `verificationSummary`
- `verificationPlan`
- `executionResults` when `--execute` runs commands
- `messages`

The default policy allows online grow and property-change intents, but blocks
offline-required, destructive, irreversible, format, shrink, and
potential-data-loss actions. Set `allowPotentialDataLoss = true` only for
reviewed rollback, shrink, or device-removal workflows; `requireBackup` and
`requireConfirmation` still gate those actions when enabled. Unsupported
actions are always blocked.

`--execute` runs storage commands only after policy validation passes and every
planned command reports `ready`. It refuses plans with unresolved desired sizes,
domain-specific placeholders, or manual-only commands. Execution is sequential,
stops on the first failed command, records stdout, stderr, and exit status for
each command result, and runs verification commands only after all planned
commands succeed:

```sh
disk-nix apply --spec ./examples/lifecycle-update.json --execute
```

Automation should treat a blocked apply report as a hard stop and surface the
reported advice before requesting a more permissive policy.
`commandSummary` reports total steps, total commands, mutating commands,
manual-review steps, and readiness counts so callers can gate automation before
iterating detailed commands.
When policy allows an action, `commandPlan` records the planned commands,
whether each command mutates system state, and notes that still require
operator review. Each command also reports readiness:
`ready`, `needs-desired-size`, `needs-domain-implementation`, or `manual-only`,
plus unresolved inputs when applicable.
When an action context includes `desiredSize`, supported resize commands use
that concrete target and no longer report `needs-desired-size`.
Cache-layer command plans include bcache sysfs operations for attaching or
detaching an existing cache-set UUID, rescanning status, changing cache mode,
checking dirty data, and staging replacement cache media without silently
formatting unknown devices. bcache `operation = "rescan"` reads state,
cache-mode, dirty-data, and modeled graph relationships without changing
attachment. bcache sysfs operations require a concrete `/dev/bcache*` target;
logical cache names remain marked `needs-domain-implementation`.
Btrfs filesystem device-removal plans use Btrfs allocation inspection and
domain-specific `btrfs device remove` rendering, but the mutating command is
blocked by default until `allowPotentialDataLoss=true` is set.
bcachefs filesystem lifecycle plans render `bcachefs device resize`,
`bcachefs device add`, `bcachefs data rereplicate`, `bcachefs device remove`,
and `bcachefs scrub` commands for mounted bcachefs filesystems. Replacement is
modeled as add replacement capacity, rereplicate, then remove the old member so
the operator can review each data-preserving step.
Swapfile growth command plans render `swapoff`, `fallocate --length`, `mkswap`,
and `swapon`; block-device swap growth keeps the backing resize command
non-ready until the partition, LV, LUN, or other backing layer is selected.
Swap grow and format commands require a path-shaped target such as `/swapfile`
or `/dev/disk/by-*`. Swap label and UUID property updates render
`swaplabel --label <label> <target>` and `swaplabel --uuid <uuid> <target>` and
remain offline-required. Swap `operation = "rescan"` renders read-only
`swapon --show`, `blkid`, and graph inspection commands for activation,
capacity, label, UUID, and backing-storage refresh.
LUKS `operation = "open"` command plans render `cryptsetup open` for preserved
existing containers. Legacy preserved `operation = "create"` still maps to the
same open flow. `operation = "close"` plans render offline-policy-gated
`cryptsetup close` steps and keep the backing LUKS container intact for later
reopen. LUKS header label and subsystem property updates render
`cryptsetup config <device> --label` or `--subsystem`, while UUID updates render
`cryptsetup luksUUID <device> --uuid <uuid>`.
Disk initialization plans render policy-gated `parted mklabel` and partition
table reread commands after inspecting the target disk.
Partition create command plans render concrete `parted mkpart`, `partprobe`,
and `blockdev --rereadpt` commands when `device`, `partitionType`, `start`, and
`end` are declared.
Partition grow command plans render concrete `parted resizepart`, `partprobe`,
and `blockdev --rereadpt` commands when `device`, `partitionNumber`, and `end`
or `desiredSize` are declared.
Disk and partition `operation = "rescan"` command plans rerun `partprobe` and
`blockdev --rereadpt` against the reviewed backing disk without editing
partition geometry, then verify the refreshed table with `parted -lm`.
Filesystem shrink command plans render Btrfs allocation checks and
`btrfs filesystem resize <size> <path>` for declared target sizes. Ext shrink
plans render `findmnt`, `umount`, `e2fsck`, and `resize2fs` review steps. Ext
grow and shrink commands use a declared filesystem `device` or `disk` when
present, with source-device commands marked unresolved when the filesystem
declaration only names a mountpoint. F2FS grow command plans render
`resize.f2fs <device>` or `resize.f2fs -t <sectors> <device>` when a target
sector count is declared. XFS shrink renders manual-only migration guidance.
Filesystem check and repair command plans render `e2fsck -n`/`e2fsck -f -y`,
`xfs_repair -n`/`xfs_repair`, `btrfs check --readonly`/`--repair`,
`fsck.fat -n`/`-a`, `fsck.exfat -n`/`-p`,
`fsck.f2fs --dry-run`/`-f -y`, `bcachefs fsck -n`/`-y`, and
`ntfsfix --no-action`/`ntfsfix` for ext, XFS, Btrfs, FAT/vfat, exFAT, F2FS,
bcachefs, and NTFS declarations. Repair commands mutate metadata and remain
offline-required; NTFS repair is limited Linux-side remediation and not a
replacement for Windows `chkdsk`. Check commands are read-only but still require
a stable source device.
Btrfs filesystem rebalance plans render `btrfs balance start`; declared
`properties.balance.data`, `properties.balance.metadata`, and
`properties.balance.system` values render as `-d`, `-m`, and `-s` filters for
scoped balances.
Btrfs filesystem scrub plans render `btrfs scrub start -B <path>` and wait for
completion. ZFS pool scrub plans render `zpool scrub <pool>`.
Filesystem trim plans render `fstrim -v <mountpoint>` after inspecting discard
support and lower storage layers.
Btrfs filesystem label property updates render
`btrfs filesystem label <path> <label>` as ready commands. Ext filesystem label
updates render `e2label <device> <label>` when an explicit backing device is
declared. FAT/vfat label updates render `fatlabel <device> <label>`. NTFS label
updates render `ntfslabel <device> <label>`. exFAT label updates render
`exfatlabel <device> <label>`. F2FS label updates render
`f2fslabel <device> <label>`. XFS filesystem label updates render
`xfs_admin -L <label> <device>`. Btrfs, ext, FAT/vfat, NTFS, exFAT, and XFS UUID,
volume-ID, or volume-serial updates render
`btrfstune -U <uuid> <device>`, `tune2fs -U <uuid> <device>`,
`fatlabel -i <device> <volume-id>`, `ntfslabel --new-serial=<serial> <device>`,
`exfatlabel -i <device> <serial>`, and `xfs_admin -U <uuid> <device>` as
offline-required changes. Missing devices remain marked
`needs-domain-implementation`, while unsupported filesystem property keys are
classified as unsupported before execution.
Filesystem remount command plans render reviewed
`mount -o remount,<options> <mountpoint>` operations for `filesystems` entries
without deleting data. Missing concrete mountpoints keep remount commands
non-ready, and long-lived option changes should still be persisted through
NixOS `fileSystems`.
Filesystem rescan command plans render read-only `findmnt --json <mountpoint>`
and `disk-nix inspect <mountpoint>` commands for `filesystems` entries without
mounting, remounting, unmounting, formatting, or checking filesystem metadata.
Missing concrete mountpoints keep rescan commands non-ready.
MD RAID assemble plans render `mdadm --assemble <array> <members...>`, stop
plans render `mdadm --stop <array>`, and create plans render
destructive-policy-gated `mdadm --create` commands from explicit `level` and
`devices` fields. Missing array, level, or member fields get exact
unresolved-input markers and `/proc/mdstat` verification. MD assemble, stop,
create, grow, member add, replacement, and removal commands require an explicit
array path such as `/dev/md/root`. MD RAID `operation = "rescan"` renders
read-only `mdadm --detail --scan`, `mdadm --examine --scan`, `/proc/mdstat`,
and topology verification; a declared `/dev/md*` target adds targeted
`mdadm --detail <array>` inspection.
VDO command plans render policy-gated `vdo create` and `vdo remove` commands,
online `vdo growLogical` and `vdo growPhysical` growth steps, and
offline-required `vdo start`/`vdo stop` lifecycle steps for existing volumes.
VDO `operation = "rescan"` renders read-only `vdo status`, `vdostats`, and
graph inspection commands to refresh status and utilization without changing
activation state or capacity.
Create preflight remains non-ready until a backing device is declared. Supported
property updates render `vdo changeWritePolicy`,
`vdo enableCompression`/`disableCompression`, and
`vdo enableDeduplication`/`disableDeduplication`; unsupported VDO properties
and invalid values are blocked as unsupported before commands are rendered.
NFS export command plans use explicit `client` and `options` lifecycle fields
to render reviewed `operation = "export"`, option update, and
`operation = "unexport"` commands. Legacy export `create` and `destroy` map to
the same command plans. `operation = "rescan"` renders read-only export
inventory and graph probes. They also require a path-shaped local export target
such as `/srv/share`.
NFS client mount command plans render reviewed `operation = "mount"` commands,
`mount -o remount,<options>` option-update commands, read-only
`operation = "rescan"` mount inventory/stat probes, and
`operation = "unmount"` commands from `nfs.mounts`; legacy NFS mount `create`
and `destroy` map to the same command plans. Missing sources or path-shaped
mountpoints keep those commands non-ready.
Local filesystem command plans render reviewed `operation = "mount"` commands,
`mount -o remount,<options>` option-update commands, and
`operation = "unmount"` commands from `filesystems`/NixOS `fileSystems`-style
declarations. Mount commands require a source device and path-shaped mountpoint;
unmount commands are non-destructive but remain blocked unless offline work is
allowed by policy.
iSCSI session command plans use `target` or the lifecycle key as the target IQN
and `portal` or `metadata.portal` for reviewed `operation = "login"` and
`operation = "logout"` commands, plus `operation = "rescan"` for online session
refresh. Legacy session `create` and `destroy` map to the same login/logout
command plans. LUN command plans model host-side `operation = "attach"`,
`operation = "rescan"`, growth rescan, and `operation = "detach"`: attach,
rescan, and grow keep the broad `iscsiadm --mode session --rescan` step,
rescan/grow add per-path SCSI rescans when stable paths are declared, and
detach deletes only declared stable SCSI path devices before reloading
multipath. Legacy LUN `create` and `destroy` map to the same command plans.
Attach, rescan, grow, and detach remain non-ready until stable `device` or
`devices` paths are declared. Target-side array
provisioning or deletion must be handled outside the host plan unless a future
target adapter is added.
The capability inventory advertises iSCSI login/logout and LUN attach/detach
as host lifecycle operations, distinct from target-side LUN creation or
deletion.
Multipath map command plans render reviewed path add, remove, replacement,
growth, and `operation = "rescan"` lifecycle actions. Rescan inspects the
reviewed map with `multipath -ll`, reloads maps with `multipath -r`, and
verifies the map again; missing stable map targets keep map-specific commands
non-ready.
NVMe namespace command plans render `nvme create-ns`, `nvme attach-ns`,
explicit `operation = "rescan"` plans through `nvme ns-rescan`,
`nvme detach-ns`, and `nvme delete-ns`. Executable create plans require a
`/dev/nvme*` controller target and `desiredSize`; attach and delete flows also
require `namespaceId` plus `controllers` where detach or attach is involved.
Namespace growth is modeled as a host rescan after a controller-side namespace
size change.
LVM logical volume command plans render concrete `lvcreate` commands when a
`volumes` create action has a `vg/lv` target and `desiredSize`, and report
missing target form and size separately when either is absent. LV grow and
remove commands also require the canonical `vg/lv` target form.
`operation = "rescan"` renders read-only `lvs` and graph inspection commands
for LV size, attributes, and dependent mappings.
LVM physical volume command plans render `pvcreate`, `pvresize`, explicit
`operation = "rescan"` plans through `pvscan --cache`, and policy-gated
`pvremove` for `physicalVolumes` lifecycle declarations. Create, grow, and
remove plans require a concrete block-device path such as `/dev/disk/by-id/*`;
rescan can refresh all visible PV metadata when no path-shaped target is
declared.
LUKS keyslot and token command plans render explicit `operation = "add-key"`,
`operation = "remove-key"`, `operation = "import-token"`, and
`operation = "remove-token"` declarations as `cryptsetup luksAddKey`,
policy-blocked `luksKillSlot`, `cryptsetup token import`, and policy-blocked
`cryptsetup token remove` commands. Legacy preserved `create`/`destroy`
declarations still map to the same access-material command plans.
`luksChangeKey` is used for key-file property updates. Executable keyslot
add/change plans require a LUKS backing device and new key file; token imports
require a token JSON file; removal also requires a keyslot number or token id.
LVM thin-pool command plans render `lvcreate --type thin-pool`, `lvextend`,
read-only `lvs` rescans, and policy-gated `lvremove` commands for `thinPools`
lifecycle declarations, with separate unresolved-input markers for target form
and size. Thin-pool grow, rescan, and remove commands require the canonical
`vg/pool` target form.
LVM cache command plans render `lvconvert --type cache`, `lvconvert --uncache`,
and `lvchange --cachemode` or `--cachepolicy` commands for `lvmCaches`
lifecycle declarations. Executable attach plans require both an origin `vg/lv`
target and a cache-pool LV. `operation = "rescan"` renders read-only `lvs`
cache mode, policy, utilization, and graph inspection commands.
LVM volume group command plans render policy-gated `vgcreate` and `vgremove`
commands for `volumeGroups` lifecycle declarations, reviewed `vgextend`
commands for grow or add-device operations with an explicit physical volume,
reviewed `vgextend`, `pvmove <old-pv> <new-pv>`, and `vgreduce` replacement
workflows, and reviewed `pvmove` then `vgreduce` commands for explicit
physical-volume removal. Volume group import/export declarations render
reviewed `vgimport <vg>` and `vgexport <vg>` commands. LVM logical volume,
thin-pool, snapshot, and volume-group activation declarations render reviewed
`lvchange --activate y|n <vg/lv>` or `vgchange --activate y|n <vg>` commands.
LVM snapshot `operation = "rescan"` renders read-only `lvs` snapshot origin,
COW usage, attribute, size, and graph inspection commands before rollback or
removal decisions.
Volume group `operation = "rescan"` renders `pvscan --cache`, `vgscan`, and
`vgchange --refresh <vg>` so LVM metadata and active LV tables can be refreshed
after lower-layer path changes without recreating the VG.
Device topology operations stay non-ready until the device to add, source
device, replacement device, or device to remove is declared explicitly.
Loop-device refresh, rescan, and detach commands require `/dev/loop*` targets.
Rescan reads `losetup --json --list` and graph state without changing capacity;
grow uses `losetup -c` after backing size changes. Multipath map growth
requires a concrete map target such as `mpatha` or `/dev/mapper/mpatha`;
arbitrary logical map names remain non-ready.
ZFS pool command plans render policy-gated `zpool create` from a single
`device` or explicit `devices` vdev list, policy-gated `zpool destroy`, plus
online topology commands such as `zpool add`, `zpool replace`, `zpool remove`,
and scrub. Pool create preflight inspects declared path-like vdev entries
instead of topology keywords such as `mirror`. Pool import/export lifecycle
declarations render `zpool import`, optional
`zpool import -o readonly=on <pool>` for `readOnly = true`, and
`zpool export <pool>` command plans.
ZFS dataset command plans render reviewed `zfs create` commands, including
create-time `-o key=value` options from declared properties, and policy-gated
`zfs destroy` commands for `datasets` lifecycle declarations. Dataset
`operation = "rescan"` renders read-only `zfs list`, `zfs get`, and graph
inspection commands. Dataset and zvol rename declarations render reviewed
`zfs rename <old> <new>` commands from `operation = "rename"` plus `renameTo`.
ZFS clone promotion declarations render reviewed `zfs get origin <clone>`
preflight checks and `zfs promote <clone>` commands from
`operation = "promote"`.
Zvol command plans render `zfs create -o key=value -V` for declared create-time
properties, `zfs set volsize=...`, policy-gated `zfs destroy`, and
read-only `operation = "rescan"` inventory/property probes plus
`zfs set key=value` property reconciliation updates for `zvols` lifecycle
declarations. Zvol clone promotion uses the same reviewed `zfs promote`
lifecycle path.
Btrfs subvolume command plans render `btrfs subvolume create`, policy-gated
`btrfs subvolume delete`, reviewed path renames with `mv -- <old> <new>`, and
`btrfs property set -ts <path> ro true|false` for read-only property
declarations. Subvolume `operation = "rescan"` renders read-only
`btrfs subvolume show`, `btrfs property get -ts <path> ro`, and graph
inspection commands for the declared path.
Btrfs qgroup command plans render `btrfs qgroup create`, policy-gated
`btrfs qgroup destroy`, and `btrfs qgroup limit` for referenced and exclusive
limit declarations in `btrfsQgroups`. Qgroup `operation = "rescan"` renders
read-only quota hierarchy, referenced/exclusive usage, limits, and graph
inspection. Qgroup create, destroy, limit, and rescan plans remain non-ready
until the mounted filesystem `target` path is declared.
The capability inventory advertises qgroup create, limit-property updates,
rescan, and destroy risks so quota lifecycle changes show up in machine-readable
`capabilities --json` output.
Generic snapshot declarations render concrete `zfs snapshot` commands for
`dataset@snapshot` names and Btrfs `subvolume snapshot` commands when both the
source target and snapshot name are absolute paths. Destructive snapshot
declarations render policy-gated `zfs destroy` or `btrfs subvolume delete`
commands for the same unambiguous domains.
ZFS snapshot retention declarations render safe `zfs hold <tag> <snapshot>`
and `zfs release <tag> <snapshot>` commands from `hold` and `releaseHold`.
ZFS snapshot clone declarations render reviewed `zfs clone <snapshot> <dataset>` commands from `cloneTo`, `cloneTarget`, or `clone`.
Snapshot rename declarations render reviewed `zfs rename <snapshot> <new>` for
ZFS names and `mv -- <old> <new>` for absolute Btrfs snapshot paths.
Snapshot `operation = "rescan"` declarations render read-only ZFS
`zfs list`, `zfs get`, and `zfs holds` probes or Btrfs `subvolume show` and
read-only property probes, plus graph inspection for snapshot/source
relationships. Btrfs snapshot rescans can use `path`, `snapshotPath`, or
`snapshot-path` when the snapshot map key is a friendly name instead of the
absolute snapshot path.
ZFS snapshot rollback declarations render reviewed `zfs rollback` command
details internally, and `recursiveRollback`, `recursive`, or
`zfs.rollbackRecursive` render reviewed `zfs rollback -r` details. Apply blocks
rollback by default and requires `allowPotentialDataLoss=true` before execution.
The capability inventory advertises ZFS snapshot create, hold/release,
clone, rescan, rollback including recursive rollback review, and destroy risks
plus Btrfs snapshot create, rescan, and destroy risks.
`verificationSummary` and `verificationPlan` record read-only commands and
state checks that run after a successful `--execute` command phase or can be
used for manual review after applying a generated script. These checks re-probe
the relevant graph node and include domain-specific commands such as `findmnt`,
`lvs`, `zpool status`, `zfs list`, `btrfs filesystem usage`, `lsblk`, or
`exportfs` when the planned action has enough context.

`--script-out <path>` writes an executable bash script after policy validation
passes. The script contains the allowed command plan, manual-review notes,
unresolved-input comments for non-ready commands, and post-apply verification
commands.
`--report-out <path>` writes the JSON report before returning blocked-policy or
not-ready or failed-execution results, so automation can archive the exact
decision record even when `apply` exits nonzero.

## Validation

`validate` emits the same dry-run report as `apply`, including command and
verification plans, but blocked policy is not a CLI failure:

```sh
disk-nix validate --spec ./examples/lifecycle-update.json
disk-nix validate --spec ./examples/lifecycle-update.json --json
disk-nix validate --spec ./examples/simple-root.json --script-out ./disk-nix-apply.sh
disk-nix validate --spec ./examples/lifecycle-update.json --report-out ./validate-report.json
```

Use `validate` for CI, NixOS activation-style checks, and review workflows that
need structured blocked-action details without failing before the report can be
consumed. `--script-out` still requires every planned action to be policy
allowed, because blocked reports do not have a runnable review script.
`--report-out` always writes the JSON report when parsing and report
preparation succeed.
