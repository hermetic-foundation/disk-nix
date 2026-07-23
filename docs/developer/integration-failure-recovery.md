# Integration failure recovery

This document describes the synthetic failure-injection harness and the recovery
report behavior it protects.

Use [Integration tests](integration-tests.md) for suite entrypoints and
[Integration smoke harnesses](integration-smoke-harnesses.md) for host-backed
harness details.

## Harness

Run the synthetic failed-apply harness with:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-failure-recovery-smoke
```

The harness refuses to run unless `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set.
It does not require root and does not mutate real storage.

Instead, fake storage tools are placed ahead of `PATH`. Each scenario lets the
planner and executor reach a specific reviewed command, then fails that command
or a required read-only inspection.

## Failure Domains

The catalog covers local filesystems, volume managers, encrypted mappings,
block-stack primitives, complex filesystems, caches, network storage, LUN
providers, and fabric-visible devices.

Filesystem scenarios include ext4 and XFS grow, Btrfs scrub and rebalance,
Btrfs and bcachefs device replacement, filesystem trim, offline checks, repair,
and identity property mutation.

Block-stack scenarios include swap labels, zram inventory and property drift,
loop devices, backing files, device-mapper rename, partitions, LUKS mapper
lifecycle, LUKS header properties, keyslots, and tokens.

Volume and cache scenarios include LVM LV grow, thin-pool create/grow, VG
rename and PV replacement, LVM cache attach/detach/replacement/property
mutation, bcache replacement/property/rescan, and VDO lifecycle/property/grow
paths.

Network and fabric scenarios include NFS remount/unmount/export/unexport, iSCSI
login/logout/rescan, host-side LUN rescan, multipath resize/path changes/flush,
NVMe namespace create/grow/attach/detach/delete, and target-side LUN providers.

Target-side LUN coverage includes LIO, Linux tgt, and SCST create, attach,
detach, destroy, grow, property, and rescan paths.

## Report Contract

The failed report and receipt preserve the partial execution boundary.

Expected fields include:

- `partialExecutionRecovery.completedActionIds`
- `partialExecutionRecovery.failedActionId`
- `partialExecutionRecovery.failedPhase`
- failed command argv, stdout, stderr, and non-zero status
- completed mutating command count
- retry and review action ids
- remaining action ids
- fresh-topology review notes

Domain-specific recovery guidance is expected whenever the failed action has a
concrete target. Reports include read-only inspection commands, roll-forward
review commands, rollback-precondition checks, verification actions, and
recovery-point preservation guidance.

## Rollback Review

The harness verifies that rollback review stays conservative. It can emit
review-only rollback recipes, refused recipes, or proven-safe reversible recipes
only when the failed report contains deterministic old state and the topology
binding requirements are satisfied.

Unsafe rollback sections remain operator-only. Destructive, potential-data-loss,
unbound, missing-tool, live-use, stale-identity, idempotency, and divergent
topology paths are refused before any rollback command can run.

## Failed And Resumed Apply

The synthetic catalog also backs the layered VM failure path. That path injects
a real partial failure after an earlier mutating command succeeds, verifies that
rollback review stays non-mutating, then reruns a clean follow-up apply after
the failure is fixed.

The data-survival checks are intentionally in the destructive smoke harnesses,
not this synthetic harness. See [Integration smoke harnesses](integration-smoke-harnesses.md)
for those host-backed details.
