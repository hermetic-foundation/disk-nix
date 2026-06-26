# Operator runbooks

These runbooks cover high-risk disk-nix workflows where operators need a
repeatable review sequence before running mutating storage commands. They are
written for reviewed execution with `disk-nix plan`, `disk-nix apply`, JSON
reports, generated scripts, receipts, and current-topology probing.

The common rule is simple: do not execute a mutating command until the reviewed
plan, current topology, backups, and rollback or roll-forward path all agree.

## Common preflight

Run this sequence before any high-risk workflow:

```sh
disk-nix probe-status --json > probe-status.json
disk-nix topology --json > topology.before.json
disk-nix plan --spec ./storage.json --probe-current --json > plan.json
disk-nix apply --spec ./storage.json --probe-current --dry-run --json > apply.dry-run.json
disk-nix apply --spec ./storage.json --probe-current --script-out ./apply.sh --json > apply.review.json
```

Review these fields before continuing:

- `probe-status.json`: no unexpected missing tools, permission failures, parse
  failures, or inaccessible kernel/service data for the affected domain.
- `plan.json`: action identities, dependency metadata, policy classification,
  unresolved inputs, and current-topology warnings match the intended change.
- `apply.dry-run.json`: destructive, offline-required, potential-data-loss, and
  manual-review actions are expected and explicitly authorized.
- `apply.review.json`: all mutating commands are concrete, scoped to intended
  devices or resources, and have a clear non-destructive inspection command.
- `apply.sh`: no placeholder command such as `<snapshot-rollback-tool>` or
  unresolved target remains in the execution path.

Take fresh backups or snapshots before workflows that can lose data. Keep
`topology.before.json`, `plan.json`, `apply.dry-run.json`, generated scripts,
and receipt files with the change record.

## Device replacement

Use this runbook for MD RAID member replacement, Btrfs or bcachefs device
replacement, ZFS vdev replacement, multipath path replacement, LVM PV movement,
or any workflow where data is expected to move from one device to another.

Preflight:

1. Capture health and membership before changing anything:

   ```sh
   disk-nix topology --json > topology.before-replacement.json
   disk-nix raid --json > raid.before.json
   disk-nix btrfs --json > btrfs.before.json
   disk-nix bcachefs --json > bcachefs.before.json
   disk-nix zfs --json > zfs.before.json
   disk-nix multipath --json > multipath.before.json
   ```

1. Confirm the replacement target has the expected identity, size, transport,
   and no unexpected existing filesystem, LUKS, LVM, VDO, RAID, or ZFS
   signature.

1. Confirm the source device is still present unless the workflow is a
   documented failed-device replacement.

1. Confirm the redundancy model can tolerate the replacement or degraded state.

Execution:

- Prefer dry-run plus generated script review first.
- Execute only the reviewed replacement commands, not unrelated pending
  lifecycle actions.
- Keep upper layers mounted read-only or stopped when the planned replacement
  requires offline operation.

Post-check:

```sh
disk-nix topology --json > topology.after-replacement.json
disk-nix probe-status --json > probe-status.after-replacement.json
disk-nix apply --spec ./storage.json --probe-current --dry-run --json > apply.after-replacement.json
```

The workflow is complete only when the replacement target is active, the old
member is absent or intentionally detached, redundancy has rebuilt or resilvered
as expected, and `apply.after-replacement.json` no longer reports the
replacement as pending except for explicitly deferred cleanup.

## Rollback

Rollback is not automatically safe. Prefer roll-forward repair when data
placement may have moved, when consumers have observed the partially changed
state, or when the rollback tool can discard newer data.

Preflight:

1. Capture the failed state:

   ```sh
   disk-nix topology --json > topology.failed.json
   disk-nix apply --spec ./storage.json --probe-current --dry-run --json > apply.failed-review.json
   ```

1. Inspect the domain-specific rollback point:

   ```sh
   disk-nix snapshots --json > snapshots.rollback-review.json
   disk-nix zfs --json > zfs.rollback-review.json
   disk-nix lvm --json > lvm.rollback-review.json
   ```

1. Confirm the rollback point still exists, its origin is correct, newer
   snapshots/clones are understood, and dependent mounts, services, exports, or
   remote clients are stopped or drained.

Execution:

- For ZFS rollback, prefer cloning the snapshot for inspection before running
  `zfs rollback`.
- For recursive ZFS rollback, review newer snapshots and clones first because
  recursive rollback can discard lineage.
- For LVM snapshot merge, confirm origin activation and merge status before
  rerunning `lvconvert --merge`.
