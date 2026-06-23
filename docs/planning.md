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
resize policy, preservation intent, and lifecycle operations for volumes,
pools, datasets, LUNs, iSCSI sessions, exports, cache layers, and snapshots. It
reports destructive or potentially destructive behavior with alternatives
instead of silently accepting unsafe mutation.

Examples:

- `resizePolicy = "grow-only"` is classified as online growth intent.
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
- `properties = { ... }` is classified as safe property-update intent.
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

Lifecycle collections currently accepted by the planner:

- `volumes`
- `volumeGroups`
- `pools`
- `datasets`
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
- `destroy`: boolean destructive intent
- `preserveData`: boolean preservation policy

Plan actions include typed `context` when a desired object provides useful
executor inputs. Context fields can include collection, name, target, device,
replacement, property, property value, filesystem type, and mountpoint. Apply
reports use this context to build command plans without relying on action-id
parsing.

Future planners should compare desired state against the probed topology before
emitting concrete executor actions.

## Apply policy

`disk-nix apply --spec <path>` reads the same document as `plan`, evaluates the
planned actions against the top-level `apply` policy, and reports whether each
action is allowed or blocked. It does not mutate storage yet.

Apply reports include `blockedSummary` counters for offline-required,
destructive, potential-data-loss, and unsupported blocked actions in addition
to the detailed blocked action list. When policy allows an action, the report
also includes a `commandPlan` with non-executed command argv, mutation markers,
manual-review flags, readiness, unresolved inputs, and notes. These command
plans are intentionally advisory until the executor can compare desired state
with the live probed graph and verify post-apply state.

Policy fields currently supported:

- `mode`
- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowGrow`
- `allowOffline`
- `allowPropertyChanges`

The default policy allows online grow and property-change intents, but blocks
offline-required, destructive, irreversible, format, shrink, and
potential-data-loss actions.
Unsupported actions are always blocked, even if permissive destructive or
shrink policy flags are enabled.
`--execute` is reserved for the future executor and is refused after policy
validation so the command cannot pretend to have modified storage.
