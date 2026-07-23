# Feature status

`disk-nix` is now a working implementation, not just a design sketch.

| Built | Still hardening |
| --- | --- |
| Rust CLI, storage graph, probe layer, lifecycle planner, guarded executor, NixOS module. | Broader destructive coverage, more real hardware fixtures, deeper failure-path proof. |

Use [Feature checklist](feature-checklist.md) for requirement-level evidence and [Operator runbooks](operator-runbooks.md) for high-risk human procedures.

## At a glance

| Area | State | Evidence |
| --- | --- | --- |
| CLI and JSON contracts | Implemented | Topology, focused views, capabilities, schema, plan, apply, validate, probe-status, and generated artifacts. |
| Storage awareness | Implemented | Local block, filesystems, complex filesystems, caches, volumes, mappings, encryption, network storage, and runtime-only devices. |
| Planning and apply | Implemented with guarded execution | Risk classification, dependency ordering, reconciliation, review scripts, receipts, and policy gates. |
| NixOS module | Implemented | Typed declarations, native option derivation, steady-state inventory, apply services, and declarative handoff artifacts. |
| Integration proof | Broad but still hardening | Loop-backed, VM-backed, lab-backed, translated disko, and synthetic failure-recovery harnesses. |

The translated upstream disko example suite dry-runs and preflights with all commands ready. Its guarded destructive mode executes the non-ZFS and non-bcachefs examples on disposable stable `/dev/disk/by-id` lab disks.

The destructive suite capability-skips kernel-unsupported ZFS and bcachefs examples. On the current lab host, the stable disk set maps to the disks currently enumerated as `/dev/sda` and `/dev/sdc` through `/dev/sdf`; `/dev/sdb` is excluded because it is the system disk after the reboot.

## Implemented foundation

### Packaging And Architecture

The repository ships as a Nix flake with package builds, a development shell, formatting, clippy, tests, NixOS module checks, example checks, schema checks, opt-in integration harness packaging, completions, and manpage output.

The Rust workspace is split into model, probe, plan, exec, and CLI crates. JSON contracts exist for topology, focused views, capabilities, schema, plan, apply, validate, probe-status, and generated artifacts.

### CLI And Probe Surface

The CLI exposes read-only topology plus focused views for devices, partitions, filesystems, complex filesystems, Btrfs, bcachefs, ZFS, volumes, pools, snapshots, mappings, encryption, caches, LVM, VDO, multipath, NVMe, RAID, loop, backing files, swap, zram, iSCSI, LUNs, NFS, mounts, network storage, identity, usage, and object inspection.

Probe-status reports classify missing tools, permission barriers, parse failures, inaccessible kernel or service data, and generic command failures. Preflight JSON includes remediation hints for packages, privileges, fixtures, and manual commands.

### Planning And Apply

Planning classifies actions as online, offline-required, destructive, potential-data-loss, reversible, safe, or unsupported. Dependency ordering records build, mutate, and teardown phases, inferred edges, recovery edges, graph-derived path diagnostics, and split-pass proposals.

The guarded apply flow produces dry-run reports, reviewable scripts, readiness summaries, manual-review markers, unresolved-input reports, policy blocks, tool requirements, PATH availability, remediation hints, optional topology probing, missing-tool refusal, and sequential execution of ready commands.

### Current-Topology Reconciliation

Current-topology reconciliation suppresses proven no-op lifecycle actions before command rendering. Covered domains include filesystem resize/mount work, swap, zram, LUKS, LVM, VDO, MD RAID, multipath, ZFS, Btrfs, bcache, loop devices, backing files, iSCSI, LUNs, NFS, disks, partitions, NVMe, and device-mapper.

Warnings remain actionable when topology is absent, degraded, wrong-kind, ambiguous, partially matched, or still in use. Destructive format actions remain planned so policy and confirmation gates continue to protect the operator.

### NixOS Module

The NixOS module declares steady-state resources and imperative lifecycle requests. It emits `/etc/disk-nix/spec.json`, `/etc/disk-nix/steady-state.json`, `/etc/disk-nix/declarative-handoff.nix`, and a reviewable handoff import patch.

Generated steady state covers native NixOS mounts, swaps, zram, LUKS, supported filesystems, NFS exports, storage identities, network-storage identities, iSCSI settings, storage service enablement, and lifecycle-managed resource indexes.

### Integration Harnesses

The smoke harnesses cover loop devices, Btrfs, bcachefs, LUKS, LVM, MD RAID, ZFS, NFS, VDO, iSCSI, multipath, NVMe, zram, bcache, target-side LUNs, and synthetic failed-apply recovery. Details now live in [Integration tests](integration-tests.md), [Integration smoke harnesses](integration-smoke-harnesses.md), and [Integration failure recovery](integration-failure-recovery.md).

Self-contained harnesses create disposable storage and verify real `inspect --json`, reviewed apply plans, property mutation, replacement workflows, and sentinel data survival. Lab-hardware harnesses require explicit environment-selected targets.

Target-side LUN support covers LIO, tgt, SCST, generic provider capability contracts, array-backed provider handoff fields, and host-visible verification probes. Generic target LUN verification plans include `lsscsi -t -s`, `multipath -ll`, and `disk-nix inspect <target> --json`.

## Proof map

| Proof type | Current coverage |
| --- | --- |
| Unit and fixture tests | Model, probe adapters, planning, execution rendering, CLI behavior, schema, and NixOS module evaluation. |
| Safe generated examples | All translated upstream disko examples dry-run and destructive-shape preflight. |
| Disposable mutation | Loop-backed and VM-backed harnesses for local storage, layered stacks, and selected complex filesystems. |
| Lab-backed mutation | NFS, VDO, iSCSI, multipath, NVMe, and target-side LUN paths when explicit disposable targets are supplied. |
| Failure recovery | Synthetic failed-command catalog, partial execution reports, rollback-review behavior, and proven-safe reversible rollback replay. |

## Hardening beyond the checklist

### Integration Breadth

Broader destructive and failure-path integration tests are still needed beyond the smoke suite. The remaining proof should cover more replacement domains, degraded-array variants, cache variants, NVMe namespace variants, LUN flows, property mutations, and failed-command recovery paths.

### Reconciliation And Ordering

More reconciliation logic is still needed for additional operation types and multi-action groups. Current reports already emit `reconciliationGroups`, `graphDependencyConflictResolutions`, dependency phases, lifecycle groups, and refusal behavior for partially suppressed groups.

### Declarative Handoff

Guarded declarative handoff can edit NixOS configuration after successful mutation when explicitly enabled. Operators should still treat it as a post-mutation review aid rather than a replacement for understanding the resulting NixOS storage state.

### Recovery And Rollback

Recovery reports expose generic recovery actions, targeted domain guidance, roll-forward review, rollback preconditions, rollback recipes, receipt binding, fresh-topology probes, and required topology evidence.

The execution crate can replay proven-safe reversible rollback steps when the recipe, receipt, tools, and topology evidence satisfy the safety contract. It refuses review-only, destructive, operator-only, not-ready, unbound, missing-tool, divergent-topology, live-use, ambiguous, stale, idempotency, and plausible-data-loss paths before running commands.

### Probe And Version Hardening

Live probe-status preflight validation still needs broader distribution, privilege, and container-profile coverage. More real-world fixtures are still needed across hardware, fabrics, filesystems, degraded arrays, encrypted stacks, and clustered storage.

Future incompatible spec versions are intentionally blocked until their contract exists. The parser validates version `1`, and `disk-nix migrate` emits a reviewable normalization report plus machine-readable migration metadata.

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
