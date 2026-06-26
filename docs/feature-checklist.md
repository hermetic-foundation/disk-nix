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
- [x] **Finished:** Migration maps documented unversioned legacy aliases into
  version `1` with a machine-readable legacy mapping matrix, applied-mapping
  audit trail, and `versionMigrations` contract that documents supported
  pre-version and version `1` normalization paths while rejecting unsupported
  future versions.

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
  including a cross-adapter shared-storage fixture that combines iSCSI session
  and node data, host-visible SCSI LUNs, and multipath paths, plus a
  standalone open-iscsi fixture covering bracketed IPv6 portals, concise node
  records, attached LUN disks, CHAP secret redaction, and iSER/RDMA session
  transport over bracketed IPv6 portals, plus a
  degraded-MD-with-LUKS fixture that combines recovering array state, failed
  member metadata, active encrypted mapper status, and LUKS header metadata,
  plus a clustered LVM over NVMe-oF fixture covering shared/clustered VG
  metadata, sanlock lock hints, remote LV activity, NVMe fabrics paths, ANA
  state, and namespace-to-controller edges, plus a Fibre Channel multipath
  fixture covering FC transport WWPN pairs, SCSI unit names, ALUA path groups,
  active/standby path state, failed path metadata, and multipath backing edges,
  plus an NVMe/TCP multipath fixture covering native NVMe namespace paths,
  live and reconnecting fabrics controllers, ANA optimized/inaccessible states,
  failed path metadata, and multipath backing edges,
  plus an NFS server/client fixture covering merged `findmnt`, `nfsstat`, and
  `exportfs` state, negotiated Kerberos mount options, NFS export client policy,
  IPv6 export selectors, mount usage, and source-to-mount edges,
  plus a SAS enclosure fixture covering non-block SES enclosure records,
  enclosure identifiers, SAS addresses, and attached disk LUN backing edges,
  plus an LVM-backed VDO fixture that merges native VDO status, vdostats usage,
  verbose VDO block counters, LVM VDO LV metadata, VDO segment policy, and
  backing-pool dependency edges.
  It still needs broader hardware, additional fabric variants, and clustered
  storage samples from more real systems.

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
- [x] **Finished:** Runtime graph-path ordering has coarse phases, dependency
  metadata, graph-derived order diagnostics, and recovery-aware reverse
  dependency edges (`recoveryDependsOn` and `recoveryUnblocks`) for complex
  multi-layer mutations.
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
  `targetcli` inventory, backstore, target, LUN mapping, ACL, target removal,
  reviewed backstore removal, and persistence commands; LIO grow requests now
  include native target/backstore inventory, LIO write-cache property requests
  render concrete reviewed
  `targetcli ... set attribute emulate_write_cache=...` commands, and LIO grow
  remains an explicit non-ready provider handoff. Linux tgt target-side
  create/map/unmap/rescan now renders concrete `tgtadm` inventory, target,
  logical-unit, initiator-address bind/unbind, and target removal commands when
  the reviewed `targetId`/`tid`, `lun`, backing object, and ACL values are
  declared; tgt grow requests now include native target inventory, tgt property
  requests render concrete reviewed
  `tgtadm --mode logicalunit --op update --name ... --value ...`
  commands when `targetId`/`tid`, `lun`, property, and value are declared, and
  tgt grow remains an explicit non-ready provider handoff. SCST target-side
  create/map/unmap/remove/rescan/grow/property requests now render concrete
  reviewed `scstadmin` inventory, backing-device open/close, target, initiator
  group, initiator, LUN map/unmap, target enable/removal, `resync_dev`, LUN
  attribute, and persistence commands when the reviewed target IQN, backing
  object, LUN, optional group, and initiators are declared.
  Provider handoffs carry declared `targetId`/`tid` and `lun` values. Other
  array/provider adapters still use provider-labeled non-ready commands and
  verification placeholders.
