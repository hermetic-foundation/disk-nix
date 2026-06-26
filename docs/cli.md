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
swap, zram, LUKS, device-mapper, LVM, VDO, MD RAID, Btrfs, ZFS, exFAT, NTFS,
iSCSI, LUNs, NFS, bcache, multipath, NVMe controllers/namespaces, and loop
devices.
Nodes are merged by id when multiple probe adapters report complementary
information. NVMe probing keeps controller, subsystem, transport, namespace id,
namespace UUID, NGUID, EUI-64, ANA state, LBA format, formatted LBA descriptor,
namespace feature/capacity counters, controller capabilities/capacity, sector
size, capacity, usage, and SMART/health telemetry from
`nvme list --output-format=json`, `nvme id-ns -o json`,
`nvme id-ctrl -o json`, and `nvme smart-log -o json` when available.
exFAT probing uses `tune.exfat` and `dump.exfat` when available to add visible
label metadata, GUID, serial, tool version, sector, cluster, size,
used-cluster, and free-space metadata beyond generic `blkid` fields. NTFS
probing uses `ntfsinfo -m` when
available to add device/volume state, volume name/version, serial,
sector/cluster sizing, index block size, MFT record size, MFT zone/location
metadata, and allocated size. F2FS probing uses
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

Each report also includes a structured `category` in JSON and human output:
`none`, `missing-tool`, `permission-denied`, `command-failed`,
`parse-failed`, or `inaccessible-data`. Use this with `status` to decide
whether installing tooling, changing privileges, or treating the topology as
degraded is the right response.
Reports also include `remediation` hints. Missing-tool reports point to tool
installation, concrete adapter tools, and likely Nix packages for
`services.disk-nix.toolPackages`, including PATH and `ENOENT` failures;
permission reports call out privileged metadata reads plus adapter-specific
surfaces such as device-mapper, LVM, ZFS, iSCSI, NVMe, multipath, MD RAID, and
VDO state, including root-only and superuser barriers; parse failures ask for
raw command-output fixtures and tool versions; inaccessible-data reports point
to missing kernel surfaces, services, imports, sessions, or mountpoints.

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
shape with top-level `version`, `spec`, and `apply` objects. The current
supported contract is version `1`; omitted versions are accepted as version
`1`, and unsupported future versions are rejected before planning. It includes
the planner's filesystem fields, including filesystem `operation`, `device`,
mount
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
See [compatibility.md](compatibility.md) for the versioning, migration, JSON,
CLI text, NixOS option, and generated-artifact compatibility policy.

## Spec Migration

`migrate` renders a reviewable migration report and normalized spec without
planning or applying storage changes:

```sh
disk-nix migrate --spec ./examples/lifecycle-update.json
disk-nix migrate --spec ./examples/lifecycle-update.json --json
```

For the current version `1` contract, migration is metadata-only. It adds
explicit `version = 1` fields to direct specs and NixOS-module wrapper specs
when they are omitted, validates the migrated document with the planner parser,
and reports warnings that no lifecycle semantics were rewritten. Future or
conflicting versions are rejected instead of being guessed.

## Focused Views

Focused commands filter the graph for common workflows:

```sh
disk-nix devices
disk-nix partitions
disk-nix filesystems
disk-nix complex-filesystems
disk-nix btrfs
disk-nix bcachefs
disk-nix zfs
disk-nix volumes
disk-nix pools
disk-nix snapshots
disk-nix mappings
disk-nix dm
disk-nix encryption
disk-nix cache
disk-nix lvm
disk-nix vdo
disk-nix multipath
disk-nix nvme
disk-nix raid
disk-nix loop
disk-nix backing-files
disk-nix swap
disk-nix zram
disk-nix iscsi
disk-nix luns
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
disk-nix btrfs --json
disk-nix bcachefs --json
disk-nix zfs --json
disk-nix volumes --json
disk-nix pools --json
disk-nix snapshots --json
disk-nix mappings --json
disk-nix dm --json
disk-nix encryption --json
disk-nix cache --json
disk-nix lvm --json
disk-nix vdo --json
disk-nix multipath --json
disk-nix nvme --json
disk-nix raid --json
disk-nix loop --json
disk-nix backing-files --json
disk-nix swap --json
disk-nix zram --json
disk-nix iscsi --json
disk-nix luns --json
disk-nix nfs --json
disk-nix mounts --json
disk-nix network-storage --json
disk-nix ids --json
disk-nix usage --json
```

The JSON form returns a focused `StorageGraph` subgraph. It includes matching
nodes plus directly related neighbors and the relationship edges that connect
them, so automation can see immediate backing devices, mountpoints, members,
exports, snapshots, and imported targets without fetching the full topology.

Use these commands for:

- `devices`: disks, partitions, dm devices, LVM objects, VDO, RAID, zvols,
  cache devices, multipath devices, NVMe subsystems/controllers/namespaces, loop
  devices, zram, and swap, including model/vendor, transport, rotational, NVMe
  model/firmware/namespace
  geometry, SCSI host/channel/target/LUN address, generic device, transport,
  LU/WWN identity, queue state, SMART health, smartctl provenance,
  self-test/offline collection state, ATA error-log and self-test log counts,
  temperature, power-on, capacity, sector, SCSI grown-defect counts, and ATA
  reallocation/pending-sector/offline-uncorrectable raw, normalized, worst,
  threshold, and failure fields,
  `lsblk` sector/I/O
  alignment, discard, scheduler, zoned-device, DAX, and hotplug metadata,
  partition table/number,
  filesystem type, zram compression/memory accounting, loop
  backing inode, backing major/minor, offset/autoclear/partition-scan metadata,
  multipath path host/major-minor, parsed
  SCSI coordinates, split dm/checker/online state details, and extra path
  flags, MD RAID member number/major/minor/raid-device/state, active swap
  state/type/priority, and udev by-id/by-path links, encoded labels,
  filesystem UUID sub-identifiers, filesystem block-size/last-block geometry,
  partition table metadata, major/minor numbers, and device-mapper flags when
  probes expose them
- `partitions`: partition nodes with size, PARTUUID, path, filesystem type,
  `blkid` signature details, partition number, raw start/end geometry,
  normalized byte offsets, type/name, and flags when probes expose them
- `filesystems`: regular filesystems, Btrfs filesystems/subvolumes/snapshots,
  bcachefs filesystems, ZFS datasets/snapshots, and NFS exports, with selected
  filesystem metadata details such as `blkid` version/block-size/usage, exFAT
  label, GUID/serial, volume length, FAT and cluster-heap layout, root cluster,
  and raw plus derived cluster geometry, NTFS volume identity, version, cluster
  sizing, MFT record and zone/location metadata, F2FS block usage, valid
  inode/node counts, segment layout, section/zone geometry, log sizing, version,
  and overprovisioning metadata, XFS source, allocation-group, inode, data,
  naming format, log type/sizing, realtime type/geometry, and metadata feature
  details,
  bcachefs external/internal UUID, superblock magic, version/upgrade state,
  member-device, mounted usage, and filesystem/member data-type byte
  accounting, Btrfs Data/Metadata/System allocation profiles and byte counts,
  and ext state/features, reserved and overhead block accounting, block/inode
  group geometry, first-block and RAID stride/stripe layout hints, mount/check
  counters, timestamps, directory hash settings, default mount options,
  lifetime writes, journal identity, first/last filesystem error telemetry, and
  checksum metadata when probes expose them
- `complex-filesystems`: Btrfs, bcachefs, and ZFS pools, vdevs, datasets,
  zvols, subvolumes, snapshots, qgroups, and member devices, including size,
  used/free capacity, utilization, backing/member counts, allocation profiles,
  qgroup hierarchy and limits, bcachefs superblock and member accounting, ZFS
  health/vdev state, and ZFS compression/dedup/checksum/copies/cache/sync/record-size,
  quota/reservation/encryption, and POSIX metadata policy properties when
  probes expose them
