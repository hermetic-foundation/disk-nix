# CLI planning and apply

This page is the structured reference for `plan`, `apply`, `validate`, command
rendering, and rollback reports.

Use [CLI](cli.md) for discovery commands and focused read-only views.

## Commands

| Task | Command |
| --- | --- |
| Plan a spec | `disk-nix plan --spec ./examples/simple-root.json --json` |
| Plan with current topology | `disk-nix plan --spec ./examples/simple-root.json --probe-current --json` |
| Dry-run apply | `disk-nix apply --spec ./examples/lifecycle-update.json --json` |
| Dry-run with topology | `disk-nix apply --spec ./examples/lifecycle-update.json --probe-current --json` |
| Execute allowed commands | `disk-nix apply --spec ./examples/lifecycle-update.json --execute` |
| Validate for CI/review | `disk-nix validate --spec ./examples/lifecycle-update.json --json` |
| Emit a review script | `disk-nix apply --spec ./examples/lifecycle-update.json --script-out ./disk-nix-apply.sh` |
| Persist a report | `disk-nix apply --spec ./examples/lifecycle-update.json --report-out ./apply-report.json` |
| Persist a receipt | `disk-nix apply --spec ./examples/lifecycle-update.json --receipt-out ./apply-receipt.json` |

## Plan Report

| Field | Purpose |
| --- | --- |
| `summary.actionCount` | Number of planned lifecycle actions. |
| `summary.offlineRequiredCount` | Actions needing offline maintenance. |
| `summary.destructiveCount` | Actions that can destroy metadata or data. |
| `summary.potentialDataLossCount` | Actions needing explicit data-loss review. |
| `summary.unsupportedCount` | Requests refused by the capability model. |
| `dependencyOrder` | Build, mutate, teardown, and recovery ordering. |
| `topologyComparison` | Current-state reconciliation when `--probe-current` is used. |
| `actions` | Target, operation, risk, context, and advice per action. |

## Dependency Order

| Concept | Meaning |
| --- | --- |
| Phase | Build, mutate, teardown, or recovery review. |
| Direction | Lower-first for backing-layer growth; upper-first for teardown. |
| Layer rank | Conservative ordering across storage layers. |
| `dependsOn` | Actions that must happen first. |
| `unblocks` | Actions made possible by this action. |
| `recoveryDependsOn` | Reverse dependency for partial-failure review. |
| `recoveryUnblocks` | Recovery path unlocked by this action. |

Current topology can add graph-derived edges between matched actions. Mixed
direction graph paths emit conflict diagnostics and block execution.

## Topology Comparison

`--probe-current` adds reconciliation data before command rendering.

| Data | Meaning |
| --- | --- |
| Matched targets | Declared targets found in the probed graph. |
| Missing targets | Declared targets absent from the current host. |
| Size diagnostics | Existing size compared with desired size. |
| Type conflicts | Current kind or filesystem type differs from the declaration. |
| Format review | Existing metadata found on a target that would be formatted. |
| Property matches | Desired labels, UUIDs, options, or policy already satisfied. |
| Suppressed actions | Safe no-op actions removed from the actionable plan. |
| Warning actions | Actions that remain planned because current state needs review. |

## Reconciliation Groups

`topologyComparison.reconciliationGroups` groups actions sharing an identity.

| Group field | Meaning |
| --- | --- |
| Shared identity | Target, backing object, portal, path, mountpoint, or parent. |
| Planned action ids | Actions still actionable after reconciliation. |
| Suppressed action ids | Actions proven already satisfied. |
| `partiallySuppressed` | Only part of the related group was suppressed. |

Partially suppressed groups remain visible in dry-run reports. Script rendering
and `apply --execute` refuse them until the plan is refreshed or split.

## Apply Report

