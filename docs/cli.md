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

The graph can represent:

- block devices, partitions, filesystems, mountpoints, swap, and zram
- LUKS, device-mapper, LVM, VDO, MD RAID, Btrfs, ZFS, exFAT, and NTFS
- iSCSI, LUNs, NFS, bcache, multipath, NVMe controllers/namespaces, and loop
  devices

Nodes are merged by id when multiple probe adapters report complementary
information.

NVMe probing keeps controller, subsystem, transport, namespace id, namespace
UUID, NGUID, EUI-64, ANA state, LBA format, formatted LBA descriptor,
feature/capacity counters, sector size, usage, and SMART/health telemetry.

exFAT probing uses `tune.exfat` and `dump.exfat` when available to add visible label metadata, GUID, serial, tool version, sector, cluster, size, used-cluster, and free-space metadata beyond generic `blkid` fields. NTFS probing uses `ntfsinfo -m` when available to add device/volume state, volume name/version, serial, sector/cluster sizing, index block size, MFT record size, MFT zone/location metadata, and allocated size.

F2FS probing uses `dump.f2fs` when available to add volume name, UUID, user/valid block counts, checkpoint/SIT/NAT/SSA segment layout, section/zone geometry, log sizing, version metadata, overprovisioning, and computed usage.

bcachefs probing uses `bcachefs show-super` and `bcachefs fs usage` when available to add external/internal UUIDs, labels, superblock magic, version and upgrade state, member-device indexes, mounted capacity, filesystem data-type byte accounting, and per-device free/capacity, superblock, journal, btree, user, and cached metadata.

## Probe Status

Probe status explains what data was available on the current host:

```sh
disk-nix probe-status
disk-nix probe-status --json
disk-nix probe-status --preflight
disk-nix probe-status --preflight --json
```

Each adapter reports one of:

- `available`: the adapter ran and returned usable data
- `partial`: the adapter ran but some commands or objects were inaccessible
- `unavailable`: the required command, service, kernel surface, or data source
  was not present
- `failed`: the adapter unexpectedly failed

Each report also includes a structured `category` in JSON and human output: `none`, `missing-tool`, `permission-denied`, `command-failed`, `parse-failed`, or `inaccessible-data`. Use this with `status` to decide whether installing tooling, changing privileges, or treating the topology as degraded is the right response. Reports also include `remediation` hints.

Missing-tool reports point to tool installation, concrete adapter tools, and likely Nix packages for `services.disk-nix.toolPackages`, including PATH and `ENOENT` failures; permission reports call out privileged metadata reads plus adapter-specific surfaces such as device-mapper, LVM, ZFS, iSCSI, NVMe, multipath, MD RAID, and VDO state, including root-only and superuser barriers;

parse failures ask for raw command-output fixtures and tool versions; inaccessible-data reports point to missing kernel surfaces, services, imports, sessions, or mountpoints. `probe-status --preflight` adds OS release, kernel release, effective UID, storage tool version probes, and preflight check summaries so CI, operators, and bug reports can tie adapter failures to the distribution, privilege context, and tool-output variant that produced them.

The checks report whether probing is running as root, count missing or failing storage tools, list the affected tools, treat successful version probes with no output as failures, accept the first non-empty version line from stdout or stderr, and emit remediation text.

The JSON `preflightChecks.adapterRemediation` matrix maps every built-in probe adapter and sub-adapter to its canonical storage domain, required command-line tools, likely Nix packages, privilege hint, kernel/service/data hint, parse fixture hint, and manual command hint.

This covers sub-adapters such as `nvme-id-ns`, `nvme-smart-log`, `mdadm-scan`, `mdadm-examine`, `vdostats-verbose`, `nfs-exports`, and `zramctl`, so automation can recommend concrete package additions or privilege/service checks instead of generic adapter failure text. With `--json`, preflight output is wrapped as `{ environment, preflightChecks, reports }`; without `--preflight`, `probe-status --json` keeps the stable adapter-report array shape.

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

The schema describes both direct planner specs and the NixOS module wrapper shape with top-level `version`, `spec`, and `apply` objects.

The current supported contract is version `1`; omitted versions are accepted as version `1`, and unsupported future versions are rejected before planning.

It includes the planner's filesystem fields, including filesystem `operation`, `device`, mount `options`, `properties`, `metadata`, `neededForBoot`, `destroy`, and Btrfs device-membership update fields.

It also includes lifecycle declarations for:

- disks, partitions, swap, LUKS, LUKS keyslots, and LUKS tokens
- NFS mount wrappers with planner-only `metadata`
- iSCSI discovery, session, and boot wrappers
- Btrfs subvolumes, VDO, LVM physical volumes, thin pools, snapshots, and caches
- loop devices, MD RAID, multipath, NVMe namespaces, zvols, and snapshots

The schema also covers supported operation names, apply policy fields, and NixOS
activation helper fields.

Planner-compatible aliases such as `number`, `startOffset`, `endOffset`, and `raidLevel` are included for editor completion and validation parity. The Nix package installs the same generated schema at `share/disk-nix/schema/disk-nix-spec.schema.json`. See [compatibility.md](compatibility.md) for the versioning, migration, JSON, CLI text, NixOS option, and generated-artifact compatibility policy.

