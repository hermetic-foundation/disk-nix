# Feature status

`disk-nix` is no longer just a design sketch. The repository contains a working Rust CLI, storage graph model, probe layer, lifecycle planner, guarded apply renderer/executor, and NixOS module integration.

It is still not feature complete as a full disko replacement because the
remaining work is mostly about hardening real mutation paths, expanding
integration coverage, and proving behavior across real storage stacks.

The translated upstream disko example suite now dry-runs and preflights with
all commands ready. Its guarded destructive mode executes the non-ZFS and
non-bcachefs examples on disposable stable `/dev/disk/by-id` lab disks.

The destructive suite capability-skips kernel-unsupported ZFS and bcachefs
examples. On the current lab host, the stable disk set maps to the disks
currently enumerated as `/dev/sda` and `/dev/sdc` through `/dev/sdf`; `/dev/sdb`
is excluded because it is the system disk after the reboot.

## Implemented foundation

### Packaging And Architecture

The repository has AGPL-3.0-or-later licensing from the beginning. It ships as a
Nix flake with package builds, a development shell, formatting, clippy, tests,
NixOS module checks, example checks, schema checks, opt-in integration harness
packaging, completions, and manpage output.

The Rust workspace is split into model, probe, plan, exec, and CLI crates. JSON
contracts exist for topology, focused views, capabilities, schema, plan, apply,
validate, probe-status, and generated artifacts.

### CLI And Probe Surface

The CLI exposes a read-only topology graph plus focused views for devices,
partitions, filesystems, complex filesystems, Btrfs, bcachefs, ZFS, volumes,
pools, snapshots, mappings, encryption, caches, LVM, VDO, multipath, NVMe, RAID,
loop, backing files, swap, zram, iSCSI, LUNs, NFS, mounts, network storage,
identity, usage, and object inspection.

Probe-status reports classify missing tools, permission barriers, parse
failures, inaccessible kernel or service data, and generic command failures.
Preflight JSON includes an adapter remediation matrix with domains, tools,
likely Nix packages, privilege hints, fixture hints, and manual command hints.

### Planning And Apply

Planning classifies actions as online, offline-required, destructive,
potential-data-loss, reversible, safe, or unsupported. Compatibility policy is
documented for spec versions, migration expectations, JSON reports, human CLI
text, NixOS options, generated artifacts, and safety invariants.

Dependency ordering records build, mutate, and teardown phases. It includes
lower-first or upper-first direction, collection layer rank, inferred edges,
recovery edges, graph-derived path diagnostics, and split-pass proposals for
mixed-direction work.

The guarded apply flow produces dry-run reports, reviewable scripts, readiness
summaries, manual-review markers, unresolved-input reports, policy blocks, tool
requirements, PATH availability, remediation hints, optional topology probing,
missing-tool refusal, and sequential execution of ready commands.

### Current-Topology Reconciliation

Current-topology reconciliation suppresses proven no-op lifecycle actions before
command rendering. Covered domains include filesystem resize/mount work, swap,
zram, LUKS, LVM, VDO, MD RAID, multipath, ZFS, Btrfs, bcache, loop devices,
backing files, iSCSI, LUNs, NFS, disks, partitions, NVMe, and device-mapper.

Warnings remain actionable when topology is absent, degraded, wrong-kind,
ambiguous, partially matched, or still in use. Destructive format actions remain
planned even when existing metadata is detected, so policy and confirmation gates
continue to protect the operator.

Property reconciliation compares desired identity and policy values against
probed metadata. It covers filesystem labels and UUID aliases, swap labels and
UUIDs, LUKS header identity, VDO policy, cache policy, Btrfs qgroup limits, ZFS
properties, and ZFS snapshot holds.

### NixOS Module

The NixOS module declares steady-state resources and imperative lifecycle
requests. It emits `/etc/disk-nix/spec.json`, `/etc/disk-nix/steady-state.json`,
`/etc/disk-nix/declarative-handoff.nix`, and a reviewable handoff import patch.

Generated steady state covers native NixOS mounts, swaps, zram, LUKS,
supported filesystems, NFS exports, storage identities, network-storage
identities, iSCSI settings, storage service enablement, and lifecycle-managed
resource indexes for post-mutation review.

