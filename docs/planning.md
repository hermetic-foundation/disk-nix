# Planning

`disk-nix plan` reads a desired storage JSON document and emits a
risk-classified action plan.

The plan summary reports total actions plus `offlineRequiredCount`,
`destructiveCount`, `potentialDataLossCount`, and `unsupportedCount` so callers
can gate workflows before looking at individual actions.

The command accepts either a direct spec:

```json
{
  "filesystems": {
    "root": {
      "mountpoint": "/",
      "fsType": "xfs",
      "resizePolicy": "grow-only",
      "desiredSize": "100%",
      "preserveData": true
    }
  }
}
```

or the NixOS module wrapper written to `/etc/disk-nix/spec.json`:

```json
{
  "spec": {
    "filesystems": {}
  },
  "apply": {}
}
```

Current planning is intentionally conservative. It classifies filesystem
resize policy, preservation intent, and lifecycle operations for disks,
partitions, swap, LUKS containers, Btrfs subvolumes, VDO volumes, volumes, LVM
thin pools, LVM snapshots, loop-device mappings, MD RAID arrays, multipath
maps, pools, datasets, zvols, LUNs, iSCSI sessions, exports, cache layers, and
snapshots. It reports
destructive or potentially destructive behavior with alternatives instead of
silently accepting unsafe mutation.

Examples:

- `resizePolicy = "grow-only"` is classified as online growth intent.
- `desiredSize`, `targetSize`, or `size` is carried into action context so
  command and verification plans can use concrete capacity targets when the
  storage domain supports them.
- `resizePolicy = "shrink-allowed"` is classified as potential data loss and
  recommends migration or backup-first alternatives. Command plans render
  reviewed Btrfs shrink commands when a target size is declared, and ext
  offline shrink steps with unresolved source-device inputs when only a
  mountpoint is known.
- XFS shrink intent is classified as unsupported because XFS does not support
  shrinking in place; the planner and command renderer recommend creating a
  smaller filesystem and migrating data.
- `preserveData = false` is classified as destructive because it permits
  formatting or replacement.
- `removeDevices = [ ... ]` is classified as potential data loss and recommends
  replacement capacity, evacuation, and health verification. Btrfs filesystem
  device removal also verifies allocation state with `btrfs filesystem usage`
  before rendering the reviewed `btrfs device remove <device> <mountpoint>`
  command.
- Btrfs filesystem `operation = "rebalance"` renders `btrfs balance start`.
  Optional `properties.balance.data`, `properties.balance.metadata`, and
  `properties.balance.system` values become `-d`, `-m`, and `-s` balance
  filters so operators can prefer scoped balances over a full balance.
- `replaceDevices = { old = new; }` is classified as reversible because the
  original device can remain available until verification passes.
- Cache `replace-device` is classified as offline-required because dirty or
  writeback data must be flushed or detached cleanly before replacement.
- Cache `remove-device` is classified as offline-required rather than
  destructive; reviewed plans require dirty-data inspection before bcache
  detach and keep the backing storage intact.
- disk partition-table creation is classified as destructive because it can
  hide or replace existing storage metadata. When destructive policy permits
  it, apply plans render reviewed `parted mklabel` and table reread commands.
- partition creation and growth are classified as offline-required because the
  kernel partition table reread and dependent consumers must be coordinated.
  Create and growth plans render concrete table rereads when the backing disk is
  declared.
- swap signature creation is classified as destructive; swap growth is
  offline-required because active swap must be disabled before backing storage
  and signatures are changed. Swapfile growth can render a concrete file resize
  command; block-device swap growth must use the backing storage layer first.
- LUKS `operation = "create"` with preserved data opens an existing encrypted
  container as a mapper and is offline-required. LUKS `operation = "format"` or
  `preserveData = false` is destructive. LUKS growth and mapper close are
  offline-required because backing capacity, mapper state, and dependent
  consumers must be coordinated. Mapper close keeps the LUKS header and backing
  data intact unless a separate format action is requested.
- Btrfs subvolume creation is online, while destruction is destructive and
  suggests read-only snapshots or rename-first validation.
- VDO creation and removal are destructive because they write or remove VDO
  metadata on the backing device; VDO growth is online, with advice to
  distinguish logical growth from physical backing growth and verify
  `vdostats`. Create preflight inspection is marked unresolved until a backing
  device is declared.