| Field | Purpose |
| --- | --- |
| `status` | Overall result. |
| `apply.policy` | Effective safety policy. |
| `apply.allowedCount` | Actions allowed by policy. |
| `apply.blockedCount` | Actions blocked by policy. |
| `apply.blockedSummary` | Count by block reason. |
| `apply.blocked` | Detailed blocked-action records. |
| `commandSummary` | Rendered command totals and readiness counts. |
| `toolRequirements` | External tools needed by commands and verification. |
| `commandPlan` | Ordered shell commands and metadata. |
| `verificationSummary` | Verification totals. |
| `verificationPlan` | Read-only post-apply checks. |
| `executionResults` | Command results when `--execute` runs. |
| `recoveryActions` | Operator recovery guidance. |
| `rollbackRecipes` | Review-only or proven-safe rollback recipes. |
| `messages` | Human-facing notes. |

## Default Policy

| Allowed by default | Blocked by default | Always refused |
| --- | --- | --- |
| Online grow | Offline-required maintenance | Unsupported actions |
| Safe property changes | Destructive actions | Future incompatible specs |
| Read-only rescans | Format actions | Unresolved command inputs |
| Review scripts | Shrink actions | Manual-only placeholders |
| Validation reports | Potential data-loss actions | Graph dependency conflicts |

`allowPotentialDataLoss = true` is the explicit opt-in for reviewed rollback,
shrink, and device-removal workflows. Backup and confirmation gates still apply
when enabled.

## Command Readiness

| Readiness | Meaning |
| --- | --- |
| `ready` | The command has concrete inputs and can run if policy allows it. |
| `needs-desired-size` | A resize action lacks a concrete target size. |
| `needs-domain-implementation` | The model exists but this adapter path is not executable yet. |
| `manual-only` | The plan intentionally requires operator handoff. |

`--execute` refuses to run unless every rendered command is ready and every
required tool is available in `PATH`.

## Tool Requirements

`toolRequirements` summarizes executables referenced by command and verification
plans.

| Data | Use |
| --- | --- |
| Tool name | Match host packages and policy. |
| Command count | See how much of the plan depends on the tool. |
| Mutating count | Identify high-risk tool use. |
| Verification count | Identify read-only validation needs. |
| PATH availability | Refuse before the first command if missing. |
| Remediation hint | Suggest likely Nix packages or `toolPackages` additions. |

## Recovery Actions

Recovery actions are advisory unless a rollback recipe is later proven safe.

| Recovery kind | Example use |
| --- | --- |
| Current-state capture | Preserve fresh topology before another attempt. |
| Policy review | Explain which safety gate blocked execution. |
| Missing-input resolution | Tell the operator which target or size is absent. |
| Roll-forward review | Re-run a dry plan against fresh topology. |
| Rollback precondition review | Inspect snapshots, old labels, old mappings, or old options. |
| Recovery point preservation | Keep snapshots, clones, receipts, and topology captures. |

## Partial Execution

Failed execution reports include `partialExecutionRecovery`.

| Field | Meaning |
| --- | --- |
| `completedActionIds` | Actions completed before the failure. |
| `failedActionId` | Action where execution stopped. |
| `failedPhase` | Command phase that failed. |
| failed command | argv, stdout, stderr, and status. |
| retry/review ids | Actions to revisit after repair. |
| remaining ids | Actions not attempted yet. |
| mutating command count | Number of completed mutating commands. |
| fresh-topology notes | Why a new probe is required before resuming. |

## Rollback Recipe Schema

Recipe version 1 separates review work from mutation.

| Section | Meaning |
| --- | --- |
| `readOnlyValidation` | Commands that inspect rollback preconditions. |
| `reversibleMutations` | Proven-safe inverse commands. |
| `destructiveMutations` | Destructive work; never replayed automatically. |
| `operatorOnlyHandoff` | Human-only recovery guidance. |

Recipes bind to the original apply receipt and a fresh topology probe. Missing
bindings refuse replay before any command runs.

## Proven-Safe Rollback

