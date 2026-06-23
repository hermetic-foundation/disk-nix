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
  recommends migration or backup-first alternatives.
- XFS shrink intent is classified as unsupported because XFS does not support
  shrinking in place; the planner recommends creating a smaller filesystem and
  migrating data.
- `preserveData = false` is classified as destructive because it permits
  formatting or replacement.
- `removeDevices = [ ... ]` is classified as potential data loss and recommends
  replacement capacity, evacuation, and health verification.
- `replaceDevices = { old = new; }` is classified as reversible because the
  original device can remain available until verification passes.
- Cache `replace-device` is classified as offline-required because dirty or
  writeback data must be flushed or detached cleanly before replacement.
- disk partition-table creation is classified as destructive because it can
  hide or replace existing storage metadata.
- partition creation and growth are classified as offline-required because the
  kernel partition table reread and dependent consumers must be coordinated.
- swap signature creation is classified as destructive; swap growth is
  offline-required because active swap must be disabled before backing storage
  and signatures are changed.
- LUKS format/create is destructive; LUKS growth is offline-required because
  backing capacity, mapper state, and dependent consumers must be coordinated.
- Btrfs subvolume creation is online, while destruction is destructive and
  suggests read-only snapshots or rename-first validation.
- VDO growth is classified as online, with advice to distinguish logical
  growth from physical backing growth and verify `vdostats`.
- zvol creation and growth are online operations, with advice to verify pool
  capacity, reservation policy, and downstream block consumers.
- MD RAID member add is online; replacement and grow/reshape are
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
- `properties = { ... }` is classified as safe property-update intent.
- Cache attach and cache-mode updates are online or safe when they use an
  existing cache-set identity; cache replacement remains offline-required
  because dirty writeback data must be flushed or detached before media
  changes.
- NFS export creation is online when it publishes an existing path to explicit
  clients and options; unexporting is offline-required because remote clients
  may need to be drained, but it is not treated as data destruction.
- LUN `operation = "grow"` is classified as offline-required because the
  storage target, host rescan, multipath, and consumers must be coordinated.
- iSCSI session `operation = "grow"` is classified as offline-required because
  target growth, session/path rescan, and dependent consumers must be
  coordinated.
- `destroy = true` is classified as destructive and recommends backup,
  migration, snapshot, rename, or unmount-first alternatives depending on the
  target type.
- snapshot creation is reversible; snapshot rollback is potential data loss;
  snapshot destruction is destructive because it removes a recovery point.

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
- `removeDevices`: list of devices to remove
- `replaceDevices`: object mapping old device to replacement device
- `properties`: object of properties to set
- `desiredSize`, `targetSize`, or `size`: desired capacity for grow, shrink,
  or create plans
- `target`, `path`, or `mountpoint`: explicit target path or object identity
  when it differs from the attribute name
- `device` or `disk`: backing device path for disk and partition operations
- `client`: NFS export client or network selector
- `options`: NFS export options used for reviewed `exportfs` create commands
- `start` and `end`: partition geometry for partition creation or resizing
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
action is allowed or blocked. It does not mutate storage yet.

Apply reports include `blockedSummary` counters for offline-required,
destructive, potential-data-loss, and unsupported blocked actions in addition
to the detailed blocked action list. When policy allows an action, the report
also includes a `commandSummary` plus a `commandPlan` with non-executed command
argv, mutation markers, manual-review flags, readiness, unresolved inputs, and
notes. If `--probe-current` is set, the report also includes the same
`topologyComparison` emitted by `plan`. It also includes a
`verificationSummary` plus a `verificationPlan` with read-only post-apply
commands and checks for the relevant storage domain. These plans are
intentionally advisory until the executor can run mutating commands directly.
Cache command plans include bcache-aware sysfs updates for existing cache-set
attachment, cache-mode property changes, dirty-data checks, and replacement
scaffolding that remains marked as needing domain implementation until the
replacement cache device and new cache-set UUID are verified.
NFS export command plans use `exportfs -i -o <options> <client>:<path>` for
reviewed create operations and `exportfs -u <client>:<path>` for reviewed
unexport operations, with unresolved-input markers when clients or options are
missing.
`disk-nix apply --script-out <path>` writes those allowed command and
verification plans as a reviewable bash script after policy validation passes.
Commands with unresolved inputs remain commented as not ready.
`disk-nix apply --report-out <path>` writes the JSON report before returning a
blocked-policy or executor-unavailable error, preserving the decision record
for automation and review.
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
`--execute` is reserved for the future executor and is refused after policy
validation so the command cannot pretend to have modified storage.
