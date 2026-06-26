# Feature checklist

This checklist tracks the desired full disko-replacement scope against the
current implementation. Each item is intentionally classified so future work can
move features between states without rewriting the whole document.

Status labels:

- `Finished`: implemented in the repository and covered by ordinary checks,
  fixture tests, module checks, or documented smoke harnesses.
- `Partial`: useful support exists, but more hardening, integration proof,
  reconciliation, or recovery coverage is needed before treating it as
  production-complete.
- `Desired`: not implemented yet, or intentionally blocked until the safety
  model is strong enough.

Update rules:

- Move an item to `Finished` only when the implementation, documentation, and
  verification path are all present.
- Keep risky mutation work as `Partial` until it has failure-path coverage or an
  explicit smoke/integration harness.
- Leave unsupported operations as `Desired` only when the desired outcome is a
  future implementation. If the desired outcome is a safety refusal, track it as
  `Finished`.
- When adding a major storage domain, update this checklist, the flake checks,
  and the relevant architecture, CLI, planning, storage-scope, status, and NixOS
  module documentation.

## Foundation

- [x] **Finished:** AGPL-3.0-or-later license from project start.
- [x] **Finished:** Rust workspace split into model, probe, plan, exec, and CLI
  crates.
- [x] **Finished:** Nix flake with package, development shell, formatting,
  tests, clippy, NixOS module checks, example checks, schema checks,
  completions, manpage generation, and integration-harness packages.
- [x] **Finished:** Machine-readable JSON contracts for topology, focused
  views, capabilities, schema, plan, apply, validate, migrate, and probe status.
- [x] **Finished:** Compatibility policy for spec versions, JSON reports, human
  CLI text, NixOS options, generated artifacts, safety invariants, and future
  migrations.
- [x] **Finished:** CLI subcommands for focused storage views and object
  inspection.
- [x] **Finished:** Parser and fixture tests for probe adapters.
- [ ] **Partial:** Migration maps documented unversioned legacy aliases into
  version `1` with a machine-readable legacy mapping matrix and applied-mapping
  audit trail; future version-to-version mappings still need a version `2`
  contract before they can be finished.

## Read-only storage awareness

- [x] **Finished:** Block devices, partitions, partition tables, IDs, labels,
  UUIDs, size, capacity, free-space, usage, and metadata views.
- [x] **Finished:** Mounts, filesystems, filesystem-specific metadata, check and
  repair support surfaces, and usage accounting.
- [x] **Finished:** Complex filesystem inventory for Btrfs, bcachefs, and ZFS.
- [x] **Finished:** Btrfs filesystems, devices, subvolumes, snapshots, qgroups,
  properties, usage, and device topology.
- [x] **Finished:** bcachefs filesystems, member devices, usage, scrub/fsck
  surfaces, and topology update metadata.
- [x] **Finished:** ZFS pools, vdevs, datasets, zvols, snapshots, holds,
  properties, health, cache/log/special vdev roles, and error counters.
- [x] **Finished:** LVM PVs, VGs, LVs, thin pools, snapshots, cache,
  writecache, and VDO metadata.
- [x] **Finished:** LUKS headers, UUIDs, labels, subsystems, keyslots, tokens,
  mapper state, and device-mapper backing data.
- [x] **Finished:** Device-mapper maps, tables, status, and local mapping
  details.
- [x] **Finished:** Cache layers including bcache, bcache cache sets, LVM
  cache, LVM writecache, and cache-relevant metadata.
- [x] **Finished:** VDO native and LVM-backed metadata, logical/physical sizing,
  compression, deduplication, operating mode, and statistics.
- [x] **Finished:** MD RAID arrays, members, states, degraded/failed indicators,
  and replacement surfaces.
- [x] **Finished:** Multipath maps, paths, policies, handlers, features, and
  degraded state.
- [x] **Finished:** NVMe controllers, namespaces, attachments, health,
  formatted LBA, and namespace capacity metadata.
- [x] **Finished:** SCSI devices, host-visible LUNs, iSCSI sessions, targets,
  portals, and login state.
- [x] **Finished:** NFS exports, NFS client mounts, sources, servers, mount
  options, and negotiated state.
- [x] **Finished:** Loop devices, backing files, swap, zram, SMART telemetry,
  and network-storage identity views.
- [ ] **Partial:** Real-world fixture coverage exists for many parser surfaces,
  but still needs broader hardware, fabrics, degraded arrays, encrypted stacks,
  clustered storage, and shared-storage samples.

## Planning and apply safety

- [x] **Finished:** Policy classification for safe, reversible, online,
  offline-required, destructive, potential-data-loss, and unsupported actions.
- [x] **Finished:** Guarded dry-run apply reports with readiness summaries and
  manual-review markers.
- [x] **Finished:** Script generation for reviewed command plans.
- [x] **Finished:** Missing-tool refusal before execution with package
  remediation hints.
- [x] **Finished:** Per-command mutating/read-only metadata.
- [x] **Finished:** Unresolved-input reporting for actions missing concrete
  required inputs.
- [x] **Finished:** Policy blocks for unsupported or unsafe requests instead of
  guessing.