| Domain | Proven-safe examples | Refused boundaries |
| --- | --- | --- |
| Filesystems | Remount rollback, mount verification `umount`, declared label rollback. | Grow, scrub, repair, failed checks. |
| Block stack | Swap/LUKS identity rollback, device-mapper rename inverse, LUKS open verification close. | Partition grow, LVM grow, MD replacement, backing-file grow. |
| Advanced storage | ZFS/VDO/bcache/Btrfs property rollback, bounded ZFS/Btrfs rename inverse. | Snapshot rollback/clone, pool topology, VDO grow, cache replacement. |
| Network storage | NFS option rollback, NFS mount verification `umount`, iSCSI login verification logout, target-side LUN property rollback. | NFS unexport, iSCSI logout, LUN growth, attach/detach topology. |

Network-storage failures can also produce proven-safe recipes when old state is
explicit or verification failed after a bounded action.

## Replay Safety Gates

Replay refuses before executing commands when any gate fails.

| Gate | Refused examples |
| --- | --- |
| Recipe section | Review-only, destructive, operator-only, not-ready, or unbound commands. |
| Tools | Missing command-line tools. |
| Topology summary | Missing targets, size diagnostics, type conflicts, graph conflicts. |
| Live use | Mounted filesystems, active sessions, open mappings, active swaps. |
| Stale identity | Missing rollback points, stale labels, ambiguous targets. |
| Idempotency | Already rolled back, externally modified, partially applied. |
| Data loss | Remove, delete, detach, flush, discard, wipe, rollback, shrink, format. |

`requiredTopologyEvidence` labels can include `expected`, `preApply`,
`failedApply`, and `current`. Replay receipts record the evidence ids used.

## Domain Command Map

| Domain | Representative executable plans |
| --- | --- |
| Filesystems | `mkfs.*`, `resize2fs`, `xfs_growfs`, `btrfs filesystem resize`, `fstrim`, `fsck.*`, `btrfs scrub`. |
| Mounts | `mount`, `mount -o remount`, `umount`, `findmnt`. |
| Partitions | `parted mklabel`, `parted mkpart`, `parted resizepart`, `partprobe`. |
| LUKS | `cryptsetup open`, `close`, `resize`, `config`, `luksAddKey`, `token import`. |
| LVM | `pvcreate`, `vgcreate`, `lvcreate`, `lvextend`, `lvchange`, `pvmove`, `vgreduce`. |
| ZFS | `zpool create`, `zpool import/export`, `zpool scrub`, `zpool replace`, `zfs create`, `zfs set`, `zfs clone`. |
| Btrfs | Subvolume create/delete, qgroup create/limit/destroy, snapshot, scrub, balance. |
| bcachefs | Device add/remove/resize, rereplicate, scrub, fsck. |
| Cache | bcache sysfs, LVM cache attach/detach, cache mode/policy mutation. |
| VDO | `vdo create`, `remove`, `growLogical`, `growPhysical`, `start`, `stop`. |
| MD RAID | `mdadm --create`, `--assemble`, `--stop`, `--grow`, member add/remove/replace. |
| Multipath | `multipath -ll`, `multipathd resize`, path add/remove, `multipath -f`. |
| NVMe | `create-ns`, `delete-ns`, `attach-ns`, `detach-ns`, `ns-rescan`. |
| NFS | `mount`, `remount`, `umount`, `exportfs -i`, `exportfs -u`. |
| iSCSI/LUNs | `iscsiadm`, `lsscsi`, `targetcli`, `tgtadm`, `scstadmin`. |

## Target-Side LUN Providers

| Provider | Native support |
| --- | --- |
| LIO | `targetcli` inventory, backstores, LUN map/unmap, ACLs, attributes, `saveconfig`. |
| LIO fileio | `truncate --size <desiredSize> <source>` before target refresh. |
| Linux tgt | `tgtadm` target and logical-unit lifecycle, `tgt-admin --dump`. |
| SCST | `scstadmin` target, initiator group, LUN, attribute, resync, persistence. |
| Generic | `providerCapabilities` handoff plus host-visible verification probes. |

Generic target LUN verification plans include `lsscsi -t -s`, `multipath -ll`,
and `disk-nix inspect <target> --json`.
