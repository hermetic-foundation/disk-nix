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
swap, LUKS, device-mapper, LVM, VDO, MD RAID, Btrfs, ZFS, iSCSI, LUNs, NFS,
bcache, multipath, NVMe namespaces, and loop devices. Nodes are merged by id
when multiple probe adapters report complementary information.

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
`options`, `properties`, and Btrfs device-membership update fields. It also
includes disk and partition lifecycle collections, swap, LUKS, NFS mount
wrappers, iSCSI discovery/session/boot wrappers, Btrfs subvolume, VDO, LVM thin
pool, LVM snapshot, loop-device, MD RAID, multipath, and zvol lifecycle
declarations, higher-layer lifecycle collections, snapshot declarations
including Btrfs `readOnly` snapshot intent, supported operation names, apply
policy fields, and NixOS activation helper fields such as
`probeCurrent`, `failOnBlocked`, `scriptOut`,
and `reportOut`.
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
disk-nix volumes
disk-nix pools
disk-nix snapshots
disk-nix mappings
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
disk-nix volumes --json
disk-nix pools --json
disk-nix snapshots --json
disk-nix mappings --json
disk-nix mounts --json
disk-nix network-storage --json
disk-nix ids --json
disk-nix usage --json
```

The JSON form returns a `StorageGraph` subgraph. Edges are preserved when both
endpoints are included in the filtered node set.

Use these commands for:

- `devices`: disks, partitions, dm devices, LVM objects, VDO, RAID, zvols,
  cache devices, multipath devices, NVMe namespaces, loop devices, and swap
- `partitions`: partition nodes with size, PARTUUID, and path
- `filesystems`: regular filesystems, Btrfs filesystems/subvolumes/snapshots,
  ZFS datasets/snapshots, and NFS exports
- `volumes`: logical storage objects such as LVM, Btrfs, ZFS, zvols, LUNs, and
  exports, including ZFS zvol `volsize` when reported by `zfs list`
- `pools`: storage pools and grouping layers such as LVM volume groups, thin
  pools, Btrfs filesystems/qgroups, ZFS pools/vdevs, and MD RAID arrays
- `snapshots`: snapshot objects across LVM, Btrfs, and ZFS, including known
  source relationships and ZFS user-reference counts for held snapshots
- `mappings`: encryption headers/keyslots/tokens, device-mapper, LVM, VDO,
  RAID, multipath, and cache
  layers
- `mounts`: local mountpoints and NFS mounts
- `network-storage`: iSCSI sessions, iSCSI targets, LUNs, NFS exports, and NFS
  mounts
- `ids`: nodes with UUID, PARTUUID, label, serial, or WWN identity fields
- `usage`: nodes with size, used, free, allocated, utilization, or selected
  metadata detail data

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
exports and client mounts, iSCSI sessions, LUNs, and ZFS/Btrfs/LVM snapshots.

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
potential-data-loss actions. Unsupported actions are always blocked.

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
detaching an existing cache-set UUID, changing cache mode, checking dirty data,
and staging replacement cache media without silently formatting unknown devices.
bcache sysfs operations require a concrete `/dev/bcache*` target; logical cache
names remain marked `needs-domain-implementation`.
Btrfs filesystem device-removal plans use Btrfs allocation inspection and
domain-specific `btrfs device remove` rendering, but the mutating command stays
blocked while potential-data-loss actions have no explicit apply override.
Swapfile growth command plans render `swapoff`, `fallocate --length`, `mkswap`,
and `swapon`; block-device swap growth keeps the backing resize command
non-ready until the partition, LV, LUN, or other backing layer is selected.
Swap grow and format commands require a path-shaped target such as `/swapfile`
or `/dev/disk/by-*`.
LUKS open command plans render `cryptsetup open` for preserved existing
containers, while close plans render offline-policy-gated `cryptsetup close`
steps and keep the backing LUKS container intact for later reopen.
Disk initialization plans render policy-gated `parted mklabel` and partition
table reread commands after inspecting the target disk.
Partition create command plans render concrete `parted mkpart`, `partprobe`,
and `blockdev --rereadpt` commands when `device`, `partitionType`, `start`, and
`end` are declared.
Partition grow command plans render concrete `parted resizepart`, `partprobe`,
and `blockdev --rereadpt` commands when `device`, `partitionNumber`, and `end`
or `desiredSize` are declared.
Filesystem shrink command plans render Btrfs allocation checks and
`btrfs filesystem resize <size> <path>` for declared target sizes. Ext shrink
plans render `findmnt`, `umount`, `e2fsck`, and `resize2fs` review steps. Ext
grow and shrink commands use a declared filesystem `device` or `disk` when
present, with source-device commands marked unresolved when the filesystem
declaration only names a mountpoint. XFS shrink renders manual-only migration
guidance.
Btrfs filesystem rebalance plans render `btrfs balance start`; declared
`properties.balance.data`, `properties.balance.metadata`, and
`properties.balance.system` values render as `-d`, `-m`, and `-s` filters for
scoped balances.
Btrfs filesystem label property updates render
`btrfs filesystem label <path> <label>` as ready commands. Ext filesystem label
updates render `e2label <device> <label>` when an explicit backing device is
declared; missing devices and unsupported filesystem property keys remain
marked `needs-domain-implementation`.
MD RAID create plans render destructive-policy-gated `mdadm --create` commands
from explicit `level` and `devices` fields, with exact unresolved-input markers
when either field is missing and `/proc/mdstat` verification. MD create, grow,
and member-removal commands require an explicit array path such as
`/dev/md/root`.
VDO command plans render policy-gated `vdo create` and `vdo remove` commands,
plus online `vdo growLogical` and `vdo growPhysical` growth steps. Create
preflight remains non-ready until a backing device is declared.
NFS export command plans use explicit `client` and `options` lifecycle fields
to render reviewed `exportfs` create, option update, and unexport commands.
They also require a path-shaped local export target such as `/srv/share`.
NFS client mount command plans render reviewed `mount` create commands and
`umount` destroy commands from `nfs.mounts`; missing sources or path-shaped
mountpoints keep those commands non-ready.
iSCSI session command plans use `target` or the lifecycle key as the target IQN
and `portal` or `metadata.portal` for reviewed discovery, login, and logout
commands. LUN command plans model host-side attach, growth rescan, and detach:
create and grow keep the broad `iscsiadm --mode session --rescan` step, grow
adds per-path SCSI rescans, and destroy deletes only declared stable SCSI path
devices before reloading multipath. Attach, grow, and destroy remain non-ready
until stable `device` or `devices` paths are declared. Target-side array
provisioning or deletion must be handled outside the host plan unless a future
target adapter is added.
LVM logical volume command plans render concrete `lvcreate` commands when a
`volumes` create action has a `vg/lv` target and `desiredSize`, and report
missing target form and size separately when either is absent. LV grow and
remove commands also require the canonical `vg/lv` target form.
LVM thin-pool command plans render `lvcreate --type thin-pool`, `lvextend`,
and policy-gated `lvremove` commands for `thinPools` lifecycle declarations,
with separate unresolved-input markers for target form and size. Thin-pool grow
and remove commands require the canonical `vg/pool` target form.
LVM volume group command plans render policy-gated `vgcreate` and `vgremove`
commands for `volumeGroups` lifecycle declarations, reviewed `vgextend`
commands for grow operations with an explicit physical volume, and reviewed
`pvmove` then `vgreduce` commands for explicit physical-volume removal.
Generic add-device, replace-device, and remove-device operations stay non-ready
until the device to add, source device, replacement device, or device to remove
is declared explicitly.
Loop-device refresh and detach commands require `/dev/loop*` targets. Multipath
map growth requires a concrete map target such as `mpatha` or
`/dev/mapper/mpatha`; arbitrary logical map names remain non-ready.
ZFS pool command plans render policy-gated `zpool create` from a single
`device` or explicit `devices` vdev list, policy-gated `zpool destroy`, plus
online topology commands such as `zpool add`, `zpool replace`, `zpool remove`,
and scrub. Pool create preflight inspects declared path-like vdev entries
instead of topology keywords such as `mirror`.
ZFS dataset command plans render reviewed `zfs create` commands, including
create-time `-o key=value` options from declared properties, and policy-gated
`zfs destroy` commands for `datasets` lifecycle declarations.
Zvol command plans render `zfs create -o key=value -V` for declared create-time
properties, `zfs set volsize=...`, policy-gated `zfs destroy`, and
`zfs set key=value` property reconciliation updates for `zvols` lifecycle
declarations.
Btrfs subvolume command plans render `btrfs subvolume create`, policy-gated
`btrfs subvolume delete`, and `btrfs property set -ts <path> ro true|false`
for read-only property declarations.
Btrfs qgroup command plans render `btrfs qgroup create`, policy-gated
`btrfs qgroup destroy`, and `btrfs qgroup limit` for referenced and exclusive
limit declarations in `btrfsQgroups`. Qgroup create, destroy, and limit plans
remain non-ready until the mounted filesystem `target` path is declared.
Generic snapshot declarations render concrete `zfs snapshot` commands for
`dataset@snapshot` names and Btrfs `subvolume snapshot` commands when both the
source target and snapshot name are absolute paths. Destructive snapshot
declarations render policy-gated `zfs destroy` or `btrfs subvolume delete`
commands for the same unambiguous domains.
ZFS snapshot retention declarations render safe `zfs hold <tag> <snapshot>`
and `zfs release <tag> <snapshot>` commands from `hold` and `releaseHold`.
ZFS snapshot rollback declarations render reviewed `zfs rollback` command
details internally, but apply remains blocked as potential data loss.
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