- LVM logical volume creation is online when it allocates from existing volume
  group free extents; LV removal is destructive because it deletes the volume
  contents. Create command plans report missing `vg/lv` target form and size
  inputs separately.
- LVM thin-pool creation and growth are online allocations inside an existing
  volume group; thin-pool removal is destructive because it removes contained
  thin volumes and their data. Create command plans report missing `vg/pool`
  target form and size inputs separately.
- LVM volume group creation and removal are destructive because they write or
  remove VG metadata on member physical volumes; prefer `vgextend` when
  preserving an existing group is possible. VG growth with an explicit physical
  volume is an online `vgextend` workflow.
- ZFS pool creation and destruction are destructive because they write labels
  to vdev devices or remove all contained datasets and zvols; create command
  plans accept either a single `device` or an explicit `devices` vdev list.
  Preflight inspection targets path-like vdev entries, while topology keywords
  such as `mirror` stay in the rendered `zpool create` command. Import/export
  is preferred when moving an existing pool.
- ZFS dataset creation is online, with declared `properties = { ... }`
  rendered as create-time `zfs create -o key=value` options as well as
  explicit property reconciliation actions. Advice still calls out inherited
  mountpoint, quota, reservation, and encryption policy; dataset destruction
  remains destructive and recommends snapshots or rename-first validation.
- zvol creation, growth, and property updates are online operations, with
  advice to verify pool capacity, reservation policy, and downstream block
  consumers. zvol `properties = { ... }` render create-time `-o key=value`
  options and `zfs set key=value <zvol>` reconciliation actions.
- MD RAID creation is destructive because it writes array metadata to member
  devices. Create command plans identify missing RAID level and member-device
  fields separately. Member add is online; replacement and grow/reshape are
  offline-required because redundancy, resync, and dependent consumers must be
  coordinated.
- Multipath map growth and path add are online; path replacement is
  offline-required because at least one healthy path must remain active while
  paths are added and deleted.
- LVM thin pool growth is online, with advice to monitor data and metadata
  utilization, autoextend policy, and thin-volume overcommit.
- LVM snapshot creation is reversible; snapshot merge rollback is potential
  data loss; snapshot removal is destructive because it deletes a recovery
  point.
- Loop-device creation and capacity refresh are online; detach is
  offline-required because mounts, mappers, and other consumers must be stopped
  before the mapping is removed.
- `properties = { ... }` is classified as safe property-update intent. Btrfs
  subvolume `readonly`, `readOnly`, or `ro` declarations render
  `btrfs property set -ts <path> ro true|false`.
- Btrfs qgroup `properties.limit` or `properties.maxReferenced` render
  `btrfs qgroup limit <size|none> <qgroupid> <path>`;
  `properties.maxExclusive` renders the exclusive limit form with `-e`.
  `operation = "create"` and `destroy = true` render Btrfs qgroup create and
  destroy commands when a filesystem `target` path is declared. Qgroup create,
  destroy, and limit plans remain non-ready until that mounted filesystem path
  is declared.
- Cache attach and cache-mode updates are online or safe when they use an
  existing cache-set identity; cache replacement remains offline-required
  because dirty writeback data must be flushed or detached before media
  changes.
- NFS export creation is online when it publishes an existing path to explicit
  clients and options; unexporting is offline-required because remote clients
  may need to be drained, but it is not treated as data destruction.
- LUN `operation = "create"` means host-side attach for an existing target-side
  LUN and is online when it only rescans sessions and verifies stable paths.
  LUN `operation = "grow"` is offline-required because the storage target,
  host rescan, multipath, and consumers must be coordinated. LUN destruction is
  modeled as host-side path detach, not target-side array deletion, and remains
  offline-required. When stable `device` or `devices` paths are declared, apply
  plans render per-path SCSI rescans or deletes in addition to broad iSCSI
  session and multipath refreshes. Executable attach, grow, and detach plans
  remain non-ready until those stable LUN paths are declared.
- iSCSI session `operation = "grow"` is classified as offline-required because
  target growth, session/path rescan, and dependent consumers must be
  coordinated.
- `destroy = true` is classified as destructive and recommends backup,
  migration, snapshot, rename, or unmount-first alternatives depending on the
  target type.
