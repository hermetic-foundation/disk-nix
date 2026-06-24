# Architecture

`disk-nix` is built around a typed storage graph.

## Data flow

```text
NixOS module or JSON spec
  -> desired storage model
  -> current topology probe
  -> planner
  -> risk-classified action graph
  -> executor
  -> post-apply verification
```

## Core crates

- `disk-nix-model`: storage graph types shared by every layer
- `disk-nix-probe`: read-only adapters for Linux storage tools and sysfs
- `disk-nix-plan`: capability and advice model for safe lifecycle changes
- `disk-nix-exec`: policy-gated execution boundary and dry-run reports
- `disk-nix-cli`: human and machine interfaces

Future crates should keep the same boundary:

- `disk-nix-nix`: NixOS-specific spec generation and validation helpers
- `disk-nix-fixtures`: parser and topology fixtures

## Storage graph

The graph is intentionally not a disk tree. Storage is layered and often
many-to-many:

```text
iSCSI LUN -> SCSI disk -> partition -> LUKS -> LVM PV -> VG -> LV -> filesystem -> mount
```

```text
ZFS pool -> vdevs -> datasets -> snapshots
```

```text
Btrfs filesystem -> devices -> subvolumes -> snapshots -> qgroups
```

```text
bcachefs filesystem -> member devices -> mountpoint
```

Nodes describe storage objects. Edges describe relationships such as
`contains`, `backs`, `maps-to`, `member-of`, `mounted-at`, `snapshot-of`,
`cache-for`, and `depends-on`.

Graph nodes can be inspected by id, path, name, UUID, PARTUUID, label, serial,
WWN, or exact property key/value through `disk-nix inspect <query>`.

## Probe adapters

Probe adapters must be optional and degradation-aware. Missing `zfs`, `btrfs`,
`lvm`, `iscsiadm`, `vdo`, or `multipath` tooling should not prevent basic
topology discovery.

Every adapter reports one of:

- `available`
- `unavailable`
- `partial`
- `failed`

Adapter status is visible in `disk-nix topology` and machine-readable through
`disk-nix probe-status --json`, so callers can distinguish a complete topology
from one that is degraded by missing tools or insufficient privileges. Probe
reports also include a structured category such as `missing-tool`,
`permission-denied`, `command-failed`, `parse-failed`, or `inaccessible-data`
so automation can choose a remediation path without scraping free-form
messages.

## Safety model

The planner classifies every action:

- `safe`
- `online`
- `offline_required`
- `reversible`
- `potential_data_loss`
- `destructive`
- `irreversible`
- `unsupported`

Dangerous or unsupported requests should return actionable alternatives instead
of only failing.

The execution boundary must remain policy-gated. `disk-nix-exec` emits dry-run
reports by default, refuses blocked or not-ready plans, and can run fully ready
command plans with `--execute`. Execution records command and verification
results so automation can audit what ran and where failures stopped the plan.