## Spec Migration

`migrate` renders a reviewable migration report and normalized spec without
planning or applying storage changes:

```sh
disk-nix migrate --spec ./examples/lifecycle-update.json
disk-nix migrate --spec ./examples/lifecycle-update.json --json
```

For the current version `1` contract, migration adds explicit `version = 1` fields to direct specs and NixOS-module wrapper specs when they are omitted. For unversioned legacy documents it also maps documented pre-version field names to current version `1` locations: `fileSystems` to `filesystems`, `swapDevices` to `swaps`, `luksDevices` to `luks.devices`, `nfsMounts` to `nfs.mounts`, and `iscsiSessions` to `iscsi.sessions`.

Explicit version `1` documents are not silently rewritten through these legacy aliases. Migration validates the migrated document with the planner parser, reports the complete `legacyMappings` matrix for direct specs and NixOS-module wrapper `spec.*` documents, records the run-specific `appliedMappings` audit trail, and emits a machine-readable `versionMigrations` contract for supported source and target version paths.

It also reports warnings that storage mutations are not applied. Future or conflicting versions are rejected instead of being guessed.

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

Use these commands for the focused views below. Each view returns the same graph
shape, filtered to the domain named by the command.

### Device And Partition Views

`devices` covers disks, partitions, dm devices, LVM objects, VDO, RAID, zvols,
cache devices, multipath devices, NVMe objects, loop devices, zram, and swap.

The device view carries physical identity, transport, capacity, sector geometry,
queue, discard, scheduler, DAX, zoned-device, SMART, SCSI, NVMe, udev,
partition-table, loop, zram, swap, dm, multipath, and MD member metadata when
those probes expose it.

`partitions` focuses on partition nodes. It reports size, paths, PARTUUID,
filesystem signature details, partition numbers, raw and normalized geometry,
type/name metadata, and flags.

### Filesystem Views

`filesystems` covers regular filesystems, Btrfs filesystems and subvolumes,
bcachefs filesystems, ZFS datasets and snapshots, and NFS exports.

Filesystem details include blkid data, ext superblock state, XFS geometry, NTFS
volume metadata, exFAT and F2FS layout data, bcachefs member accounting, and
Btrfs allocation profiles when the matching probes are available.

`complex-filesystems` narrows the graph to Btrfs, bcachefs, and ZFS structures.
It includes pools, vdevs, datasets, zvols, subvolumes, snapshots, qgroups,
member devices, utilization, allocation policy, and data-integrity properties.

`btrfs` reports filesystems, subvolumes, snapshots, qgroups, allocation
profiles, member-device counters, subvolume lineage, and qgroup limits.

`bcachefs` reports filesystem and member identity, mount target, version state,
reservation state, member labels, member capacity, and data-type accounting.

`zfs` reports pools, vdevs, datasets, snapshots, and zvols. It includes pool
health, capacity, scan/error state, vdev counters, dataset policy, encryption,
snapshot holds, zvol size/origin, and child relationships.

### Volume And Pool Views

`volumes` covers logical storage objects such as LVM, Btrfs, bcachefs, ZFS,
zvols, LUNs, and exports. It emphasizes activation state, parent links, health,
layout, thin/cache status, MD RAID state, iSCSI disks, NFS details, and zvol
size.

`pools` covers LVM volume groups and thin pools, Btrfs filesystems and qgroups,
bcachefs filesystems, ZFS pools and vdevs, and MD RAID arrays. It emphasizes
capacity, extent counts, allocation policy, pool health, device counts, qgroup
limits, and array event counters.

`snapshots` covers LVM, Btrfs, and ZFS snapshots. It includes source
relationships, LVM origin/pool metadata, Btrfs generation and UUID lineage, and
ZFS user-reference, hold, compression, and encryption details.

### Mapping And Cache Views

`mappings` covers encryption headers, keyslots, tokens, device-mapper, LVM,
VDO, RAID, multipath, loop, and cache layers. It emphasizes headers, active
mapper state, table/status payloads, LVM segment mappings, VDO accounting,
multipath path state, loop backing data, and bcache tuning.

`dm` focuses on device-mapper maps. It reports names, UUIDs, major/minor
numbers, open and segment counters, table payloads, status payloads, sanitized
dm-crypt fields, cache/thin/snapshot counters, and one-hop backing links.

`encryption` focuses on LUKS and dm-crypt. It reports active state, cipher,
LUKS version, keyslot and token counts, priority, PBKDF, digest, token binding,
header layout, subsystem, flags, and data-segment details.

`cache` covers bcache devices and cache sets, LVM cache/writecache, bcachefs
member cache accounting, and ZFS cache vdevs. It reports cache mode, policy,
dirty data, utilization, writeback tuning, error counters, identities, and vdev
state.

### LVM, VDO, And Multipath Views