- [x] **Finished:** Multi-layer lifecycle groups such as LUN refresh,
  multipath refresh, partition growth, LUKS/LVM resize, and filesystem growth
  are exposed through graph-derived dependency ordering, reverse recovery edges,
  and `topologyComparison.lifecycleGroups` with connected action ids, edge
  counts, phases, directions, and grouped-review guidance.

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
- [x] **Finished:** Multi-action reconciliation emits
  `topologyComparison.reconciliationGroups` with shared identities, planned
  action ids, suppressed action ids, counts, and partially-suppressed group
  flags before command rendering. Group identities now include NFS export/client
  mount relationships, device-mapper consumers, backing-file/loop relationships,
  and host-visible LUN detach coverage. Dry-run reports with partially
  suppressed groups are not scriptable, and execute mode refuses them until the
  plan is re-reviewed against fresh topology or split.
- [x] **Finished:** Cross-domain grouped updates such as iSCSI LUN refresh,
  multipath refresh, partition growth, LUKS/LVM resize, and filesystem growth
  are represented by `topologyComparison.lifecycleGroups` after current-topology
  graph analysis, separate from suppression-oriented reconciliation groups.

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
- [x] **Finished:** The module generates a reviewable
  `/etc/disk-nix/declarative-handoff.nix` Nix module snippet and
  `/etc/disk-nix/declarative-handoff-import.patch` review patch after
  evaluation. It can also opt in to guarded automatic editing after successful
  imperative mutation with
  `services.disk-nix.apply.declarativeHandoff.autoImport.enable = true`, which
  requires `apply.execute = true`, backs up the configured NixOS configuration
  file, skips already-imported configurations, and applies the import patch.
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
  still needed for additional device replacement domains, broader degraded-array
  variants, additional cache variants, additional NVMe namespace variants,
  additional LUN flows, property mutation across more supported domains, and
  failed-command recovery beyond the synthetic LVM-plus-filesystem, LVM grow,
  XFS grow, Btrfs scrub, Btrfs rebalance, Btrfs device replacement,
  bcachefs replacement, filesystem trim, filesystem check,
  filesystem repair, filesystem property, swap label, device-mapper rename,
  ZFS dataset rename,
  Btrfs snapshot clone, ZFS snapshot clone, LVM VG rename, ZFS pool
  replacement, ZFS rollback, NVMe namespace create, NVMe namespace
  grow, NVMe namespace attach, NVMe namespace detach, NVMe namespace delete,
  target-side LUN LIO create, target-side LUN LIO attach, target-side LUN LIO
  detach, target-side LUN LIO destroy, target-side LUN LIO grow not-ready with
  concrete property rendering, target-side LUN LIO property, target-side LUN
  LIO rescan, target-side LUN tgt create, target-side LUN tgt attach,
  target-side LUN tgt detach, target-side LUN tgt destroy, target-side LUN tgt
  grow not-ready with concrete property rendering, target-side LUN tgt property,
  target-side LUN tgt rescan, target-side LUN SCST create, target-side LUN SCST
  attach, target-side LUN SCST detach, target-side LUN SCST destroy,
  target-side LUN SCST grow, target-side LUN SCST property, target-side LUN
  SCST rescan, host-side LUN rescan, multipath resize, multipath add,
  multipath remove, multipath flush, multipath replace, LVM VG replacement, ZFS pool
  replacement, MD RAID grow, MD RAID add-member, MD RAID remove-member, MD RAID replace,
  LUKS open, LUKS format, LUKS close, LUKS grow,
  LUKS keyslot add, LUKS token import, LUKS keyslot remove, LUKS token remove,
  partition grow,
  NFS remount, NFS unmount, NFS export, NFS unexport, iSCSI logout, iSCSI
  login, iSCSI rescan, LVM cache attach, LVM cache detach, LVM cache
  replacement, LVM cache rescan, VDO create,
  VDO rescan, VDO logical grow, VDO physical grow, VDO start, VDO stop, VDO
  remove, VDO property, bcache replacement, bcache property, bcache rescan, and
  LVM cache property paths.
- [ ] **Partial:** A VM smoke harness exists, but deeper destructive VM tests
  are still needed; the default VM suite now includes the synthetic
  failure-recovery harness and a disposable loop/LUKS/LVM/ext4 layered grow
  harness that executes `resize2fs` through disk-nix after an LV extension,
  then unmounts/deactivates the stack, executes a disk-nix LUKS close plan,
  reopens the mapper, remounts the LV, and verifies sentinel data survived.
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