- `btrfs`: Btrfs filesystems, subvolumes, snapshots, and qgroups, including
  size, used/free capacity, utilization, mount targets, backing relationships,
  allocation profiles, subvolume IDs/generations/parentage, snapshot UUID
  lineage, qgroup hierarchy and limits, and member device write/read/flush I/O,
  corruption, and generation error counters
- `bcachefs`: bcachefs filesystems and member devices, including external and
  internal UUIDs, mount target, version/upgrade state, online reservation,
  member count, data-type accounting, member labels, member state, member free
  and capacity counters, and one-hop member relationships
- `zfs`: ZFS pools, vdevs, datasets, snapshots, and zvols, including pool
  health/state, capacity, dedup ratio, fragmentation, altroot, ashift,
  autotrim, autoexpand, autoreplace, bootfs, cachefile, delegation, failmode,
  listsnapshots, multihost, status/action advisories, scan/error summaries,
  pool aggregate READ/WRITE/CKSUM counters, vdev roles and error counters,
  dataset compression, dedup, checksum, copies, sync, cache policy, record
  size, quota, reservation, encryption, key status, POSIX metadata policy,
  snapshot user references and hold tags, zvol volume size, origin, and
  pool/dataset/snapshot child
  relationships when `zpool` and `zfs` probes expose them
- `volumes`: logical storage objects such as LVM, Btrfs, bcachefs, ZFS, zvols,
  LUNs, and exports, including LVM origin/pool/data metadata, activation state,
  activation locality/exclusivity, role, layout, health, tags, device-mapper
  path, parent, read-ahead, table state, thin-pool fullness behavior, metadata
  size, and cache or writecache status, MD RAID level/state, iSCSI attached
  disks, NFS server/export details, and ZFS zvol `volsize` when reported by
  `zfs list`
- `pools`: storage pools and grouping layers such as LVM volume groups, thin
  pools, Btrfs filesystems/qgroups, bcachefs filesystems, ZFS pools/vdevs, and
  MD RAID arrays,
  including ZFS health/vdev role/state/error counters, LVM extent/free extent
  counts, PV/LV/snapshot counts, permissions, allocation policy, lock/system-id,
  and metadata-area counters, Btrfs qgroup hierarchy and limits, and MD RAID
  metadata version, name, level, state, device counts, and event counters where
  probes expose them
- `snapshots`: snapshot objects across LVM, Btrfs, and ZFS, including known
  source relationships, LVM origin/pool/data metadata, Btrfs subvolume IDs,
  generation, creation generation, parent IDs, top-level, parent UUIDs, and
  received UUIDs, and ZFS user-reference, hold tag, compression, and encryption
  details
- `mappings`: encryption headers/keyslots/tokens, device-mapper, LVM, VDO,
  RAID, multipath, and cache layers, including LUKS active/keyslot/token
  counts, keyslot priorities/ciphers/PBKDF cost and keyslot area metadata,
  digest identifiers and digest hash/iteration metadata, token-to-keyslot
  bindings, token metadata such as TPM PCR/hash hints, header area/epoch/flag
  details, data-segment cipher/offset/length
  details, dm name/UUID, major/minor numbers, open/segment counters, mapper
  table targets, live target status, sanitized dm-crypt table details, parsed
  linear/striped/thin/cache/snapshot table fields, cache/thin-pool/snapshot
  status usage counters, LVM segment data/metadata device mappings,
  thin-pool discard/zeroing/transaction details, cache segment policy/settings,
  VDO segment compression/dedup/write-policy details, multipath WWID/size,
  parsed SCSI path coordinates, and split path state, VDO backing device,
  logical/physical size, mode, configured and active write policy, index/cache
  sizing, data-reduction settings, and block
  accounting, loop backing/offset/read-only/direct-I/O settings, and bcache
  role/cache-set/tuning details such as UUID, label, state, running flag,
  block/bucket sizing, btree cache size, available cache percentage, cache mode,
  discard, cache read races, I/O errors, written/metadata-written accounting,
  readahead, sequential cutoff, priority stats, writeback delay, and writeback
  rate when probes expose them
- `dm`: device-mapper maps, including dm name/UUID, major/minor numbers,
  open and segment counters, table target payloads, live status target
  payloads, sanitized dm-crypt table fields, cache/thin/snapshot status
  counters, and one-hop backing relationships
- `encryption`: LUKS/dm-crypt mappings and header metadata, including cipher,
  active/in-use state, keyslot/token counts and ids, LUKS version, epoch,
  metadata/keyslot area sizes, flags, subsystem, keyslot priority/cipher/PBKDF
  details, keyslot area offsets/lengths, AF stripes, digest identifiers,
  digest hash/iteration metadata, token-to-keyslot bindings, token metadata
  such as TPM PCR/hash hints, and data-segment details
- `cache`: bcache devices/cache sets, LVM cache/writecache metadata, bcachefs
  member-device cache accounting, and ZFS cache vdevs, including cache mode,
  policy, dirty/writeback data, LVM cache block totals, dirty blocks,
  hit/miss and promotion/demotion counters, writecache total/free/block-size/
  error counters, backing device, cache-set identity, state/running flags,
  cache-set average key size, root usage, journal delay, error thresholds,
  available cache percentage, discard, I/O errors,
  written/metadata-written accounting, priority stats, congestion thresholds,
  writeback-rate tuning, and vdev state
- `lvm`: LVM physical volumes, volume groups, logical volumes, segments, thin
  pools, snapshots, and cache/writecache layers, including data and metadata
  percentages, active state and locality/exclusivity, device-mapper paths,
  parent links, read-ahead, table suspension/live/inactive state, host and
  historical flags, PV format/device-id/extent/metadata-area state, VG
  permissions/allocation/lock/system-id and extent/PV/LV/snapshot counts,
  origin/pool relationships, thin-pool fullness behavior, segment device
  mappings, stripe/reshape/range metadata, segment integrity settings, detailed
  VDO segment tuning, cache policy, LVM RAID sync/recovery/integrity status,
  health, tags, and backing/member counts when `pvs`, `vgs`, `lvs`, or
  `dmsetup` expose them
- `vdo`: native VDO volumes and LVM VDO segment metadata, including backing
  device, logical and physical size, used/free/percent utilization columns,
  status/stat counters, operating mode, recovery progress, configured and
  active write policy, LVM VDO compression and index state, byte-normalized
  used size, saving counters, index/cache sizing, compression, deduplication,
  version/release data, and block accounting when probes expose them
- `multipath`: multipath maps and their backing paths, including WWID, dm
  device, vendor/product, raw size, normalized byte capacity, features,
  hardware handler, write protection, path count, host path, SCSI
  host/channel/id/LUN coordinates, major/minor, path-group policy, priority,
  group status, dm/checker/online state columns, extra path flags, and raw path
  state when `multipath -ll` exposes them
- `nvme`: NVMe subsystems, controllers, and namespaces, including path, serial,
  model, firmware, namespace index/id, generic namespace path, subsystem NQN,
  host NQN, controller, controller id, transport, address, fabrics endpoint,
  path state, ANA state, namespace capacity, LBA format, maximum LBA, sector
  size, formatted LBA descriptor, namespace feature/capacity counters,
  controller capabilities/capacity, physical size, used bytes, free bytes,
  utilization, temperature, spare capacity, media errors, unsafe shutdowns,
  error-log count, and power-on telemetry when `nvme list-subsys -o json`,
  `nvme list -o json`, `nvme id-ns -o json`, `nvme id-ctrl -o json`, and
  `nvme smart-log -o json` expose them
