# Planning

`disk-nix plan` reads a desired storage JSON document and emits a
risk-classified action plan.

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
pools, datasets, LUNs, exports, cache layers, and snapshots. It reports
destructive or potentially destructive behavior with alternatives instead of
silently accepting unsafe mutation.

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
- `properties = { ... }` is classified as safe property-update intent.
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

Future planners should compare desired state against the probed topology before
emitting concrete executor actions.

## Apply policy

`disk-nix apply --spec <path>` reads the same document as `plan`, evaluates the
planned actions against the top-level `apply` policy, and reports whether each
action is allowed or blocked. It does not mutate storage yet.

Policy fields currently supported:

- `mode`
- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowGrow`
- `allowPropertyChanges`

The default policy allows grow and property-change intents, but blocks
destructive, irreversible, format, shrink, and potential-data-loss actions.
Unsupported actions are always blocked, even if permissive destructive or
shrink policy flags are enabled.
`--execute` is reserved for the future executor and is refused after policy
validation so the command cannot pretend to have modified storage.