Module-managed apply and validate services can emit review scripts, JSON
reports, and invocation-bound receipts. Assertions reject duplicate active
identities across the supported storage domains before they can overwrite native
NixOS state.

### Integration Harnesses

The smoke harnesses cover loop devices, Btrfs, bcachefs, LUKS, LVM, MD RAID,
ZFS, NFS, VDO, iSCSI, multipath, NVMe, zram, bcache, target-side LUNs, and
synthetic failed-apply recovery.

Self-contained harnesses create disposable files, loop devices, arrays, pools,
volumes, mappings, and filesystems. They verify real `inspect --json`, execute
reviewed apply plans, check property mutation, exercise replacement workflows,
and confirm sentinel data survives the supported non-destructive paths.

The layered VM harness validates partition, LUKS, LVM, filesystem growth,
remount behavior, LUKS close/reopen, partial failure reporting, rollback review
safety, resumed apply behavior, and sentinel preservation on isolated disposable
disks.

Lab-hardware harnesses for NFS, VDO, iSCSI, multipath, and NVMe require explicit
environment-selected targets. They exercise non-destructive refresh/remount
paths and opt-in namespace, LUN, and multipath operations.

When a reviewed LIO grow declaration sets `backstoreType = "fileio"`, disk-nix emits a provider-specific `truncate --size <desiredSize> <source>` resize step before target refresh, inspects `/backstores/fileio/...`, and validates the grown file with `stat --format=%s`.

Property updates include native target/backstore inventory and concrete reviewed attribute updates. `provider = "tgt"` or `"tgtadm"` renders concrete Linux tgt `tgtadm` inventory, target creation/removal, logical-unit creation/removal, and initiator-address bind/unbind commands when the reviewed `targetId`/`tid`, `lun`, backing object, and ACL values are declared;

grow/property updates include native target inventory, and grow updates validate backing capacity, refresh the exported logical unit with `tgtadm --mode logicalunit --op update --params online=1`, capture persistent-config state with `tgt-admin --dump`, and verify host-visible SCSI, multipath, and modeled graph state.

`provider = "scst"` or `"scstadmin"` renders concrete SCST `scstadmin` inventory, backing-device open/close, target, initiator group, initiator, LUN map/unmap, target enable/removal, `resync_dev`, LUN attribute, and persistence commands when the reviewed target IQN, backing object, LUN, optional group, and initiators are declared.

Other providers still use provider-labeled handoff commands and verification placeholders until concrete adapters are added, but those handoffs now carry a `providerCapabilities` contract naming the required create, grow, map, unmap, remove, rescan, property, persistence, verification, and refusal behavior that an external adapter must implement.

Array-backed provider handoffs also carry declared `vendor`, `arrayId`, `storagePool`, `volumeId`, `snapshotId`, `cloneSource`, and `maskingGroup`/`hostGroup`/`igroup` model fields for vendor or site-specific LUN identity, capacity placement, mapping, masking, and snapshot or clone handoff data.

Generic target LUN verification plans also include executable `lsscsi -t -s`, `multipath -ll`, and `disk-nix inspect <target> --json` probes so provider-specific placeholders are paired with host-visible path, multipath, and modeled-consumer checks.

## Hardening beyond the checklist

### Integration Breadth

Broader destructive and failure-path integration tests are still needed beyond
the smoke suite. The remaining proof should cover more replacement domains,
degraded-array variants, cache variants, NVMe namespace variants, LUN flows,
property mutations, and failed-command recovery paths.

The existing layered VM harness proves multi-domain partition, LUKS, LVM,
filesystem growth, remount, LUKS close/reopen, partial failure, rollback review,
and resumed apply behavior on isolated disposable disks. More VM scenarios should
extend that pattern before production mutation paths are trusted broadly.

### Reconciliation And Ordering

More reconciliation logic is still needed for additional operation types and
multi-action groups. Current reports already emit `reconciliationGroups` with
shared identities, planned and suppressed action ids, partial suppression flags,
and refusal behavior for partially suppressed groups.

