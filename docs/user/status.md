# Feature status

`disk-nix` is now a working implementation, not just a design sketch.

| Built | Still hardening |
| --- | --- |
| Rust CLI, storage graph, probe layer, lifecycle planner, guarded executor, NixOS module. | Broader destructive coverage, more real hardware fixtures, deeper failure-path proof. |

Use [Feature checklist](../developer/feature-checklist.md) for requirement-level evidence and [Operator runbooks](operator-runbooks.md) for high-risk human procedures.

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

The smoke harnesses cover loop devices, Btrfs, bcachefs, LUKS, LVM, MD RAID, ZFS, NFS, VDO, iSCSI, multipath, NVMe, zram, bcache, target-side LUNs, and synthetic failed-apply recovery. Details now live in [Integration tests](../developer/integration-tests.md), [Integration smoke harnesses](../developer/integration-smoke-harnesses.md), and [Integration failure recovery](../developer/integration-failure-recovery.md).

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

## Evidence Coverage

The detailed checklist lives in [feature-checklist.md](../developer/feature-checklist.md).
High-risk operator procedures live in [operator-runbooks.md](operator-runbooks.md).

### Local Mutation Evidence

| Area | Current proof |
| --- | --- |
| Filesystem grow | ext4 grow plus real remount verification. |
| LUKS identity | real LUKS header label mutation. |
| Btrfs identity | real Btrfs filesystem label mutation. |
| Btrfs replacement | real Btrfs filesystem device replacement. |
| Swap identity | real loop-backed swap label mutation. |
| ZFS property | real ZFS pool property mutation. |
| ZFS replacement | real ZFS pool device replacement. |
| Backing files | real backing-file mode mutation. |
| Loop devices | real loop-device read-only mutation. |
| zram | real zram property reconciliation. |
| bcachefs | real bcachefs member replacement. |

### Cache And Volume Evidence

| Area | Current proof |
| --- | --- |
| LVM cache property | real LVM cache property mutation. |
| LVM cache lifecycle | real LVM cache detach and reattach. |
| LVM cache replacement | real LVM cache replacement. |
| LVM cache data | cached-origin ext4 sentinel. |
| bcache property | real bcache cache-mode mutation, real bcache cache detach/reattach. |
| bcache lifecycle | real bcache cache detach/reattach. |
| bcache recovery | real bcache failed-attach recovery. |
| bcache replacement | real bcache cache replacement. |
| VDO policy | real VDO write-policy mutation. |
| VDO fixture | LVM-backed VDO fixture and stressed VDO fixture. |

### Network And Fabric Evidence

| Area | Current proof |
| --- | --- |
| Target LUN property | real target-side LUN property mutation. |
| LIO mapping | target-side LIO map/unmap. |
| LUN refusal | target-side LUN destroy refusal. |
| Host LUN | host-side LUN rescan. |
| Multipath resize | lab-backed multipath resize. |
| Multipath paths | lab-backed multipath path add/remove. |
| Multipath lifecycle | replacement, resize, and flush operations. |
| Multipath flush | multipath flush with `multipath -f`. |
| NFS export | real NFS export option mutation. |
| NFS data survival | NFS failed-and-resumed remount data-survival. |
| iSCSI data survival | iSCSI host-LUN failed-and-resumed rescan data-survival. |
| NVMe create/delete | lab-backed NVMe namespace create/delete. |
| NVMe grow | lab-backed NVMe namespace grow. |
| NVMe attach/detach | lab-backed NVMe namespace attach/detach. |
| NVMe drift | NVMe namespace identity-drift assertions. |

### RAID And Recovery Evidence

| Area | Current proof |
| --- | --- |
| MD replacement | real MD RAID member replacement. |
| MD stale metadata | MD RAID stale-superblock evidence. |
| MD failed detach | MD RAID failed-detach recovery. |
| MD failed reattach | MD RAID failed-reattach recovery. |
| MD missing member | missing-member MD RAID rescan. |
| Layered failure | real partial failure. |
| Rollback review | rollback review safety. |
| Resume path | failed-and-resumed. |
| Layered stack | partition, LUKS, LVM, filesystem grow, and remount. |
| Automatic rollback | proven-safe reversible rollback. |

### Real-World Fixture Evidence

| Area | Current proof |
| --- | --- |
| iSCSI security | CHAP secret redaction. |
| iSCSI transport | iSER/RDMA session transport. |
| iSCSI discovery | discovery authentication redaction. |
| Fibre Channel | zoning-style fabric/WWPN layouts. |
| NVMe paths | native NVMe namespace paths. |
| NVMe-oF | mixed NVMe-oF fixture. |
| Clustered LVM | DLM/lvmlockd failure fixture. |
| NFS fixture | NFS server/client fixture. |
| NFS drift | client remount drift. |
| Array metadata | vendor LUN metadata. |
| Enclosures | non-block SES enclosure records. |