- [x] **Finished:** Receipt files that bind apply reports to invocation
  metadata.
- [x] **Finished:** Sequential execution of ready commands.
- [x] **Finished:** Dependency-order metadata for build, mutate, and teardown
  phases.
- [x] **Finished:** Inferred dependency edges from declared adjacent-layer
  identities and probed graph paths.
- [ ] **Partial:** Runtime graph-path ordering has coarse phases and dependency
  metadata plus graph-derived order diagnostics, but still needs recovery-aware
  ordering for complex multi-layer mutations.
- [x] **Finished:** Mixed-direction graph-path diagnostics include structured
  conflict resolution proposals. Topology comparison JSON reports
  `graphDependencyConflictResolutions` with the conflicting path, lower and
  upper action ids, dependency directions, build/update pass,
  teardown/recovery pass, and split-plan recommendation; execution remains
  refused while conflicts are present.
- [ ] **Desired:** Production-grade automatic rollback. Current reports provide
  guidance; safe automated rollback remains intentionally unimplemented.

## Lifecycle operations

- [x] **Finished:** Lifecycle vocabulary covers create, grow, shrink where
  supported, check, repair, scrub, trim, mount, remount, unmount, import,
  export, login, logout, attach, detach, open, close, start, stop, assemble,
  activate, deactivate, add/remove/replace device, add/remove LUKS keys and
  tokens, property changes, rename, clone, promote, rollback, and destroy where
  those operations make sense.
- [x] **Finished:** Filesystem lifecycle for ext, XFS, Btrfs, bcachefs, F2FS,
  exFAT, NTFS, FAT, and swap where supported by the domain.
- [x] **Finished:** Filesystem identity updates for labels, UUIDs, FAT volume
  IDs, NTFS serials, exFAT serials, and related metadata.
- [x] **Finished:** LVM PV/VG/LV/thin/snapshot/cache/writecache/VDO lifecycle
  planning and command rendering.
- [x] **Finished:** ZFS pool, dataset, zvol, snapshot, hold, clone, promote,
  rollback, and property lifecycle planning.
- [x] **Finished:** Btrfs device, subvolume, snapshot, qgroup, rebalance, scrub,
  property, and filesystem resize lifecycle planning.
- [x] **Finished:** bcachefs device resize/add/remove, rereplicate, scrub, and
  fsck planning.
- [x] **Finished:** LUKS format/open/close/header/keyslot/token lifecycle
  planning.
- [x] **Finished:** MD RAID create/assemble/stop/member add/removal/replacement
  planning.
- [x] **Finished:** NVMe namespace attach/detach and rescan planning.
- [x] **Finished:** iSCSI discovery, login, logout, rescan, and session
  planning.
- [x] **Finished:** Host-side LUN attach/detach/grow/rescan planning.
- [x] **Finished:** NFS export/unexport and client mount/remount/unmount
  planning.
- [x] **Finished:** VDO create/remove/grow/start/stop/property planning.
- [x] **Finished:** Multipath map/path add/removal/rescan planning.
- [x] **Finished:** Loop, backing-file, swap, zram, cache, and device-mapper
  lifecycle planning.
- [x] **Finished:** Unsafe or unsupported requests such as XFS shrink,
  unsupported filesystem properties, unsupported Btrfs subvolume properties, and
  unsupported VDO property values are blocked or forced to manual review instead
  of being guessed.
- [ ] **Partial:** Target-side LUN provisioning is modeled through
  `targetLuns` create, grow, map, unmap, remove, rescan, and property handoff
  actions. Linux LIO target-side create/map/unmap/rescan now renders concrete
  `targetcli` inventory, backstore, target, LUN mapping, ACL, and persistence
  commands, while non-LIO array/provider adapters still use provider-labeled
  non-ready commands and verification placeholders.
- [ ] **Partial:** Multi-layer lifecycle groups such as LUN refresh,
  multipath refresh, partition growth, LUKS/LVM resize, and filesystem growth
  need stronger ordering, reconciliation, and recovery proof before they are
  production-complete.

## Current-topology reconciliation

- [x] **Finished:** Suppression of many already-satisfied create, grow, import,
  export, login, logout, attach, detach, mount, unmount, remount, start, stop,
  open, close, activate, deactivate, rename, promote, and property actions.
- [x] **Finished:** Reconciliation for LVM activation/deactivation,
  PV/VG/LV/thin/cache state, VDO grow/start/stop/properties, cache properties,
  ZFS properties and holds, Btrfs qgroups, filesystem identities, swap
  identities, LUKS header identities, loop devices, backing files, MD members,
  multipath paths, NFS exports/mounts, iSCSI sessions, LUNs, and NVMe namespace
  visibility.
- [x] **Finished:** Actionable warnings for unsafe or ambiguous current state
  instead of silent suppression.
- [ ] **Partial:** Multi-action reconciliation now emits
  `topologyComparison.reconciliationGroups` with shared identities, planned
  action ids, suppressed action ids, counts, and partially-suppressed group
  flags before command rendering, but more command-rendering gates are still
  needed for complex grouped mutations.