- snapshot creation is reversible; snapshot rollback is potential data loss;
  snapshot destruction is destructive because it removes a recovery point.
  Generic snapshot names such as `pool/dataset@snap` map to ZFS snapshots;
  absolute source and snapshot paths map to Btrfs subvolume snapshots. Btrfs
  snapshot declarations can set `readOnly = true` to render
  `btrfs subvolume snapshot -r`. Snapshot destruction remains destructive, and
  unambiguous ZFS snapshot names or Btrfs absolute snapshot paths render
  reviewed `zfs destroy` or `btrfs subvolume delete` commands. ZFS snapshot
  `hold` and `releaseHold` declarations are safe property actions that render
  `zfs hold <tag> <snapshot>` and `zfs release <tag> <snapshot>`. ZFS rollback
  command rendering is available for review, but apply remains blocked until a
  safer explicit potential-data-loss policy exists.

The checked-in specs under `examples/` are part of `nix flake check`. The
flake validates stable plan summaries, selected action ids, allowed simple
apply output, blocked lifecycle apply output, and review-script generation.
`disk-nix schema` emits a JSON Schema-style contract for direct specs, NixOS
module wrapper specs, lifecycle collections, snapshot declarations, and apply
policy fields.

Lifecycle collections currently accepted by the planner:

- `disks`
- `partitions`
- `swaps`
- `luks.devices`
- `btrfsSubvolumes`
- `btrfsQgroups`
- `vdoVolumes`
- `volumes`
- `volumeGroups`
- `thinPools`
- `lvmSnapshots`
- `loopDevices`
- `mdRaids`
- `multipathMaps`
- `pools`
- `datasets`
- `zvols`
- `luns`
- `iscsiSessions`
- `exports`
- `caches`
- `snapshots`

Lifecycle objects may use:

- `operation` or `action`: `create`, `format`, `grow`, `shrink`,
  `replace-device`, `add-device`, `remove-device`, `set-property`, `snapshot`,
  `rebalance`, `rollback`, or `destroy`
- `addDevices`: list of devices to attach
- `devices`: member devices for arrays, pools, or explicit LUN paths that
  should receive per-path host rescans
- `removeDevices`: list of devices to remove
- `replaceDevices`: object mapping old device to replacement device
- `properties`: object of properties to set
- `desiredSize`, `targetSize`, or `size`: desired capacity for grow, shrink,
  or create plans
- `target`, `path`, or `mountpoint`: explicit target path or object identity
  when it differs from the attribute name
- `device` or `disk`: backing device path for disk, partition, and LUN operations
- `level` or `raidLevel`: MD RAID level for reviewed array creation
- `client`: NFS export client or network selector
- `portal`: iSCSI target portal such as `192.0.2.10:3260`; `metadata.portal`
  is also accepted for NixOS-module-derived session declarations
- `options`: NFS export options used for reviewed `exportfs` create commands
- `start` or `startOffset`, and `end` or `endOffset`: partition geometry for
  partition creation or resizing
- `partitionNumber` or `number`: partition number for concrete partition
  resize commands
- `partitionType` or `type`: partition type/name metadata for partition
  lifecycle plans
- `destroy`: boolean destructive intent
- `preserveData`: boolean preservation policy

Plan actions include typed `context` when a desired object provides useful
executor inputs. Context fields can include collection, name, target, device,
replacement, property, property value, filesystem type, mountpoint, and
desired size, plus partition start, end, and type. Apply reports use this
context to build command plans without relying on action-id parsing.

`disk-nix plan --probe-current --spec <path>` probes the current host and adds
a `topologyComparison` section to the plan. The comparison matches action
targets against the storage graph and reports missing targets, current size
state versus `desiredSize`, filesystem type conflicts, and already-satisfied
property updates where the current graph has enough data. It is advisory and
does not remove actions from the plan.

## Apply policy

`disk-nix apply --spec <path>` reads the same document as `plan`, evaluates the
planned actions against the top-level `apply` policy, and reports whether each
action is allowed or blocked. By default it is a dry run. With `--execute`, it
requires a fully ready command plan before running any storage command.

