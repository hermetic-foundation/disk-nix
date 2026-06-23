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

## Focused Views

Focused commands filter the graph for common workflows:

```sh
disk-nix devices
disk-nix filesystems
disk-nix volumes
disk-nix mappings
disk-nix mounts
disk-nix ids
```

Every focused view accepts `--json`:

```sh
disk-nix devices --json
disk-nix filesystems --json
disk-nix volumes --json
disk-nix mappings --json
disk-nix mounts --json
disk-nix ids --json
```

The JSON form returns a `StorageGraph` subgraph. Edges are preserved when both
endpoints are included in the filtered node set.

Use these commands for:

- `devices`: disks, partitions, dm devices, LVM objects, VDO, RAID, zvols,
  cache devices, multipath devices, NVMe namespaces, loop devices, and swap
- `filesystems`: regular filesystems, Btrfs filesystems/subvolumes/snapshots,
  ZFS datasets/snapshots, and NFS exports
- `volumes`: logical storage objects such as LVM, Btrfs, ZFS, zvols, LUNs, and
  exports
- `mappings`: encryption, device-mapper, LVM, VDO, RAID, multipath, and cache
  layers
- `mounts`: local mountpoints and NFS mounts
- `ids`: nodes with UUID, PARTUUID, label, serial, or WWN identity fields

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
```

The planner accepts either a direct storage spec or the NixOS module wrapper
written to `/etc/disk-nix/spec.json`.

Plan JSON includes:

- `summary.actionCount`
- `summary.offlineRequiredCount`
- `summary.destructiveCount`
- `summary.potentialDataLossCount`
- `summary.unsupportedCount`
- `actions`

Each action includes the target id, operation, risk class, destructive flag,
typed context, and optional advice with non-destructive alternatives.

## Apply Evaluation

Apply is currently policy evaluation, not mutation:

```sh
disk-nix apply --spec ./examples/lifecycle-update.json
disk-nix apply --spec ./examples/lifecycle-update.json --json
```

The report includes:

- `status`
- `apply.policy`
- `apply.allowedCount`
- `apply.blockedCount`
- `apply.blockedSummary`
- `apply.blocked`
- `commandPlan`
- `messages`

The default policy allows online grow and property-change intents, but blocks
offline-required, destructive, irreversible, format, shrink, and
potential-data-loss actions. Unsupported actions are always blocked.

`--execute` is intentionally refused until the executor and post-apply
verification layers are implemented:

```sh
disk-nix apply --spec ./examples/lifecycle-update.json --execute
```

Automation should treat a blocked apply report as a hard stop and surface the
reported advice before requesting a more permissive policy.
When policy allows an action, `commandPlan` records the non-executed commands,
whether each command would mutate system state, and notes that still require
manual or future executor review. Each command also reports readiness:
`ready`, `needs-desired-size`, `needs-domain-implementation`, or `manual-only`,
plus unresolved inputs when applicable.
