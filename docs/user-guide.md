# User guide

This guide explains how to use `disk-nix` as an operator.

Use the reference docs when you need exact field contracts:

- [CLI](cli.md)
- [Planning](planning.md)
- [NixOS module](nixos-module.md)
- [Storage scope](storage-scope.md)
- [Operator runbooks](operator-runbooks.md)

## Operating Model

`disk-nix` separates storage work into four stages:

1. Inspect the current topology.
1. Write or generate a desired spec.
1. Plan and review the actions.
1. Apply only when policy and operator review allow it.

This split is intentional. The tool should make storage changes explainable
before it makes them executable.

## Install And Build

Use the flake from a checkout:

```sh
nix develop
nix build
```

Run the CLI from the flake:

```sh
nix run . -- topology
nix run . -- plan --spec ./storage.json
```

Inside `nix develop`, run `disk-nix` directly after building:

```sh
cargo build
target/debug/disk-nix topology
```

## Discover The Host

Start with adapter readiness:

```sh
disk-nix probe-status
disk-nix probe-status --preflight --json
```

Use this before writing specs. It tells you which tools are available, which
adapters are degraded, and which Nix packages or privileges are likely needed.

Then inspect the full graph:

```sh
disk-nix topology --json > topology.json
```

Use focused views for daily work:

```sh
disk-nix devices
disk-nix partitions
disk-nix filesystems
disk-nix mounts
disk-nix usage
disk-nix ids --json
```

Use domain views for complex storage:

```sh
disk-nix lvm
disk-nix vdo
disk-nix zfs
disk-nix btrfs
disk-nix bcachefs
disk-nix raid
disk-nix multipath
disk-nix nvme
disk-nix iscsi
disk-nix luns
disk-nix nfs
disk-nix network-storage
disk-nix encryption
disk-nix cache
disk-nix snapshots
```

Use object inspection when you need one target:

```sh
disk-nix inspect /dev/disk/by-label/data --json
disk-nix inspect /dev/mapper/cryptroot --json
disk-nix inspect tank/home --json
disk-nix inspect vg0/root --json
disk-nix inspect iqn.2026-06.example:storage.root --json
```

## Write A Spec

Specs are JSON documents with `version = 1`.

A minimal filesystem grow request:

```json
{
  "version": 1,
  "filesystems": {
    "data": {
      "device": "/dev/disk/by-label/data",
      "fsType": "ext4",
      "mountpoint": "/data",
      "resizePolicy": "grow-only",
      "desiredSize": "100%"
    }
  },
  "apply": {
    "probeCurrent": true
  }
}
```

Validate before planning:

```sh
disk-nix validate --spec ./storage.json
```

Normalize older aliases:

```sh
disk-nix migrate --spec ./storage.json --json > migrated.json
```

## Plan And Review

Generate a human plan:

```sh
disk-nix plan --spec ./storage.json
```

Generate a machine-readable plan:

```sh
disk-nix plan --spec ./storage.json --probe-current --json > plan.json
```

Review these fields first:

- `summary`
- `actions`
- `risk`
- `destructive`
- `dependencyOrder`
- `topologyComparison`
- `reconciliationGroups`
- `lifecycleGroups`
- `diagnostics`

If `topologyComparison` suppresses an action, verify that the current host
really already satisfies the request. If a group is only partially suppressed,
split the change or rerun the plan after refreshing topology.

## Apply

Dry-run apply:

```sh
disk-nix apply --spec ./storage.json --json > apply-review.json
```

Generate a reviewable script:

```sh
disk-nix apply --spec ./storage.json --script-out ./apply.sh
```

Execute after review:

```sh
sudo disk-nix apply \
  --spec ./storage.json \
  --execute \
  --report-out ./apply-report.json
```

Use the policy block as a checklist. Do not silence it by enabling every flag.
Enable only the policy needed for the specific change.

Common policy flags:

- `allowGrow`
- `allowShrink`
- `allowOffline`
- `allowDestructive`
- `allowPotentialDataLoss`
- `allowDeviceReplacement`
- `requireBackup`
- `requireConfirmation`
- `probeCurrent`

## Recover From A Failed Apply

Keep the failed report.

Inspect:

- `partialExecutionRecovery.completedActionIds`
- `partialExecutionRecovery.failedActionId`
- `partialExecutionRecovery.failedCommand`
- `partialExecutionRecovery.remainingActionIds`
- `recoveryActions`
- `rollbackRecipes`
- `requiredTopologyEvidence`

Then run a fresh topology probe:

```sh
disk-nix topology --json > topology-after-failure.json
```

Prefer roll-forward when the report says retry is safe. Use rollback only when
the report includes a proven-safe recipe and the required topology evidence
matches the current host.

See [Operator runbooks](operator-runbooks.md) before acting on production
storage.

## Use The NixOS Module

Import the module:

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix.enable = true;
}
```

Start with manual apply mode:

```nix
{
  services.disk-nix.apply = {
    mode = "manual";
    probeCurrent = true;
    allowGrow = true;
    allowShrink = false;
    allowDestructive = false;
    allowPotentialDataLoss = false;
  };
}
```

Declare storage:

```nix
{
  services.disk-nix.filesystems.data = {
    device = "/dev/disk/by-label/data";
    fsType = "ext4";
    mountpoint = "/data";
    resizePolicy = "grow-only";
    desiredSize = "100%";
  };
}
```

The module writes `/etc/disk-nix/spec.json`. Review it:

```sh
disk-nix plan --spec /etc/disk-nix/spec.json --probe-current
```

Use activation or service execution only after manual review is routine for the
host.

## Common Workflows

### Grow A Filesystem

1. Inspect the filesystem and its backing stack.
1. Declare `resizePolicy = "grow-only"` and `desiredSize`.
1. Plan with `--probe-current`.
1. Confirm no shrink or destructive action appears.
1. Apply with grow policy enabled.

### Replace A Device

1. Confirm a current backup or replica.
1. Inspect the pool, array, filesystem, cache, or multipath map.
1. Use the domain-specific `replaceDevices` declaration.
1. Review dependency ordering.
1. Apply only with replacement policy enabled.
1. Verify the replacement appears in the domain inventory.

### Change A Property

1. Inspect the current property value.
1. Declare the desired property under `properties`.
1. Plan with current topology probing.
1. If already satisfied, expect reconciliation suppression.
1. Apply only if the rendered command matches the intended native tool.

### Update Network Storage

1. Verify session, target, LUN, export, or mount identity.
1. Inspect host-visible paths and multipath state.
1. Avoid target-side detach or destructive operations while consumers are
   active.
1. Prefer rescan, remount, login, logout, attach, or detach operations with
   explicit target identities.
1. Keep the apply report for recovery review.

## Test Safely

Run non-destructive checks:

```sh
nix flake check
```

Run the synthetic recovery harness:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-failure-recovery-smoke
```

Run destructive harnesses only in disposable environments:

```sh
nix build .#integration-vm-test
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 nix run .#integration-vm-smoke
```

Targeted harnesses are documented in [Integration tests](integration-tests.md).

## Render Documentation

Render the docs:

```sh
node scripts/render-docs.mjs
```

Open:

```text
build/docs-site/index.html
```

Check desktop and mobile layouts before publishing major documentation changes.