Apply reports include `blockedSummary` counters for offline-required,
destructive, potential-data-loss, and unsupported blocked actions in addition
to the detailed blocked action list. When policy allows an action, the report
also includes a `commandSummary` plus a `commandPlan` with planned command
argv, mutation markers, manual-review flags, readiness, unresolved inputs, and
notes. If `--probe-current` is set, the report also includes the same
`topologyComparison` emitted by `plan`. It also includes a
`verificationSummary` plus a `verificationPlan` with read-only post-apply
commands and checks for the relevant storage domain. Executed reports also
include `executionResults` with command phase, argv, success, exit status,
stdout, and stderr for each command that ran.
Cache command plans include bcache-aware sysfs updates for existing cache-set
attachment, cache-mode property changes, dirty-data checks, and replacement
scaffolding that remains marked as needing domain implementation until the
replacement cache device and new cache-set UUID are verified.
NFS export command plans use `exportfs -i -o <options> <client>:<path>` for
reviewed create and option-update operations and `exportfs -u <client>:<path>`
for reviewed unexport operations, with unresolved-input markers when clients or
options are missing.
LVM logical volume command plans use `lvcreate --size <size> --name <lv> <vg>`
for `volume` create operations and `lvremove --yes <vg>/<lv>` only after
destructive policy gates allow removal.
LVM volume group grow command plans use `vgextend <vg> <pv>` when a physical
volume device is declared, and mark the command unresolved when it is missing.
Generic add-device and replace-device lifecycle operations remain unresolved
until the device to add, the source device, and the replacement device are
declared explicitly.
ZFS pool device removal renders reviewed `zpool remove <pool> <device>` steps
when the pool layout supports evacuation. LVM volume group device removal
renders reviewed `pvmove <pv>` then `vgreduce <vg> <pv>` steps so allocated
extents are evacuated before the physical volume is reduced. These remain
potential-data-loss intents unless a safer explicit workflow is selected.
Btrfs filesystem rebalance plans render `btrfs balance start` with optional
declared data, metadata, and system filters from lifecycle properties.
Btrfs filesystem label property updates render
`btrfs filesystem label <path> <label>`. Ext filesystem label updates render
`e2label <device> <label>` when the filesystem declaration includes a backing
device; missing devices and unsupported filesystem property keys stay marked
`needs-domain-implementation`.
Ext filesystem grow and shrink actions also carry the declared filesystem
`device` or `disk` into `resize2fs` and `e2fsck` command plans. Mountpoint-only
ext declarations keep source-device mutations marked unresolved until the block
device is explicitly selected.
`disk-nix apply --script-out <path>` writes those allowed command and
verification plans as a reviewable bash script after policy validation passes.
Commands with unresolved inputs remain commented as not ready.
`disk-nix apply --report-out <path>` writes the JSON report before returning a
blocked-policy, not-ready, or failed-execution error, preserving the decision
record for automation and review.
`disk-nix validate --spec <path>` emits the same dry-run report but treats
blocked policy as a successful command result, making it the better fit for
CI, preflight checks, and NixOS validation paths that need to inspect blocked
details. `validate --report-out <path>` writes the same report to disk.

Policy fields currently supported:

- `mode`
- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowGrow`
- `allowOffline`
- `allowPropertyChanges`
- `allowDeviceReplacement`
- `allowRebalance`
- `requireBackup`
- `backupVerified`
- `requireConfirmation`
- `confirmation`
- `requireConfirmationFile`

The default policy allows online grow and property-change intents, but blocks
offline-required, destructive, irreversible, format, shrink, and
potential-data-loss actions.
Unsupported actions are always blocked, even if permissive destructive or
shrink policy flags are enabled.
`allowDeviceReplacement=false` blocks device add, replacement, and removal
actions. `allowRebalance=false` blocks rebalance actions. `requireBackup=true`
requires `backupVerified=true` for destructive or potential-data-loss actions.
`requireConfirmation=true` requires `confirmation=true` for high-risk or
offline actions. `requireConfirmationFile` points at an operator-controlled
file; the CLI treats it as confirmed only when the file contains a standalone
line equal to `disk-nix confirm`, and otherwise leaves the action blocked.
`--execute` requires policy validation and a fully ready command plan. It runs
planned commands sequentially, stops on the first command failure, records
stdout, stderr, and exit status, and only runs verification commands after the
planned command phase succeeds.