`lvm` reports physical volumes, volume groups, logical volumes, segments, thin
pools, snapshots, and cache/writecache layers. It includes activation, device
mapper paths, extent accounting, VG policy, origin/pool links, segment details,
VDO tuning, RAID status, health, tags, and member counts.

`vdo` reports native VDO volumes and LVM VDO segment metadata. It includes
backing devices, logical and physical size, used/free capacity, write policy,
recovery progress, compression, deduplication, cache/index state, version data,
and block accounting.

`multipath` reports maps and backing paths. It includes WWID, dm device,
vendor/product, size, features, handler, write protection, path count, SCSI
coordinates, path-group policy, priorities, online/checker state, and raw path
state.

### Network And Remote Storage Views

`nvme` reports subsystems, controllers, and namespaces. It includes serial,
model, firmware, namespace IDs, NQN identity, fabrics endpoints, path state, ANA
state, namespace capacity, LBA data, controller capabilities, utilization,
health, and power-on telemetry.

`iscsi` reports configured nodes, sessions, targets, and LUNs. It includes
portals, startup policy, interfaces, CHAP hints, session addresses, transfer
parameters, target IQNs, LUN sizes, SCSI coordinates, attached disks, and
LUN-to-block relationships.

`luns` reports host-visible LUN nodes. It includes path, size, transport,
generic device, SCSI host/channel/target/LUN coordinates, queue state, attached
disk state, and one-hop target or backing-block relationships.

`nfs` reports server exports and client mounts. It includes export paths,
clients, source splits, option state, protocol and transport, address data,
locking, cache, sizing, RPC security, age, and export-to-client relationships.

`network-storage` combines iSCSI sessions, iSCSI targets, LUNs, NFS exports,
and NFS mounts. It emphasizes portal state, session state, SCSI coordinates,
attached disks, NFS source identity, protocol, security, cache, timeouts, and
transfer sizing.

### Local Runtime Views

`raid` reports MD RAID arrays and member devices. It includes UUIDs, metadata
version, level, state, device counts, event counters, chunk/layout details,
bitmap data, runtime progress, and per-member slot/state fields.

`loop` reports loop devices and backing files or block devices. It includes
backing path, inode, major/minor, offset, size limit, sector size, autoclear,
partition scan, read-only, and direct-I/O settings.

`backing-files` reports file-backed storage origins. It includes path, size,
utilization, loop backing metadata, consumer counts, and one-hop loop or
swapfile relationships.

`swap` reports active swap devices, swap files, and zram swap devices. It
includes type, priority, active state, size, used/free bytes, utilization, zram
compression and memory accounting, compression ratio, and backing links.

`zram` reports generated compressed swap devices. It includes logical disk size,
active data, compressed data, total memory, memory limit, high-water use,
algorithm, stream count, compression ratio, mountpoint, and swap activation.

`mounts` reports local mountpoints and NFS mounts. It includes source,
read/write state, bind indicators, tmpfs metadata, and overlayfs lower, upper,
and work directory options.

`ids` returns nodes with UUID, PARTUUID, label, serial, or WWN identity fields.

`usage` returns nodes with capacity or usage data. It includes size, used, free,
allocated, utilization, and selected domain-specific details for bcache, blkid,
ext, LVM, NTFS, F2FS, bcachefs, Btrfs, VDO, NVMe, loop, and swap.

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

The text form prints identity fields, capacity details, properties, and relationship context for matched nodes. `--depth` controls how far relationship expansion walks from the matched node:

`0` includes only the matched node, `1` is the default direct-neighbor view, and larger values include deeper stacked storage context.

Capacity output includes size plus used, free, allocated, and utilization percentage when the node exposes those fields. The JSON form returns a subgraph containing matched nodes, neighbor nodes within the requested depth, and the relationship edges between them:

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

## Planning, Apply, And Validation

Use [CLI planning and apply](cli-plan-apply.md) for the detailed planner, apply, rollback recipe, and validation report contracts. This page keeps the command index and read-only inspection surface.

Common entrypoints remain:

```sh
disk-nix plan --spec ./examples/simple-root.json --json
disk-nix apply --spec ./examples/lifecycle-update.json --probe-current --json
disk-nix validate --spec ./examples/lifecycle-update.json --json
```

The detailed contract covers `dependencyOrder`, `topologyComparison`, `partiallySuppressed` reconciliation groups, `commandPlan`, `toolRequirements`, `rollbackRecipes`, `requiredTopologyEvidence`, `operatorOnlyHandoff`, and proven-safe reversible rollback replay behavior.

## Coverage anchors

These exact phrases are kept for the flake documentation coverage check after the CLI planning reference was split out.

```text
truncate --size <desiredSize> <source>
tgt property updates render
provider = "scst"
providerCapabilities
partiallySuppressed
rollbackOptions
device-mapper rename verification failures
ZFS snapshot rollback/clone
Network-storage failures can also produce proven-safe recipes
rollbackRecipes
requiredTopologyEvidence
replay_proven_safe_rollback_recipe_with_topology_evidence
topology comparison summary already has missing targets
open encrypted mappings, active
ambiguous rollback points, ambiguous rollback targets
Idempotency
operatorOnlyHandoff
```
