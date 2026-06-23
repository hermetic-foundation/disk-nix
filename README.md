# disk-nix

`disk-nix` is planned as a NixOS-native storage lifecycle manager: a
read-only storage topology engine first, and a safe imperative planner/apply
engine second.

The long-term goal is a full disko replacement that understands modern Linux
storage stacks:

- block devices, partitions, filesystems, mounts, swap, loop devices
- LUKS and device-mapper mappings
- LVM PVs, VGs, LVs, thin pools, snapshots, cache, and VDO
- Btrfs filesystems, devices, subvolumes, snapshots, qgroups, and usage
- ZFS pools, vdevs, datasets, zvols, snapshots, properties, cache, log, and
  special vdevs
- MD RAID, multipath, NVMe namespaces, iSCSI sessions/targets/LUNs, and NFS
- safe lifecycle operations such as grow, replace, rebalance, property updates,
  and migration advice

The project is licensed under AGPL-3.0-or-later from the beginning.

## Current status

This repository is an active implementation. The CLI provides read-only storage
topology views, adapter status reporting, lifecycle planning, policy-gated apply
evaluation, NixOS module integration, and fixture-backed parser coverage for the
storage graph.

## Development

Use the flake:

```sh
nix develop
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Or run all configured checks:

```sh
nix flake check
```

The flake checks build the CLI, run workspace tests, validate the NixOS module,
and execute the checked-in example specs through `plan`, `apply`, and
`--script-out` so JSON contracts and review-script generation stay covered.

## CLI

```sh
disk-nix topology
disk-nix topology --json
disk-nix probe-status
disk-nix probe-status --json
disk-nix capabilities
disk-nix capabilities --json
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
disk-nix ids --json
disk-nix usage
disk-nix usage --json
disk-nix inspect /dev/nvme0n1
disk-nix inspect /
disk-nix inspect / --json
disk-nix plan --spec ./examples/simple-root.json
disk-nix plan --spec ./examples/lifecycle-update.json
disk-nix plan --spec ./examples/simple-root.json --json
disk-nix plan --spec ./examples/simple-root.json --probe-current --json
disk-nix apply --spec ./examples/lifecycle-update.json
disk-nix apply --spec ./examples/lifecycle-update.json --json
disk-nix apply --spec ./examples/lifecycle-update.json --probe-current --json
disk-nix apply --spec ./examples/simple-root.json --script-out ./disk-nix-apply.sh
disk-nix apply --spec ./examples/lifecycle-update.json --report-out ./apply-report.json
disk-nix validate --spec ./examples/lifecycle-update.json --json
disk-nix schema
disk-nix completions bash
disk-nix manpage
```

The canonical interface is intended to be stable JSON. Human tables and tree
views are presentation layers over the same model. Focused JSON commands such
as `devices --json`, `partitions --json`, `pools --json`,
`snapshots --json`, `network-storage --json`, `ids --json`, and
`usage --json` return subgraphs and preserve relationships between nodes
included in the result. `usage` summarizes size, used, free, allocated, and
utilization fields across graph nodes that expose capacity data.
`inspect --json` returns matched nodes plus their direct neighbors and
relationship edges. `capabilities --json` returns the modeled operation/risk
matrix.
The Nix package installs generated bash, zsh, and fish completions plus a
`disk-nix(1)` manpage. The `completions` and `manpage` commands can also emit
those artifacts directly.
`schema` emits the supported desired-spec JSON contract for editor integration
and automation; the Nix package also installs it at
`share/disk-nix/schema/disk-nix-spec.schema.json`.

See [docs/cli.md](docs/cli.md) for the command reference and JSON contracts.

## NixOS module

The flake exposes a NixOS module:

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;
    apply.mode = "activation";
    apply.probeCurrent = true;
    apply.failOnBlocked = true;
    apply.scriptOut = "/run/disk-nix/apply.sh";
    apply.reportOut = "/run/disk-nix/apply-report.json";
  };
}
```

The module installs the CLI, writes a normalized storage spec to
`/etc/disk-nix/spec.json`, derives typed NixOS `fileSystems`, `swapDevices`,
and initrd LUKS options, and keeps lifecycle domains available in the same
planner spec. When `apply.scriptOut` is set, activation validation asks the CLI
to write the allowed command plan and post-apply verification plan to that
reviewable shell script path. When `apply.reportOut` is set, activation also
writes the JSON report before returning blocked-policy failures. Set
`apply.failOnBlocked = false` to use report-only validation during activation;
blocked actions are still reported, but the unit exits successfully.

## Safety model

`disk-nix` treats all mutation as planned work:

1. discover current topology
1. normalize it into a typed graph
1. compare it with desired state
1. classify every action by risk
1. recommend non-destructive alternatives where possible
1. require explicit policy before mutation
1. verify after execution

No destructive operation should be implicit.

`disk-nix apply` is currently a policy-gated dry run. It evaluates the planned
actions against the `apply` policy in the spec, reports blocked operations,
emits advisory command and verification plans, and can write those plans to a
reviewable shell script with `--script-out`. The `--execute` flag is
intentionally refused until a direct mutating executor exists.
Planner coverage includes filesystem resize intent, disk and partition
lifecycle declarations, swap signature/resize workflows, LUKS format/resize
workflows, Btrfs subvolume creation/deletion, VDO create/grow/remove, LVM
thin-pool create/grow/remove, LVM snapshot create/merge/remove, loop-device mapping
updates, MD RAID member updates, multipath map updates, ZFS dataset and zvol
updates, volume and pool updates, network LUN growth, snapshots, and cache
replacement.
Cache apply plans include bcache-aware attach, cache-mode, dirty-data, and
replacement review steps instead of a generic cache placeholder.
VDO apply plans render gated `vdo create` and `vdo remove` commands, plus
online `vdo growLogical` and physical growth review steps.
NFS export apply plans render reviewed `exportfs` create/unexport commands
from explicit client and option declarations.
ZFS dataset apply plans render reviewed `zfs create` commands and
policy-gated `zfs destroy` commands.
LVM logical volume apply plans render reviewed `lvcreate` and gated
`lvremove` steps for volume lifecycle declarations.
LVM thin-pool apply plans render reviewed `lvcreate --type thin-pool`,
`lvextend`, and gated `lvremove` steps.
LVM volume group apply plans render gated `vgcreate` and `vgremove` steps for
volume group lifecycle declarations.
`disk-nix validate` emits the same dry-run report but exits successfully when
policy blocks actions, which makes it suitable for CI and NixOS config checks.
Use `--report-out` with either command to persist the JSON report for review
even when policy blocks the operation.