- `raid`: MD RAID arrays and member devices, including array UUID, scan-level
  metadata version, array name, spare count, device hints, active detail
  metadata version, level, state, size, raid, total, array, active, working,
  failed, spare, and degraded device counts, event counters, chunk/layout
  details, preferred minor, consistency policy, rebuild, reshape, resync, and
  check progress, intent bitmap, persistence, bitmap detail, timestamps,
  `/proc/mdstat` runtime state, device health strings, live recovery/resync
  progress, finish and speed estimates, bitmap state, and per-member number,
  major/minor, raid-device, slot, flags, and state when `/proc/mdstat`,
  `mdadm --detail --scan`, `mdadm --examine --scan`, or
  `mdadm --detail` exposes them
- `loop`: loop devices and backing files/devices, including backing path,
  backing inode, backing major/minor, offset, size limit, logical sector size,
  major/minor, autoclear, partition-scan, read-only, and direct-I/O settings
  when `losetup --json` exposes them
- `backing-files`: file-backed storage origins, including path, size,
  utilization, loop backing metadata, consumer counts, and one-hop loop or
  swapfile relationships
- `swap`: active swap devices and files plus zram swap devices, including type,
  priority, active state, size, used bytes, free bytes, utilization, zram
  compression algorithm, compressed/data/total memory accounting, memory limit
  and high-water use, compression ratio, and backing relationship when
  `/proc/swaps` exposes them
- `zram`: generated compressed swap devices, including logical disk size,
  active data bytes, compressed bytes, total memory, memory limit, memory used,
  high-water memory use, compression algorithm, stream count, compression
  ratio, mountpoint, and swap activation marker when `zramctl` exposes them
- `iscsi`: configured iSCSI nodes, sessions, targets, and LUNs, including node
  portals, node startup policy, interface, leading-login, CHAP method/user
  hints, current and persistent session portals plus parsed portal
  address/port/TPGT fields, target portal group tag, connection/session state,
  connection CID/local/peer addresses, interface identity, negotiated transfer
  parameters, target IQNs, LUN sizes, SCSI host/channel/id coordinates,
  generic devices, transport, LU/WWN identity, queue state, attached disk
  path/state, table-level path identity for attached LUN block devices,
  session to target imports, target-contained LUN counts, and
  LUN-to-block-device backing relationships when `iscsiadm --mode node -P 1`,
  `iscsiadm --mode session -P 3`, or `lsscsi` exposes them
- `luns`: host-visible LUN nodes, including path, size, transport, generic
  device, SCSI host/channel/target/LUN coordinates, queue state, attached disk
  state, and one-hop target or backing-block relationships
- `nfs`: NFS server exports and client mounts, including exportfs path,
  client, server/export split, export option state such as rw/ro, sync,
  subtree checking, security flavor, squash flags, FSID, NFS protocol version,
  transport and mount transport, client/server addresses, port/mount address,
  read/write transfer sizes, timeout/retransmit settings, local locking,
  lookup cache, FS-Cache, capability flags, transfer multipliers, directory
  transfer/block sizing, RPC security flavor identifiers, age, and
  export-to-client mount relationships when `exportfs -v`, `findmnt`, or NFS
  mount probes expose them
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
  metadata detail data, including bcache role/backing-device/set/state, UUID,
  cache mode, replacement policy, block/bucket sizing, available cache
  percentage, dirty data, cache read races, I/O errors, writeback percentage,
  `blkid` signature
  details, ext superblock state, block/inode geometry, RAID layout hints,
  reservation, mount/check, and journal details, LVM layout, health,
  thin/cache/writecache
  capacity/status counters, NTFS volume geometry and MFT record sizing, F2FS
  block usage,
  valid inode/node counts, segment layout, section/zone geometry, log sizing,
  bcachefs filesystem and member-device capacity plus data-type accounting,
  Btrfs allocation class profiles and byte counts, VDO backing, logical/physical
  size, used/free capacity, data-reduction, cache/index, and block-accounting
  details, NVMe namespace details, loop mapping details, and active swap
  state/type/priority when probed

## Inspect

`inspect` finds nodes by id, path, name, UUID, PARTUUID, label, serial, WWN, or
property key/value:

```sh
disk-nix inspect /dev/nvme0n1
disk-nix inspect /
disk-nix inspect tank/home
disk-nix inspect 7420d5e2-2f0f-4709-a1d1-61a9116412f8
disk-nix inspect / --depth 3
```

The text form prints identity fields, capacity details, properties, and
relationship context for matched nodes. `--depth` controls how far relationship
expansion walks from the matched node: `0` includes only the matched node, `1`
is the default direct-neighbor view, and larger values include deeper stacked
storage context. Capacity output includes size plus used, free, allocated, and
utilization percentage when the node exposes those fields. The JSON form
returns a subgraph containing matched nodes, neighbor nodes within the requested
depth, and the relationship edges between them:

```sh
disk-nix inspect / --json
disk-nix inspect / --depth 3 --json
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
groups, MD RAID, multipath, Btrfs, NVMe namespaces, backing files, and cache
devices.

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
- `dependencyOrder`
- `topologyComparison` when `--probe-current` is set
- `actions`

Each action includes the target id, operation, risk class, destructive flag,
typed context, and optional advice with non-destructive alternatives.
`dependencyOrder` explains the current planner ordering for every action,
including build/mutate/teardown phase, lower-first or upper-first direction,
collection layer rank, inferred `dependsOn` and `unblocks` edges, and ordering
notes. It reflects the current coarse layer ordering plus conservative edges
derived from declared action identities. When `--probe-current` is set, direct
and multi-hop relationships in the probed storage graph also add dependency
edges between matched planned actions, including lower-to-upper growth paths and
reversed upper-to-lower teardown paths. The topology comparison summary reports
the number of graph-derived dependency edges as `graphDependencyEdgeCount` and
mixed-direction graph-path warnings as `graphDependencyConflictCount`.
Dry-run reports keep those conflicts visible for review, but `--execute`
refuses to run while the count is non-zero.

With `--probe-current`, the CLI also probes the current host and adds
`topologyComparison`, including matched target counts, missing target counts,
size diagnostics, filesystem type conflicts, matching filesystem format types
and swap format targets that still require destructive review, and
already-satisfied property, size, or remount option checks. Mount actions are
also compared with
`mount.source` when the current graph has mountpoint data, absent mountpoints
stay actionable as mount-required work, unmount actions are suppressed when the
mountpoint is absent, remount actions treat declared options as a required
subset of current mount options, LVM
activation and deactivation actions are compared with `lvm.active` where that
metadata is available, absent LVM activation targets stay actionable and absent
deactivation targets are suppressed as already inactive, LUKS open and close
actions are compared with `cryptsetup.active`, absent mapper opens stay
actionable with LUKS warnings, absent mapper closes are suppressed as already
satisfied, LUKS
label/subsystem/UUID property actions are reconciled
against probed identity and `cryptsetup.luks-*` header metadata, and LUKS
keyslot/token removal actions are compared with `cryptsetup.luks-keyslots` and
`cryptsetup.luks-tokens` header metadata from the matched container; absent
LUKS containers for keyslot/token removal remain actionable with header review
warnings. Loop-device
create/destroy actions are compared with
`loop.back-file` mapping metadata, device-mapper destroy actions are compared
with current mapper presence and `dm.open-count` metadata, multipath destroy
actions are compared with current map presence plus WWID or dm map metadata,
bcache detach actions are compared with current bcache target presence,
dirty-data, cache-mode, and cache-set metadata, LVM cache detach actions are
compared with origin LV cache/writecache metadata, and absent LVM cache origins
remain actionable with metadata review warnings. Btrfs subvolume destroy
actions are compared with concrete absolute-path presence plus subvolume id,
generation, parent, top-level, and UUID metadata, LUN attach/detach and NVMe
namespace attach/detach actions are compared with concrete host-visible path
matches, NFS export actions are compared with
`nfs.export-client` and `nfs.export-option-*` properties, absent NFS exports
remain actionable as export work instead of generic missing targets, NFS
unexport actions are suppressed when the export is absent, VDO destroy actions
are compared with current VDO presence plus operating-mode, size, backing-device,
write-policy, and LVM VDO utilization metadata, VDO start actions are compared
with `vdo.operating-mode`, VDO stop actions are compared with
explicitly stopped, not-running, or inactive `vdo.operating-mode` values, MD
assemble actions are compared with `md.state`, `md.degraded-devices`, and
`md.failed-devices`, ZFS dataset and zvol destroy actions are compared with
concrete target presence and ZFS metadata, generic snapshot destroy actions are
compared with concrete ZFS snapshot names or absolute Btrfs snapshot paths,
ZFS snapshot rollback actions are compared with the concrete rollback snapshot
instead of only the target dataset, ZFS pool import actions are compared with
`zfs.state` and `zfs.health`, LVM volume-group import/export actions are
compared with `lvm.vg-exported`, and iSCSI login/logout actions are compared
with current session state across all matching target/session nodes when
metadata is available. Safe already-satisfied grow, shrink, device-mapper destroy,
multipath destroy, bcache detach, iSCSI login/logout, LVM
activation/deactivation, LVM volume-group import/export, LUKS open, LUKS close,
LUKS keyslot/token removal, loop create/destroy, LUN attach/detach, NVMe
namespace attach/detach, mount, unmount, remount, NFS export/unexport, VDO
destroy, VDO start, VDO stop, backing-file create/grow, MD assemble, Btrfs
subvolume destroy, ZFS dataset/zvol destroy, generic snapshot destroy, ZFS
pool import, LVM cache detach, and property actions that have no warning diagnostics are suppressed from the actionable plan and counted as
`topologyComparison.summary.suppressedActionCount`; inactive LVM objects,
still-active LVM deactivation targets, still-exported LVM volume groups,
inactive LUKS open targets, active LUKS close targets, still-present LUKS
keyslots/tokens selected for removal, loop devices mapped to different backing
files, backing-file create targets with different or unknown current size,
still-mapped loop detach targets, present device-mapper removal targets,
LVM rename sources whose destinations are also absent, device-mapper rename
sources whose destinations are also absent, present multipath flush targets,
absent LUN attach paths, visible LUN detach paths, present bcache detach
targets, still-attached or absent LVM cache origins, absent multipath path-add
maps, absent NVMe namespace attach paths, visible NVMe namespace detach paths,
present VDO destroy targets, non-normal VDO start modes, running VDO stop
targets, present
Btrfs subvolume destroy targets, present ZFS dataset/zvol destroy targets,
absent ZFS dataset/zvol rename destinations, present ZFS or Btrfs snapshot
destroy targets, missing ZFS/Btrfs snapshot clone sources, missing or present
ZFS/Btrfs snapshot rename sources, missing or present ZFS rollback snapshots,
degraded or failed MD arrays,
degraded ZFS pools, mountpoints using a different source, currently mounted unmount targets,
published unexport targets, export client/option differences, or known iSCSI
targets without a logged-in session and logout targets that still have a
logged-in session stay actionable with a warning diagnostic.

## Apply Evaluation

Apply defaults to policy evaluation and dry-run command planning:

```sh
disk-nix apply --spec ./examples/lifecycle-update.json
disk-nix apply --spec ./examples/lifecycle-update.json --json
disk-nix apply --spec ./examples/lifecycle-update.json --probe-current --json
disk-nix apply --spec ./examples/simple-root.json --script-out ./disk-nix-apply.sh
disk-nix apply --spec ./examples/lifecycle-update.json --report-out ./apply-report.json
disk-nix apply --spec ./examples/lifecycle-update.json --receipt-out ./apply-receipt.json
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
- `toolRequirements`
- `commandPlan`
- `verificationSummary`
- `verificationPlan`
- `executionResults` when `--execute` runs commands
- `recoveryActions` for blocked, non-ready, or failed execution reports
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
Blocked, non-ready, and failed reports include `recoveryActions` with
machine-readable action kinds, read-only inspection commands, and operator notes.
These actions are advisory: they steer operators toward current-state capture,
policy review, missing-input resolution, validation before resume, and preserving
recovery points after partial mutation. Failed risky actions also include
`domain-recovery` guidance with domain-specific read-only inspection commands
where the failed action context is concrete, such as ZFS/Btrfs snapshot
lifecycle checks, ZFS pool import/export/device/property changes, and ZFS
dataset or zvol rename/grow/promote/property updates. Concrete risky failures
also emit `roll-forward-review` guidance that starts with a fresh
`disk-nix apply --probe-current --json` dry run against the original spec and
`rollback-review` guidance for domains with inspectable rollback preconditions,
such as ZFS rollback points, ZFS/Btrfs snapshot lifecycle changes, LVM snapshot
merges, VG device migration, LVM VG/volume/thin/PV changes, cache lifecycle
changes, ZFS pool/dataset/zvol lifecycle changes, swap signature/activation
changes, filesystem format/grow/shrink/check/repair/mount/remount/unmount/trim
updates, disk and partition-table create/grow/rescan changes, LUKS
mapper/header/keyslot/token changes, MD RAID member replacement, NVMe namespace
changes, iSCSI session login/logout, VDO lifecycle changes, multipath map
changes, loop-device, backing-file, and device-mapper map changes, and
NFS export and client mount changes, and host-side LUN detach. These commands
remain read-only or manual-only; disk-nix does not automatically roll back
storage because rollback safety is domain- and topology-specific.
`commandSummary` reports total steps, total commands, mutating commands,
manual-review steps, and readiness counts so callers can gate automation before
iterating detailed commands.
`toolRequirements` summarizes the external executables referenced by rendered
command and verification plans, including command counts, mutating counts,
verification counts, phases, PATH availability, an availability message, and
per-tool remediation hints such as the likely Nix package or
`services.disk-nix.toolPackages` adjustment. Automation can compare this
inventory with host policy or `probe-status` output before allowing mutation.
`--execute` refuses to run when any rendered required tool is missing, before
invoking the first storage command.
When policy allows an action, `commandPlan` records the planned commands,
whether each command mutates system state, and notes that still require
operator review. Each command also reports readiness:
`ready`, `needs-desired-size`, `needs-domain-implementation`, or `manual-only`,
plus unresolved inputs when applicable.
When an action context includes `desiredSize`, supported resize commands use
that concrete target and no longer report `needs-desired-size`.
Cache-layer command plans include bcache sysfs operations for attaching or
detaching an existing cache-set UUID, rescanning status, changing cache mode,
checking dirty data, updating `bcache.set-*` cache-set tuning fields, and
replacing cache media only when the replacement device and explicit
`cacheSetUuid` are declared. bcache `operation = "rescan"` reads state,
cache-mode, dirty-data, and modeled graph relationships without changing
attachment. bcache device sysfs operations require a concrete `/dev/bcache*`
target; logical cache names can declare `target = "/dev/bcacheN"`,
`path = "/dev/bcacheN"`, or `device = "/dev/bcacheN"` to make attach, detach,
rescan, replacement, and device-local property commands ready. Cache-set sysfs
property updates require `cacheSetUuid`, `cache-set-uuid`, or equivalent
metadata so commands can write `/sys/fs/bcache/<set>/<field>`. Logical cache
declarations without concrete identities remain marked
`needs-domain-implementation`. With current-topology probing, concrete absent
bcache detach actions are suppressed as already satisfied, while present
targets stay actionable with a warning that includes dirty data, cache mode,
and cache-set UUID when available. Cache property comparison also reconciles
declared `cacheMode`/`cachePolicy` aliases and `bcache.set-*` cache-set
properties with bcache `bcache.*` and LVM cache `lvm.*` metadata, normalizing
dashed cache-mode values before suppressing already-satisfied updates.
Loop-device command plans require a `/dev/loop*` target for grow, rescan, and
detach operations. Logical loop declarations can supply that target with
`target` or `path`; `device` is reserved for the backing file or block device
used by create plans. Current-topology probing suppresses loop create actions
only when the loop device already maps the declared backing file and suppresses
destroy/detach actions only when the loop device is already absent; different
existing backing files stay actionable with a warning.
Backing-file command plans use `backingFiles` declarations for file-backed
storage origins. `operation = "create"` first renders `test ! -e` for the
reviewed path and then `truncate --size` for the requested sparse file size,
so existing images are not overwritten by the generated command sequence.
`operation = "rescan"` renders read-only `stat`, `du`, and graph inspection
commands. `operation = "grow"` renders `truncate --size` only when a concrete
file path and desired size are declared; logical names can supply the file path
with `target` or `path`.
Current-topology probing suppresses backing-file create only when the existing
file already has the declared size, suppresses grow when current size already
satisfies the desired size, and keeps mismatched existing files actionable with
a warning because the generated create command refuses to overwrite them.
Device-mapper command plans use `dmMaps` declarations for map refreshes and
reviewed mapper renames or removals. `operation = "rescan"` renders `dmsetup info`, `dmsetup deps -o devname`, `dmsetup table`, `dmsetup status`, and graph
inspection commands when a concrete `/dev/mapper/*` or `/dev/dm-*` target is
declared. `operation = "rename"` renders `dmsetup rename` with an
offline-required policy gate because dependent consumers must move to the new
mapper name. `operation = "destroy"` or `destroy = true` renders destructive
`dmsetup remove` after identity, dependency, and status inspection. With
current-topology probing, mapper renames are suppressed when the old mapper is
absent and the new mapper path exists, absent mapper removals are suppressed as
already satisfied, and present maps remain actionable with a warning, including
the current `dm.open-count` when available. Use domain-specific LUKS, LVM, VDO,
multipath, or cache teardown when those layers own the mapper.
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
or `/dev/disk/by-*`; logical swap names can declare it with `target`, `path`,
or `device`. Swap label and UUID property updates render
`swaplabel --label <label> <target>` and
`swaplabel --uuid <uuid> <target>` and remain offline-required. Numeric priority
updates render reviewed `swapoff <target> && swapon --priority <priority> <target>` reactivation commands. Swap property comparison maps declared label,
UUID, and priority aliases onto probed swap identity and signature metadata
before suppressing already-satisfied updates.
Swap `operation = "rescan"` renders read-only `swapon --show`, `blkid`, and graph
inspection commands for activation,
capacity, label, UUID, and backing-storage refresh.
Swap `operation = "deactivate"` renders `swapoff` while keeping the signature
intact. Swap `operation = "destroy"` renders `swapoff` and `wipefs --all`, so
it remains blocked until destructive policy is explicitly allowed. With
`--probe-current`, inactive or absent swap teardown requests are suppressed,
while active swap targets stay actionable with size, usage, type, or priority
diagnostics. Swap format targets that already have swap metadata, or that match
another current node kind, warn with the current metadata while keeping `mkswap`
destructive and review-gated.
Plain zram declarations render read-only `zramctl`, `swapon --show`, and
`disk-nix zram` commands for compressed swap size, algorithm, memory use, and
activation review. Explicit zram `operation = "rescan"` uses the same inventory
path as a named refresh action. With current-topology probing, declared zram
algorithm, stream count, disk size, memory limit, compression ratio, and swap
priority properties are compared against discovered `/dev/zram*` and active
swap metadata; already-satisfied generated-state updates are suppressed, while
drift remains actionable for offline-required NixOS `zramSwap` reconciliation.
LUKS `operation = "open"` command plans render `cryptsetup open` for preserved
existing containers. With current-topology probing, active mappers are
suppressed from the actionable plan, inactive matched or absent mappers remain
warnings, and absent mapper closes are suppressed as already satisfied. Legacy
preserved `operation = "create"` still maps to the same open flow. LUKS
`operation = "format"` and `preserveData = false` compare the
declared backing `device` against current topology and report existing LUKS
header metadata or other matched node kinds, but destructive format commands
remain reviewable. `operation = "close"` plans render offline-policy-gated
`cryptsetup close` steps and keep the backing LUKS container intact for later
reopen. LUKS header label and subsystem property updates render
`cryptsetup config <device> --label` or `--subsystem`, while UUID updates render
`cryptsetup luksUUID <device> --uuid <uuid>`. Current-topology probing matches
these property actions by backing device and suppresses already-satisfied
label, subsystem, and UUID declarations after comparing probed LUKS identity and
header metadata. Logical LUKS declaration keys can declare the concrete mapper
name with `target`, `mapperName`, `mapper`, or `name`.
Disk initialization plans render policy-gated `parted mklabel` and partition
table reread commands after inspecting the target disk. With
`--probe-current`, disk create is suppressed when the matched physical disk
already reports the requested partition table label; mismatched labels, unknown
labels, and matched non-disk nodes remain actionable warnings because `mklabel`
can hide existing metadata.
Partition create command plans render concrete `parted mkpart`, `partprobe`,
and `blockdev --rereadpt` commands when `device`, `partitionType`, `start`, and
`end` are declared. With `--probe-current`, create is suppressed when the target
partition already exists and any declared desired size matches; size conflicts,
unknown current size, and matched non-partition nodes remain actionable warnings.
Partition grow command plans render concrete `parted resizepart`, `partprobe`,
and `blockdev --rereadpt` commands when `device`, `partitionNumber`, and `end`
or `desiredSize` are declared. When `--probe-current` is used, parseable
byte-sized `end` values are reconciled against the current partition size so
already-satisfied growth is suppressed; percentage ends such as `100%` still
render reviewable geometry changes.
Disk and partition `operation = "rescan"` command plans rerun `partprobe` and
`blockdev --rereadpt` against the reviewed backing disk without editing
partition geometry, then verify the refreshed table with `parted -lm`.
Filesystem declarations with `preserveData = false` render destructive
`mkfs.*` command plans for common filesystem types when a concrete `device` or
`disk` is declared. Mountpoint-only format declarations remain non-ready rather
than guessing a backing block device. With `--probe-current`, matching current
filesystem types are reported, but format commands remain reviewable because
they overwrite metadata.
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
completion. bcachefs filesystem scrub plans render `bcachefs scrub <path>`.
ZFS pool scrub plans render `zpool scrub <pool>`.
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
`needs-domain-implementation`. With current-topology probing, filesystem label,
UUID, FAT volume-ID, NTFS serial, and exFAT serial property declarations are
compared against probed identity fields and filesystem metadata aliases with
hex identity normalization before already-satisfied updates are suppressed,
while unsupported filesystem property keys are classified as unsupported before
execution.
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
array path such as `/dev/md/root`; logical declarations can provide that path
through `target` or `device`. Current-topology probing suppresses MD create
only when the matched array is already cleanly active; degraded, inactive, or
wrong-kind matches stay actionable with warnings. It suppresses MD stop when
the array is already absent or inactive; present active, unknown-state, or
wrong-kind matches stay actionable with warnings. Member add is suppressed
when probed `MemberOf` edges show the device is already in the array, and
member removal is suppressed when the device is already absent from the matched
array. Member replacement is suppressed only when the old member is absent and
the replacement member is attached. MD RAID
`operation = "rescan"` renders read-only `mdadm --detail --scan`,
`mdadm --examine --scan`, `/proc/mdstat`, and topology verification; a
declared `/dev/md*` target adds targeted `mdadm --detail <array>` inspection.
Current-topology probing suppresses an assemble action only when the current
array is active or clean and has zero degraded and failed devices.
VDO command plans render policy-gated `vdo create` and `vdo remove` commands,
online `vdo growLogical` for `desiredSize`, explicit `vdo growPhysical` for
`physicalSize`, and offline-required `vdo start`/`vdo stop` lifecycle steps for
existing volumes. With current-topology probing, `vdo start` actions are
suppressed only when the current operating mode is already `normal`; `vdo stop`
actions are suppressed only when the current operating mode explicitly reports
stopped, not-running, or inactive, or when the volume is absent; absent starts
remain actionable with VDO warnings; `vdo growLogical` actions are suppressed
when current byte size or VDO logical-size metadata already satisfies `desiredSize`;
below-target, unknown, or absent current targets stay actionable with VDO grow
diagnostics. `vdo remove`/destroy actions are suppressed only when the VDO
volume is already absent and otherwise warn with available operating-mode,
logical/physical size, backing-device, write-policy, or LVM VDO utilization
metadata.
VDO `operation = "rescan"` renders read-only `vdo status`, `vdostats`, and
graph inspection commands to refresh status and utilization without changing
activation state or capacity.
Create preflight remains non-ready until a backing device is declared; with
current-topology probing, create targets that already have VDO metadata or match
another current node kind warn without suppressing the destructive create.
Supported property updates render `vdo changeWritePolicy`,
`vdo enableCompression`/`disableCompression`, and
`vdo enableDeduplication`/`disableDeduplication`; unsupported VDO properties
and invalid values are blocked as unsupported before commands are rendered.
With current-topology probing, declared VDO write policy, compression, and
deduplication properties are compared against native `vdo.*` metadata and LVM
`lvm.vdo-*` metadata. Compression and deduplication boolean values are
normalized across spellings such as `enabled`, `true`, `disabled`, and `0`, so
already-satisfied changes are suppressed and real mismatches remain visible as
warnings.
Logical VDO volume names can declare the concrete VDO name with `target`.
NFS export command plans use explicit `client` and `options` lifecycle fields
to render reviewed `operation = "export"`, option update, and
`operation = "unexport"` commands. Legacy export `create` and `destroy` map to
the same command plans. `operation = "rescan"` renders read-only export
inventory and graph probes. They also require a path-shaped local export target
such as `/srv/share`; logical export names can declare it through `target` or
`path`. With current-topology probing, already published exports are suppressed
only when the client and requested option subset already match the graph;
absent exports remain actionable with an export-required diagnostic.
NFS client mount command plans render reviewed `operation = "mount"` commands,
`mount -o remount,<options>` option-update commands, read-only
`operation = "rescan"` mount inventory/stat probes, and
`operation = "unmount"` commands from `nfs.mounts`; legacy NFS mount `create`
and `destroy` map to the same command plans. Missing sources or path-shaped
mountpoints keep those commands non-ready. Logical NFS mount names can declare
the concrete local path with `mountpoint`. With current-topology probing,
absent NFS mountpoints stay actionable as mount-required work.
Local filesystem command plans render reviewed `operation = "mount"` commands,
`mount -o remount,<options>` option-update commands, and
`operation = "unmount"` commands from `filesystems`/NixOS `fileSystems`-style
declarations. Mount commands require a source device and path-shaped mountpoint;
unmount commands are non-destructive but remain blocked unless offline work is
allowed by policy.
iSCSI session command plans use `target` or the lifecycle key as the target IQN
and `portal` or `metadata.portal` for reviewed `operation = "login"` and
`operation = "logout"` commands, plus `operation = "rescan"` for online session
refresh with read-only `lsscsi -t -s` host LUN inventory. Legacy session
`create` and `destroy` map to the same login/logout command plans. LUN command
plans model host-side `operation = "attach"`,
`operation = "rescan"`, growth rescan, and `operation = "detach"`: attach,
rescan, and grow keep the broad `iscsiadm --mode session --rescan` step,
capture host-visible LUN transport and size through `lsscsi -t -s`, add
per-path SCSI rescans when stable paths are declared, and reload multipath.
Detach captures the same `lsscsi` inventory, then deletes only declared stable
SCSI path devices before reloading multipath. Legacy LUN `create` and
`destroy` map to the same command plans.
Attach, rescan, grow, and detach remain non-ready until stable paths are
declared through `device`, `path`, `devices`, `paths`, or `devicePaths`.
Target-side array
provisioning or deletion must be handled outside the host plan unless a future
target adapter is added.
The capability inventory advertises iSCSI login/logout, LUN attach/detach, and
NVMe namespace attach/detach as host lifecycle operations, distinct from
target-side LUN provisioning or destructive namespace deletion.
Multipath map command plans render reviewed path add, remove, replacement,
growth, map flush, and `operation = "rescan"` lifecycle actions. Rescan
inspects the reviewed map with `multipath -ll`, captures host-visible SCSI path
transport and size with `lsscsi -t -s`, reloads maps with `multipath -r`, and
verifies the map again. Growth captures the same `lsscsi` inventory before
`multipathd resize map`. `operation = "destroy"` or `destroy = true`
renders offline-gated `multipath -f <map>` after map inspection; missing
stable map targets keep map-specific commands non-ready. With
current-topology probing, absent map flushes are suppressed as already
satisfied and present maps remain actionable with a warning, including the
current WWID or dm map name when available. Path add is suppressed when probed
`Backs` edges show the path already feeds the map, and path removal is
suppressed when the path is already absent from the matched map. Path add
against an absent map stays actionable with a warning so the map can be reviewed
or recreated before membership changes run.
NVMe namespace command plans render `nvme create-ns`, standalone
`operation = "attach"` plans through `nvme attach-ns`, explicit
`operation = "rescan"` plans through `nvme ns-rescan` with `nvme list-subsys`
path inventory, standalone `operation = "detach"` plans through
`nvme detach-ns`, and destructive delete plans through `nvme detach-ns` plus
`nvme delete-ns`. Attach, detach, grow, rescan, and delete flows capture
subsystem path state through `nvme list-subsys --output-format=json`.
Executable create
plans require a `/dev/nvme*` controller path from the declaration key,
`target`, `path`, or `device`, plus `desiredSize`; attach, detach, and delete
flows also require `namespaceId` plus `controllers` where attachment state is
changed. When a declaration needs both executable commands and topology
reconciliation, use `target` or `path` for the controller and `device` for the
host-visible namespace block path such as `/dev/nvme0n1`.
Namespace growth is modeled as a host rescan after a controller-side namespace
size change.
LVM logical volume command plans render concrete `lvcreate` commands when a
`volumes` create action has a `vg/lv` target and `desiredSize`, and report
missing target form and size separately when either is absent. LV grow and
remove commands also require the canonical `vg/lv` target form from the
declaration key, `target`, or `path`.
`operation = "rescan"` renders read-only `lvs` and graph inspection commands
for LV size, attributes, and dependent mappings. Current-topology probing
suppresses `volumes` create actions when the matched LVM logical volume already
exists and any declared desired size exactly matches; existing LVs with
different or unknown size remain actionable with warnings that recommend grow
or shrink lifecycle instead of recreating data. LVM logical volume, thin-pool,
and volume-group rename actions are suppressed when the old identity is absent
and the new destination already exists with the expected LVM kind.
LVM physical volume command plans render `pvcreate`, `pvresize`, explicit
`operation = "rescan"` plans through `pvscan --cache`, and policy-gated
`pvremove` for `physicalVolumes` lifecycle declarations. Create, grow, and
remove plans require a concrete block-device path such as `/dev/disk/by-id/*`
from the declaration key, `target`, `path`, or `device`; rescan can refresh all
visible PV metadata when no path-shaped target is declared. Current-topology
probing suppresses `operation = "create"` only when the matched target already
has LVM PV metadata; a matched non-PV device, duplicate PV, or missing PV stays
actionable with a warning before any destructive `pvcreate` review.
LUKS keyslot and token command plans render explicit `operation = "add-key"`,
`operation = "remove-key"`, `operation = "import-token"`, and
`operation = "remove-token"` declarations as `cryptsetup luksAddKey`,
policy-blocked `luksKillSlot`, `cryptsetup token import`, and policy-blocked
`cryptsetup token remove` commands. Legacy preserved `create`/`destroy`
declarations still map to the same access-material command plans.
`luksChangeKey` is used for key-file property updates, and keyslot `priority`
updates render
`cryptsetup config <device> --key-slot <slot> --priority <prefer|normal|ignore>`.
Executable keyslot add/change plans require a LUKS backing device and new key
file; priority updates require a LUKS backing device, keyslot number, and one of
`prefer`, `normal`, or `ignore`; token imports require a token JSON file;
removal also requires a keyslot number or token id. Logical keyslot and token
names can declare concrete slot/token ids with `keySlot`, `key-slot`, `slot`,
`tokenId`, `token-id`, or `token`. With current-topology probing, removal is
suppressed only when the matched LUKS container no longer lists the keyslot or
token id; keyslot priority changes are suppressed when probed metadata already
matches. Present entries stay actionable with warnings that include keyslot
priority, cipher, PBKDF, token type, or token keyslot binding metadata when
available.
LVM thin-pool command plans render `lvcreate --type thin-pool`, `lvextend`,
read-only `lvs` rescans, and policy-gated `lvremove` commands for `thinPools`
lifecycle declarations, with separate unresolved-input markers for target form
and size. Thin-pool grow, rescan, and remove commands require the canonical
`vg/pool` target form from the declaration key, `target`, or `path`.
Current-topology probing suppresses thin-pool create actions only when the
matched object is an LVM thin pool and any declared desired size exactly
matches; wrong-kind or size-mismatched targets stay planned with warnings.
LVM cache command plans render `lvconvert --type cache`, `lvconvert --uncache`,
and `lvchange --cachemode` or `--cachepolicy` commands for `lvmCaches`
lifecycle declarations. Executable attach plans require both an origin `vg/lv`
target and a cache-pool LV. `operation = "rescan"` renders read-only `lvs`
cache mode, policy, utilization, and graph inspection commands. With
current-topology probing, detach actions are suppressed only when the matched
origin LV no longer reports cache or writecache metadata; still-attached
origins remain actionable with warnings that include cache pool, mode, policy,
dirty blocks, and utilization when available.
LVM volume group command plans render policy-gated `vgcreate` and `vgremove`
commands for `volumeGroups` lifecycle declarations, reviewed `vgextend`
commands for grow or add-device operations with an explicit physical volume,
reviewed `vgextend`, `pvmove <old-pv> <new-pv>`, and `vgreduce` replacement
workflows, and reviewed `pvmove` then `vgreduce` commands for explicit
physical-volume removal. Volume group import/export declarations render
reviewed `vgimport <vg>` and `vgexport <vg>` commands. Current-topology probing
suppresses a volume-group create when the VG already exists without exported,
partial, or missing-PV metadata. It also suppresses a volume-group import when
the VG is already visible and not marked `lvm.vg-exported`, and suppresses a
volume-group export when the VG is already marked exported; existing exported,
partial, or missing-PV create targets, still-exported imports, and
still-imported exports stay actionable with a warning. LVM
logical volume, thin-pool, snapshot, and volume-group activation declarations render reviewed
`lvchange --activate y|n <vg/lv>` or `vgchange --activate y|n <vg>` commands.
With current-topology probing, already-active logical-volume, thin-pool, and
snapshot activation actions are suppressed from the actionable plan.
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
logical map names can declare that target through `target` or `device`.
ZFS pool command plans render policy-gated `zpool create` from a single
`device` or explicit `devices` vdev list, including declared pool `properties`
as create-time `-o key=value` options. They also render policy-gated
`zpool destroy`, plus online topology commands such as `zpool add`,
`zpool replace`, `zpool remove`, and scrub. Pool create preflight inspects
declared path-like vdev entries instead of topology keywords such as `mirror`.
Current-topology probing suppresses pool create only when the matched pool is
already visible with `zfs.state = ONLINE` and `zfs.health = ONLINE`; degraded,
faulted, or wrong-kind matches stay actionable with warnings. Pool property
comparison maps declarations such as `autotrim`, `autoExpand`, `altroot`, and
`cachefile` onto `zfs.*` or pool-scoped `zfs.pool-*` metadata before suppressing
already-satisfied `zpool set` updates. Pool import/export lifecycle
declarations render `zpool import`, optional
`zpool import -o readonly=on <pool>` for `readOnly = true`, and
`zpool export <pool>` command plans. Current-topology probing suppresses a pool
import only when the current pool is visible with `zfs.state = ONLINE` and
`zfs.health = ONLINE`; degraded or faulted pools stay actionable with a warning.
ZFS dataset command plans render reviewed `zfs create` commands, including
create-time `-o key=value` options from declared properties, and policy-gated
`zfs destroy` commands for `datasets` lifecycle declarations. Dataset
`operation = "rescan"` renders read-only `zfs list`, `zfs get`, and graph
inspection commands. With current-topology probing, concrete `pool/name`
dataset create actions are suppressed when the matched node is already a ZFS
dataset, and destroy actions are suppressed only when the dataset is already
absent. Existing non-dataset matches stay actionable for create with warnings;
present datasets stay actionable for destroy with warnings that include
mountpoint, quota, reservation, encryption, key status, origin, usage, or
compression metadata when available. Dataset and zvol rename declarations render
reviewed `zfs rename <old> <new>` commands from `operation = "rename"` plus
`renameTo`. Current-topology probing suppresses rename actions when the old ZFS
object is absent and the new dataset or zvol name already exists with the
expected kind.
ZFS clone promotion declarations render reviewed `zfs get origin <clone>`
preflight checks and `zfs promote <clone>` commands from
`operation = "promote"`. Current-topology probing suppresses promote actions
when the matched dataset or zvol no longer reports `zfs.origin`; clones that
still report an origin stay actionable with warnings. Dataset and zvol
declarations may use logical attribute names when `target` or `path` supplies
the concrete `pool/name` ZFS object.
Zvol command plans render `zfs create -o key=value -V` for declared create-time
properties, `zfs set volsize=...`, policy-gated `zfs destroy`, and
read-only `operation = "rescan"` inventory/property probes plus
`zfs set key=value` property reconciliation updates for `zvols` lifecycle
declarations. Current-topology probing suppresses concrete `pool/name` zvol
create actions when the matched node is already a ZFS zvol and any declared
desired size is already satisfied, and suppresses destroy actions only when the
zvol is already absent. Existing non-zvol matches or existing zvols with
different or unknown current size stay actionable for create with warnings;
dataset and zvol property comparison maps declarations onto probed `zfs.*`
metadata, including mountpoint, compression, volsize, cache, and common on/off
properties, before suppressing already-satisfied `zfs set` updates.
present zvols stay actionable for destroy with warnings that include volsize,
origin, usage, reservation, encryption, or compression metadata when available.
Zvol clone promotion uses the same reviewed `zfs promote` lifecycle path.
Btrfs subvolume command plans render `btrfs subvolume create`, policy-gated
`btrfs subvolume delete`, reviewed path renames with `mv -- <old> <new>`, and
`btrfs property set -ts <path> ro true|false` for read-only property
declarations. Subvolume `operation = "rescan"` renders read-only
`btrfs subvolume show`, `btrfs property get -ts <path> ro`, and graph
inspection commands for the declared path. With current-topology probing,
concrete absolute-path subvolume create actions are suppressed when the matched
node is already a Btrfs subvolume, and destroy actions are suppressed only when
the subvolume is already absent. Existing non-subvolume path matches stay
actionable for create with warnings; present subvolumes stay actionable for
destroy with warnings that include subvolume id, generation, parent, top-level,
or UUID metadata when
available. Logical subvolume names remain actionable unless a graph node
actually matches them.
Btrfs qgroup command plans render `btrfs qgroup create`, policy-gated
`btrfs qgroup destroy`, and `btrfs qgroup limit` for referenced and exclusive
limit declarations in `btrfsQgroups`. Qgroup `operation = "rescan"` renders
read-only quota hierarchy, referenced/exclusive usage, limits, and graph
inspection. With current-topology probing, concrete numeric qgroup destroy
actions such as `0/257` are suppressed only when the qgroup is already absent;
concrete numeric qgroup create actions are suppressed when the matched node is
already a Btrfs qgroup, and qgroup referenced/exclusive limit properties are
suppressed when declared aliases match probed `btrfs.max-*` metadata. Existing
non-qgroup matches stay actionable for create with warnings; present qgroups
stay actionable for destroy with warnings that include referenced and exclusive
usage, limits, parent, or child metadata when available. Logical qgroup names
remain actionable unless a graph node actually matches them.
Qgroup create, destroy, limit, and rescan plans remain non-ready until the
mounted filesystem path is declared through `target`, `path`, or `mountpoint`.
The capability inventory advertises qgroup create, limit-property updates,
rescan, and destroy risks so quota lifecycle changes show up in machine-readable
`capabilities --json` output.
Generic snapshot declarations render concrete `zfs snapshot` commands for
`dataset@snapshot` names and Btrfs `subvolume snapshot` commands when both the
source target and snapshot name are absolute paths. Destructive snapshot
declarations render policy-gated `zfs destroy` or `btrfs subvolume delete`
commands for the same unambiguous domains. With current-topology probing,
already-absent concrete ZFS snapshot names and absolute Btrfs snapshot paths
are suppressed; present snapshots stay actionable with warnings that include
available ZFS user-reference/usage metadata or Btrfs subvolume metadata.
ZFS snapshot retention declarations render safe `zfs hold <tag> <snapshot>`
and `zfs release <tag> <snapshot>` commands from `hold` and `releaseHold`.
With current-topology probing, existing hold tags suppress duplicate hold
actions and absent hold tags suppress no-op release actions. Snapshot views
surface probed ZFS hold tags in metadata details.
Snapshot clone declarations render reviewed `zfs clone <snapshot> <dataset>`
commands for ZFS snapshots and
`btrfs subvolume snapshot <snapshot-path> <clone-path>` for absolute Btrfs
snapshot paths from `cloneTo`, `cloneTarget`, or `clone`. Btrfs clone
declarations with `readOnly = true` render `btrfs subvolume snapshot -r`.
Clone and rollback plans remain non-ready until the declaration resolves to a
concrete ZFS snapshot name or, for clone, an absolute Btrfs snapshot path. With
current-topology probing, clone compares the source snapshot identity or
absolute Btrfs snapshot path; missing sources warn, and available sources are
reported with snapshot metadata. Friendly Btrfs clone declarations can use
`snapshotPath` or `snapshot-path` to provide the concrete source path.
Snapshot rename declarations render reviewed `zfs rename <snapshot> <new>` for
ZFS names and `mv -- <old> <new>` for absolute Btrfs snapshot paths. Friendly
snapshot keys remain non-ready for rename until `name`, `snapshotName`, `path`,
or `snapshotPath` supplies the concrete snapshot identity. With
current-topology probing, rename compares the concrete source snapshot name or
absolute Btrfs snapshot path; missing and present rename sources both stay
actionable with warning diagnostics and present sources include snapshot
metadata.
Snapshot `operation = "rescan"` declarations render read-only ZFS
`zfs list`, `zfs get`, and `zfs holds` probes or Btrfs `subvolume show` and
read-only property probes, plus graph inspection for snapshot/source
relationships. Snapshot declarations can use `name`, `snapshotName`, or
`snapshot-name` when the map key is a friendly name instead of the concrete
snapshot identity. Btrfs snapshot rescans can also use `path`, `snapshotPath`,
or `snapshot-path` when the snapshot map key is a friendly name instead of the
absolute snapshot path.
ZFS snapshot rollback declarations render reviewed `zfs rollback` command
details internally, and `recursiveRollback`, `recursive`, or
`zfs.rollbackRecursive` render reviewed `zfs rollback -r` details. Apply blocks
rollback by default and requires `allowPotentialDataLoss=true` before execution.
With current-topology probing, rollback compares the concrete ZFS snapshot
identity and warns when the rollback point is missing or available; available
rollback points still stay actionable because rollback remains potential data
loss.
The capability inventory advertises ZFS snapshot create, hold/release, clone,
rescan, rollback including recursive rollback review, and destroy risks plus
Btrfs snapshot create, clone, rename, rescan, and destroy risks.
`verificationSummary` and `verificationPlan` record read-only commands and
state checks that run after a successful `--execute` command phase or can be
used for manual review after applying a generated script. These checks re-probe
the relevant graph node and include domain-specific commands such as `findmnt`,
`lvs`, `zpool status`, `zfs list`, `btrfs filesystem usage`, `lsblk`, or
`exportfs` when the planned action has enough context.