- Keep `allowPotentialDataLoss` disabled until the reviewed rollback command is
  the intended operation and a backup or snapshot has been verified.

Post-check:

```sh
disk-nix topology --json > topology.after-rollback.json
disk-nix apply --spec ./storage.json --probe-current --dry-run --json > apply.after-rollback.json
```

The rollback is complete only when the target state matches the reviewed
rollback point, dependent services have been restarted deliberately, and the
remaining plan is either empty or intentionally describes follow-up work.

## Failed apply recovery

Use this when `disk-nix apply --execute` fails after one or more commands have
already run.

Preflight:

1. Preserve the failed apply report and receipt.

1. Do not rerun the full plan immediately.

1. Capture fresh state:

   ```sh
   disk-nix topology --json > topology.after-failure.json
   disk-nix probe-status --json > probe-status.after-failure.json
   disk-nix apply --spec ./storage.json --probe-current --dry-run --json > apply.after-failure.json
   ```

1. Review `recoveryActions` in the failed apply report. Run only read-only
   inspection commands first.

Decision:

- Choose roll-forward when the partially completed topology is closer to the
  intended state and upper-layer data has moved.
- Choose rollback only when domain-specific tooling proves it is safer than
  completing the remaining plan.
- Keep dependent filesystems, exports, sessions, and services stopped until the
  graph and live storage state agree.

Post-check:

Run another dry run with current probing. The recovery is complete when the
remaining plan has only intended follow-up actions and all failed-action
recovery notes have been resolved or recorded as operator decisions.

## Degraded arrays and pools

Use this for degraded MD RAID arrays, degraded ZFS pools, incomplete Btrfs or
bcachefs device sets, multipath path loss, or storage fabrics with missing
paths.

Preflight:

```sh
disk-nix topology --json > topology.degraded.json
disk-nix raid --json > raid.degraded.json
disk-nix zfs --json > zfs.degraded.json
disk-nix btrfs --json > btrfs.degraded.json
disk-nix bcachefs --json > bcachefs.degraded.json
disk-nix multipath --json > multipath.degraded.json
disk-nix luns --json > luns.degraded.json
```

Review:

- Identify whether the fault is media, transport, multipath, controller,
  target-side, or an expected maintenance state.
- Do not remove a member unless redundancy, allocation, and recovery status show
  that removal is safe for the domain.
- Prefer replacing or reattaching missing paths over destructive removal when
  data may still be recoverable.
- For multipath, confirm path groups and policies before flushing a map.
- For ZFS, confirm pool health, scan state, vdev role, and error counters before
  any detach, replace, or destroy.
- For MD RAID, confirm sync, reshape, recovery, and spare state before member
  removal or replacement.

Post-check:

The workflow is complete only when the array or pool reports the expected
healthy or intentionally degraded state, path counts are understood, and the
current-topology dry run no longer reports unexpected degraded-state warnings.

## Shared storage and network storage

Use this for NFS, iSCSI, host-visible LUNs, multipath, NVMe fabrics, and any
storage where other clients or storage-array state may exist outside the local
host.

Preflight:

```sh
disk-nix network-storage --json > network-storage.before.json
disk-nix nfs --json > nfs.before.json
disk-nix iscsi --json > iscsi.before.json
disk-nix luns --json > luns.before.json
disk-nix multipath --json > multipath.before.json
disk-nix nvme --json > nvme.before.json
```

Review:

- Confirm whether the operation is host-side only or depends on target-side
  provisioning.
- Drain or coordinate remote clients before unexporting NFS paths, detaching
  shared LUNs, flushing multipath maps, or changing namespace visibility.
- For iSCSI and LUN growth, coordinate target-side growth first, then rescan
  sessions and host-visible paths.
- For host-side detach, verify upper layers, mounts, LVM, filesystems, and
  services no longer consume the path.
- For NFS remounts, compare requested options with negotiated mount options
  before retrying.

Post-check:

```sh
disk-nix network-storage --json > network-storage.after.json
disk-nix apply --spec ./storage.json --probe-current --dry-run --json > apply.shared-after.json
```

The workflow is complete only when all expected sessions, paths, exports,
mounts, and namespace attachments match the intended topology and all remote
client coordination has been recorded outside disk-nix.

## Change record

For every high-risk workflow, retain:

- the spec used for the operation
- `probe-status` output before and after
- topology output before, after failure if applicable, and after completion
- dry-run and generated-script review output
- execute report and receipt if commands were run
- backup or snapshot evidence
- operator decisions for destructive, potential-data-loss, manual-review, or
  rollback steps