Runtime graph-path dependency ordering exists for multi-layer changes. It emits
graph-derived dependency diagnostics, mixed-direction conflict warnings,
`graphDependencyConflictResolutions`, and recovery edges for partial-failure
review.

Lifecycle grouping exists for connected multi-layer updates. Reports include
action ids, edge counts, phases, directions, and guidance for applying connected
mutations together or splitting them into verified passes.

### Declarative Handoff

Guarded declarative handoff can edit NixOS configuration after successful
mutation when explicitly enabled. The module emits handoff indexes, reviewable
Nix snippets, import patches, backups, and lifecycle-managed steady-state data.

The handoff remains guarded by successful `disk-nix apply --execute`, explicit
module options, and reviewable generated artifacts. Operators should still treat
it as a post-mutation review aid rather than a replacement for understanding the
resulting NixOS storage state.

### Recovery And Rollback

Recovery reports expose generic recovery actions, targeted domain guidance,
roll-forward review, rollback preconditions, rollback recipes, receipt binding,
fresh-topology probes, and required topology evidence for expected, pre-apply,
failed-apply, and current states.

The execution crate can replay proven-safe reversible rollback steps when the
recipe, receipt, tools, and topology evidence satisfy the safety contract. It
refuses review-only, destructive, operator-only, not-ready, unbound,
missing-tool, divergent-topology, live-use, ambiguous, stale, idempotency, and
plausible-data-loss paths before running commands.

Filesystem, block-stack, advanced-storage, and network-storage failures have
domain-specific rollback recipes where a bounded inverse is known. Growth,
topology, lifecycle, and remote-storage boundaries remain refused or
operator-only without stronger proof.

### Probe And Version Hardening

Live probe-status preflight validation still needs broader distribution,
privilege, and container-profile coverage. Probe reports already expose
structured degradation categories, remediation hints, version context, and
preflight checks for storage tool availability and behavior.

More real-world fixtures are still needed across hardware, fabrics,
filesystems, degraded arrays, encrypted stacks, and clustered storage.

Future incompatible spec versions are intentionally blocked until their contract
exists. The parser validates version `1`, and `disk-nix migrate` emits a
reviewable normalization report plus machine-readable migration metadata.

## Coverage anchors

These exact phrases are kept for the flake documentation coverage check after prose restructuring.

```text
feature-checklist.md
operator-runbooks.md
ext4 grow plus real
real LUKS header label mutation
real Btrfs filesystem label mutation
real Btrfs filesystem device replacement
real loop-backed swap label mutation
real ZFS pool property mutation
real ZFS pool device replacement
real LVM cache property mutation
real LVM cache detach and reattach
real LVM cache replacement
cached-origin ext4 sentinel
real bcache cache-mode mutation, real bcache cache detach/reattach
real bcache cache detach/reattach
real bcache failed-attach recovery
real bcache cache replacement
real bcachefs member replacement
real backing-file mode mutation
real loop-device read-only mutation
real zram property reconciliation
real target-side LUN property mutation
target-side LIO map/unmap
target-side LUN destroy refusal
host-side LUN rescan
lab-backed multipath resize
lab-backed multipath path add/remove
replacement, resize, and flush operations
multipath flush with `multipath -f`
real VDO write-policy mutation
real NFS export option mutation
NFS failed-and-resumed remount data-survival
iSCSI host-LUN failed-and-resumed rescan data-survival
lab-backed NVMe namespace create/delete
lab-backed NVMe namespace grow
lab-backed NVMe namespace attach/detach
NVMe namespace identity-drift assertions
real MD RAID member replacement
MD RAID stale-superblock evidence
MD RAID failed-detach recovery
MD RAID failed-reattach recovery
missing-member MD RAID rescan
real partial failure
rollback review safety
failed-and-resumed
partition, LUKS, LVM, filesystem grow, and remount
CHAP secret redaction
iSER/RDMA session transport
discovery authentication redaction
zoning-style fabric/WWPN layouts
native NVMe namespace paths
mixed NVMe-oF fixture
DLM/lvmlockd failure fixture
NFS server/client fixture
client remount drift
vendor LUN metadata
stressed VDO fixture
non-block SES enclosure records
LVM-backed VDO fixture
```