- [ ] **Partial:** More cross-domain reconciliation is needed for grouped
  updates such as iSCSI LUN refresh, multipath refresh, partition growth,
  LUKS/LVM resize, and filesystem growth.

## Recovery guidance

- [x] **Finished:** Generic recovery actions for failed apply runs.
- [x] **Finished:** Targeted failed-action domain recovery guidance.
- [x] **Finished:** Current-topology roll-forward review commands.
- [x] **Finished:** Read-only rollback precondition review commands for
  concrete risky actions.
- [x] **Finished:** Recovery inspection for ZFS/Btrfs snapshots, ZFS
  pools/datasets/zvols, LVM PV/VG/LV/thin, LUKS mapper/header/keyslot/token,
  filesystem lifecycle, caches, swap, disks, partition tables, MD member
  replacement, NVMe namespaces, iSCSI sessions, VDO, multipath, loop devices,
  backing files, device-mapper maps, NFS exports/client mounts, and
  host-visible LUN detach.
- [x] **Finished:** Domain-specific recovery recipes cover failed actions and
  partially completed multi-layer apply runs. Failed execution reports include
  `partialExecutionRecovery` with completed action ids, failed action id, failed
  phase and command, retry/review action ids, remaining action ids, completed
  mutating command count, and fresh-topology review notes, alongside
  domain-specific recovery, roll-forward, rollback-precondition, verification,
  and recovery-point preservation actions.
- [ ] **Desired:** Proven automatic rollback recipes per topology and domain.

## NixOS integration

- [x] **Finished:** NixOS module exposed by the flake.
- [x] **Finished:** Module options for steady-state resources plus imperative
  lifecycle declarations.
- [x] **Finished:** Generated `/etc/disk-nix/spec.json`.
- [x] **Finished:** Generated `/etc/disk-nix/steady-state.json`.
- [x] **Finished:** Declarative handoff index for native NixOS mounts, swap,
  LUKS, NFS exports, iSCSI boot/session state, and generated artifacts.
- [x] **Finished:** Module-managed apply and validate services with review
  scripts, JSON reports, and receipt files.
- [x] **Finished:** Assertions for duplicate active identities across supported
  storage domains.
- [x] **Finished:** Service enablement and boot/initrd integration hints for
  supported storage declarations.
- [ ] **Partial:** The module generates a reviewable
  `/etc/disk-nix/declarative-handoff.nix` Nix module snippet and
  `/etc/disk-nix/declarative-handoff-import.patch` review patch after
  evaluation, but automatic editing of the user's declarative NixOS
  configuration after successful imperative mutation is still not implemented.
- [x] **Finished:** Steady-state synthesis for lifecycle-managed resources after
  mutation. `/etc/disk-nix/steady-state.json` includes a `lifecycleManaged`
  index for active disk-nix declarations across supported storage domains,
  excluding teardown/export/logout/unmount style declarations and preserving
  stable identities, operations, and available target or desired-size details
  for post-mutation review.

## Testing and proof

- [x] **Finished:** Unit tests across model, probe, plan, exec, and CLI
  behavior.
- [x] **Finished:** Nix flake checks for package build, tests, clippy, module
  checks, examples, schema checks, completions, manpage output, docs freshness,
  and integration harness syntax.
- [x] **Finished:** Root-only opt-in smoke harnesses for loop-backed and
  selected lab-backed storage domains.
- [x] **Finished:** Smoke harness coverage for loop devices, Btrfs, bcachefs,
  LUKS, LVM, MD RAID, ZFS, NFS, VDO, iSCSI, multipath, NVMe, and synthetic
  failed-apply recovery.
- [ ] **Partial:** Broader destructive and failure-path integration tests are
  still needed for device replacement, rollback, degraded arrays, cache
  attach/detach, namespace creation/deletion, LUN login/logout flows, property
  mutation, and failed-command recovery beyond the synthetic
  LVM-plus-filesystem path.
- [ ] **Partial:** A VM smoke harness exists, but deeper destructive VM tests
  for multi-layer apply behavior on isolated disposable disks are still needed;
  the default VM suite now includes the synthetic failure-recovery harness.
- [x] **Finished:** Probe-status diagnostics include adapter remediation,
  structured OS, kernel, effective UID, tool-version context, and preflight
  checks for root privilege plus missing, failing, stderr-only, and empty-output
  storage tool version probes. Preflight JSON includes an `adapterRemediation`
  matrix for built-in adapters and sub-adapters with canonical domains, tools,
  likely Nix packages, privilege hints, data hints, parse-fixture hints, and
  manual command hints.

## Documentation

- [x] **Finished:** README with project goal, current status, CLI overview, and
  NixOS module entry point.
- [x] **Finished:** Architecture, CLI, planning, compatibility, status, storage
  scope, and integration-test documentation.
- [x] **Finished:** Field-level probe coverage documentation.
- [x] **Finished:** Feature status documentation.
- [x] **Finished:** Feature checklist for finished, partial, and desired work.
- [x] **Finished:** README and status documentation link to this checklist.
- [x] **Finished:** Operator runbooks for high-risk workflows such as
  replacement, rollback, recovery, degraded-array handling, and shared-storage
  changes.