`--script-out <path>` writes an executable bash script after policy validation
passes and graph dependency conflict checks are clean. The script contains the
allowed command plan, manual-review notes, unresolved-input comments for
non-ready commands, and post-apply verification commands.
`--report-out <path>` writes the JSON report before returning blocked-policy or
not-ready or failed-execution results, so automation can archive the exact
decision record even when `apply` exits nonzero.
`--receipt-out <path>` writes a JSON receipt that wraps the same report with
receipt version, command name, spec path, probe-current flag, execute flag, and
generation timestamp. Prefer receipts for apply journals and recovery handoff
where the report must stay tied to the invocation that produced it.

## Validation

`validate` emits the same dry-run report as `apply`, including command and
verification plans, but blocked policy is not a CLI failure:

```sh
disk-nix validate --spec ./examples/lifecycle-update.json
disk-nix validate --spec ./examples/lifecycle-update.json --json
disk-nix validate --spec ./examples/simple-root.json --script-out ./disk-nix-apply.sh
disk-nix validate --spec ./examples/lifecycle-update.json --report-out ./validate-report.json
disk-nix validate --spec ./examples/lifecycle-update.json --receipt-out ./validate-receipt.json
```

Use `validate` for CI, NixOS activation-style checks, and review workflows that
need structured blocked-action details without failing before the report can be
consumed. `--script-out` still requires every planned action to be policy
allowed and graph dependency conflicts to be resolved, because blocked or
conflicting reports do not have a runnable review script.
`--report-out` always writes the JSON report when parsing and report
preparation succeed. `--receipt-out` writes the same receipt envelope as apply,
with `command = "validate"` and `executeRequested = false`.
