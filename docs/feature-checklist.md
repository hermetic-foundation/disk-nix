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
- [x] **Finished:** Real-world shared-storage fixture combines iSCSI session
  and node data, host-visible SCSI LUNs, and multipath paths.
- [x] **Finished:** Standalone open-iscsi fixture covers bracketed IPv6
  portals, concise node records, attached LUN disks, CHAP secret redaction, and
  iSER/RDMA session transport over bracketed IPv6 portals.
- [x] **Finished:** Degraded-MD-with-LUKS fixture covers recovering array
  state, failed member metadata, active encrypted mapper status, and LUKS
  header metadata.
- [x] **Finished:** Clustered LVM over NVMe-oF fixture covers shared/clustered
  VG metadata, sanlock lock hints, remote LV activity, NVMe fabrics paths, ANA
  state, and namespace-to-controller edges.
- [x] **Finished:** Fibre Channel multipath fixture covers FC transport WWPN pairs,
  SCSI unit names, ALUA path groups, active/standby path state, failed path
  metadata, and multipath backing edges.
- [x] **Finished:** NVMe/TCP multipath fixture covers native NVMe namespace
  paths, live and reconnecting fabrics controllers, ANA
  optimized/inaccessible states, failed path metadata, and multipath backing
  edges.
- [x] **Finished:** NFS server/client fixture covers merged `findmnt`,
  `nfsstat`, and `exportfs` state, negotiated Kerberos mount options, NFS
  export client policy, IPv6 export selectors, mount usage, and source-to-mount
  edges.
- [x] **Finished:** SAS enclosure fixture covers non-block SES enclosure
  records, enclosure identifiers, SAS addresses, and attached disk LUN backing
  edges.
- [x] **Finished:** LVM-backed VDO fixture merges native VDO status, vdostats
  usage, verbose VDO block counters, LVM VDO LV metadata, VDO segment policy,
  and backing-pool dependency edges.
- [x] **Finished:** Real-world physical Fibre Channel fixture coverage includes
  additional adapters, switch zoning-style fabric/WWPN layouts, ALUA states,
  and failed-path conditions.
- [x] **Finished:** Real-world NVMe-oF fixture coverage includes samples beyond
  the current NVMe/TCP multipath fixture for RoCE/RDMA, Fibre Channel transport,
  namespace sharing, ANA transitions, and controller loss/reconnect cases.
- [x] **Finished:** Real-world iSCSI fixture coverage includes multi-portal
  sessions, mutual CHAP, discovery authentication, replacement LUN identity
  changes, and logout/login churn.
- [x] **Finished:** Real-world server/client NFS fixture coverage includes
  NFSv4 referrals, pNFS, export reload behavior, client remount drift, and
  Kerberos policy variants.
- [x] **Finished:** Real-world clustered storage fixture coverage includes
  clustered LVM, shared VG locking, remote LV activity, and split-brain or
  lock-manager failure states.
- [x] **Finished:** Real-world hardware enclosure and array fixture coverage
  includes SAS enclosure variants, SES failures, vendor LUN metadata, and
  array-backed multipath identity drift.
- [x] **Finished:** Real-world VDO fixture coverage includes additional samples
  beyond the LVM-backed VDO fixture for physical-space pressure, index rebuild
  state, dedupe/compression policy drift, and VDO start/stop failure states.

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
- [x] **Finished:** Apply reports provide recovery guidance, current-topology
  roll-forward review, read-only rollback precondition review, and recovery
  point preservation actions.
- [x] **Finished:** Automatic rollback has a stable rollback recipe schema
  that separates read-only validation, reversible mutations, destructive
  mutations, and operator-only handoff steps.
- [x] **Finished:** Automatic rollback has an execution engine that replays
  only proven-safe reversible rollback steps after a failed apply and binds the
  result to the original receipt plus a fresh topology probe.
- [x] **Finished:** Automatic rollback replay refuses review-only,
  destructive, operator-only, not-ready, or unbound recipes before executing
  any command.
- [x] **Finished:** Automatic rollback recipes emit filesystem safety gates for
  ext, XFS, FAT, exFAT, NTFS, f2fs, mount/remount, trim, scrub, repair, grow,
  and shrink boundaries.
- [x] **Finished:** Automatic rollback recipes emit block-stack safety gates
  for disk labels, partitions, LUKS, LVM, MD RAID, device-mapper, loop devices,
  backing files, swap, and zram.
- [x] **Finished:** Automatic rollback recipes emit advanced-storage safety
  gates for ZFS, Btrfs, bcachefs, bcache, LVM cache, VDO, snapshots, clones,
  and pool membership changes.
- [x] **Finished:** Automatic rollback recipes emit network-storage safety gates
  for NFS, iSCSI, multipath, NVMe-oF, host-side LUNs, and target-side LUN
  providers.
- [x] **Finished:** Automatic rollback replay refuses reversible mutation
  commands whose metadata advertises already rolled-back, partially rolled-back,
  externally modified, rollback already applied, rollback partially applied, or
  diverged rollback topology states.
- [x] **Finished:** Automatic rollback replay refuses topology-derived
  idempotency diagnostics for already satisfied, already rolled-back, matched,
  available rollback point, and available clone-source states that are not
  already present in rollback command metadata.
- [x] **Finished:** Automatic rollback recipes declare required topology
  evidence labels for expected, pre-apply, failed-apply, and current topology
  identities, and replay receipts bind the supplied evidence IDs.
- [x] **Finished:** Automatic rollback replay refuses proven-safe recipes
  before command execution when required topology evidence bindings are missing
  or empty.
- [x] **Finished:** Automatic rollback replay can materialize deterministic
  topology evidence IDs for expected, pre-apply, failed-apply, and current
  replay bindings from the failed execution report plus a fresh probe ID.
- [x] **Finished:** Automatic rollback replay can bind full expected,
  pre-apply, failed-apply, and current topology payloads into
  `receiptBinding.topologyPayloads` through an explicit replay API instead of
  evidence IDs alone.
- [x] **Finished:** Automatic rollback replay refuses proven-safe recipes when
  the failed report's topology comparison summary already reports missing
  targets, size diagnostics, type conflicts, graph dependency conflicts, or
  partially suppressed reconciliation groups.
- [x] **Finished:** Automatic rollback replay refuses proven-safe recipes when
  detailed post-failure topology diagnostics report divergent mount/remount,
  NFS export, iSCSI session, host/target LUN, NVMe namespace, LVM activation,
  LUKS mapping, device-mapper, multipath, swap, loop, MD RAID, or VDO live-use
  state before command metadata is trusted.
- [x] **Finished:** Automatic rollback replay refuses proven-safe recipes when
  detailed post-failure topology diagnostics report missing rollback points,
  missing clone/rename sources, missing targets, mount source conflicts, loop
  conflicts, or pre-existing format targets before command metadata is trusted.
- [x] **Finished:** Automatic rollback replay refuses reversible mutation
  commands whose metadata advertises ambiguous rollback points, ambiguous
  rollback targets, missing rollback points, stale rollback points, stale
  identity data, or unbound rollback targets.
- [x] **Finished:** Automatic rollback replay has topology-derived refusal
  behavior for ambiguous rollback points and stale identity data that are not
  already present in rollback command metadata.
- [x] **Finished:** Automatic rollback replay refuses reversible mutation
  commands whose metadata advertises active consumers, mounted filesystems,
  exported LUNs, open encrypted mappings, active sessions, holders, or live
  mappings.
- [x] **Finished:** Automatic rollback replay has topology-derived refusal
  behavior for mounted filesystems, exported NFS/LUN state, active iSCSI/NVMe
  sessions, open encrypted mappings, device-mapper maps, multipath state,
  activated LVM state, swap, loop, MD RAID, and VDO live-use blockers that are
  not already present in rollback command metadata.
- [x] **Finished:** Automatic rollback replay refuses missing required tools
  before running read-only validation or reversible mutation commands.
- [x] **Finished:** Automatic rollback replay refuses reversible mutation
  commands whose argv or command metadata advertises plausible data-loss
  semantics beyond already-refused destructive and operator-only recipe
  sections.
- [x] **Finished:** Automatic rollback replay has topology-aware refusal
  behavior for domain-specific plausible data-loss paths including Btrfs
  subvolume/qgroup destroy, bcache/LVM-cache detach, LUKS keyslot/token remove,
  multipath destroy/path removal, swap destroy, MD member removal, snapshot
  destroy, VDO destroy, and ZFS object destroy diagnostics that cannot be
  proven from rollback command argv, notes, unresolved inputs, or provider
  capability metadata alone.

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
- [x] **Finished:** `targetLuns` model create, grow, map, unmap, remove,
  rescan, and property handoff actions.
- [x] **Finished:** Linux LIO target-side create/map/unmap/rescan renders
  concrete `targetcli` inventory, backstore, target, LUN mapping, ACL, target
  removal, reviewed backstore removal, and persistence commands.
- [x] **Finished:** LIO grow requests include native target/backstore
  inventory, and LIO write-cache property requests render concrete reviewed
  `targetcli ... set attribute emulate_write_cache=...` commands.
- [x] **Finished:** Linux tgt target-side create/map/unmap/rescan renders
  concrete `tgtadm` inventory, target, logical-unit, initiator-address
  bind/unbind, and target removal commands when the reviewed `targetId`/`tid`,
  `lun`, backing object, and ACL values are declared.
- [x] **Finished:** Linux tgt grow requests include native target inventory,
  and tgt property requests render concrete reviewed
  `tgtadm --mode logicalunit --op update --name ... --value ...` commands when
  `targetId`/`tid`, `lun`, property, and value are declared.
- [x] **Finished:** SCST target-side
  create/map/unmap/remove/rescan/grow/property requests render concrete
  reviewed `scstadmin` inventory, backing-device open/close, target, initiator
  group, initiator, LUN map/unmap, target enable/removal, `resync_dev`, LUN
  attribute, and persistence commands when the reviewed target IQN, backing
  object, LUN, optional group, and initiators are declared.
- [x] **Finished:** Provider handoffs carry declared `targetId`/`tid` and
  `lun` values.
- [x] **Finished:** LIO target-side LUN grow has a native reviewed block
  backstore path that validates backing capacity, refreshes LIO target/LUN
  inventory, persists target state, and verifies initiator-visible capacity.
- [x] **Finished:** LIO target-side LUN grow has a provider-specific forced
  fileio backstore resize primitive. Declarations can set
  `backstoreType = "fileio"` to emit a reviewed `truncate --size <desiredSize> <source>` step before target/LUN refresh, use `/backstores/fileio/...`
  inventory, validate the grown file with `stat --format=%s`, and keep block
  backstores on the non-destructive backing-capacity validation path.
- [x] **Finished:** tgt target-side LUN grow has a native reviewed refresh path
  that validates backing capacity, refreshes the exported logical unit with
  `tgtadm --mode logicalunit --op update --params online=1`, captures
  persistent-config state with `tgt-admin --dump`, and verifies
  initiator-visible capacity.
- [x] **Finished:** Other target provider adapters have a provider capability
  contract for create, grow, map, unmap, remove, rescan, property mutation,
  persistence, verification, and refusal behavior.
- [x] **Finished:** Array-backed target providers have concrete adapter models
  for vendor or site-specific LUN identity, capacity, mapping, masking, and
  snapshot or clone handoff data.
- [x] **Finished:** Target provider verification placeholders include executable
  probes for post-change target state, initiator visibility, multipath refresh,
  and consumer safety checks.
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
- [x] **Finished:** Failed apply reports include domain-specific recovery,
  current-topology roll-forward review, read-only rollback-precondition
  commands, verification actions, and recovery-point preservation actions.
- [x] **Finished:** Proven automatic rollback recipes have filesystem-level
  recipes and fixtures for grow, mount/remount, property mutation, scrub,
  repair, and failed-check recovery boundaries. Mount verification failures
  can replay a receipt-bound `umount`, remount failures can replay declared
  `rollbackOptions`, filesystem property failures can replay declared
  `rollbackValue`, and grow, scrub, repair, and failed-check boundaries emit
  refused/operator-only recipes because they have no generic data-preserving
  inverse.
- [x] **Finished:** Proven automatic rollback recipes have block-stack recipes
  and fixtures for partition, LUKS, LVM, MD RAID, device-mapper, loop,
  backing-file, swap, and zram mutation boundaries. Swap and LUKS identity
  property failures can replay declared `rollbackValue`, device-mapper rename
  and LUKS open verification failures can replay bounded inverse commands, and
  partition growth, LVM growth, MD RAID replacement, loop create, backing-file
  growth, swap deactivation failures, and zram generated-state mutation
  boundaries emit refused/operator-only recipes unless stronger topology proof
  is available.
- [x] **Finished:** Proven automatic rollback recipes have advanced-storage
  recipes and fixtures for ZFS, Btrfs, bcachefs, bcache, LVM cache, VDO, and
  snapshot or clone boundaries. ZFS, VDO, bcache, and Btrfs subvolume property
  failures can replay declared `rollbackValue`; ZFS/Btrfs rename verification
  boundaries have bounded inverse recipes; ZFS snapshot rollback/clone, VDO
  growth, bcache replacement, LVM cache mutation, Btrfs qgroup mutation, pool
  topology, and dataset/zvol lifecycle boundaries emit refused/operator-only
  recipes without stronger topology proof.
- [x] **Finished:** Proven automatic rollback recipes have network-storage
  recipes and fixtures for NFS, iSCSI, host-side LUNs, and target-side LUN
  providers. NFS remount and export option failures can replay declared
  `rollbackValue`; NFS mount and iSCSI login verification failures can replay
  bounded inverse unmount/logout commands; target-side LUN property failures can
  replay provider-specific declared `rollbackValue`; and NFS unmount/unexport,
  iSCSI logout, host LUN growth, target LUN growth, remote export lifecycle, and
  LUN topology boundaries emit refused/operator-only recipes without stronger
  initiator, target, and backing-store proof.
- [x] **Finished:** Automatic rollback recipes have crate-level integration
  proof that a failed apply report can bind fresh topology evidence and
  payloads, choose a proven rollback recipe, run read-only validation and
  reversible mutation steps, verify success, and emit a rollback receipt with
  original receipt, fresh topology, evidence, and payload bindings.
- [x] **Finished:** Automatic rollback recipes have negative tests proving
  refusal when rollback points are missing, stale, or not bound to the failed
  apply receipt.
- [x] **Finished:** Automatic rollback recipes have negative tests proving
  refusal when current topology differs from both expected and failed-apply
  topology in ways the recipe cannot prove safe.
- [x] **Finished:** Automatic rollback recipes have negative tests proving
  refusal when active consumers, mounted filesystems, open encrypted mappings,
  exported LUNs, or data-loss-prone operations make rollback unsafe.

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
- [x] **Finished:** Synthetic failed-command recovery covers layered
  LVM-plus-filesystem, LVM grow, LVM thin-pool create/grow, XFS grow, Btrfs
  scrub/rebalance/device replacement, bcachefs replacement, filesystem
  trim/check/repair/property, swap label, zram rescan/property inventory, loop
  rescan, backing-file rescan/grow/create, device-mapper rename, ZFS dataset
  rename, Btrfs/ZFS snapshot clone, LVM VG rename/replacement, ZFS pool
  replacement, and ZFS rollback paths.
- [x] **Finished:** Synthetic failed-command recovery covers NVMe namespace
  create/grow/attach/detach/delete, host-side LUN rescan, target-side LUN
  LIO/tgt/SCST lifecycle/property/rescan paths, and multipath
  add/remove/flush/resize/replace paths.
- [x] **Finished:** Synthetic failed-command recovery covers MD RAID
  create/assemble/stop/grow/add-member/remove-member/replace, LUKS
  open/format/close/grow/keyslot/token/property, partition grow, NFS
  remount/unmount/export/unexport, iSCSI logout/login/rescan, LVM cache
  attach/detach/replacement/rescan/property, VDO lifecycle/property, and bcache
  replacement/property/rescan paths.
- [ ] **Partial:** Destructive integration tests need real or VM-backed device
  replacement coverage for MD RAID, ZFS pools, Btrfs filesystems, bcachefs,
  bcache, LVM cache, and multipath-backed stacks.
- [ ] **Partial:** Destructive integration tests need broader degraded-array
  variants covering missing members, stale superblocks, replacement races,
  partial rebuilds, failed detach, and failed reattach behavior.
- [x] **Finished:** Destructive integration tests include MD RAID degraded
  missing-member coverage: the loop-backed MD harness creates a temporary RAID1
  array, fails and removes one member, verifies `disk-nix inspect` still sees
  degraded array metadata, and reruns the read-only MD rescan apply.
- [ ] **Partial:** Destructive integration tests need VM-backed cache mutation
  coverage for LVM cache attach/detach/replacement, bcache replacement, and
  cache-device failure states.
- [x] **Finished:** Destructive integration tests include real bcache read-only
  rescan coverage: the loop-backed bcache harness applies
  `caches.bcacheSmoke.operation = "rescan"` against the generated bcache
  device and verifies `disk-nix inspect` plus `disk-nix-bcache-read` checks for
  `state`, `cache_mode`, and `dirty_data` all succeed.
- [ ] **Partial:** Destructive integration tests need VM-backed NVMe namespace
  coverage for create, grow, attach, detach, delete, controller reconnect, and
  namespace identity drift.
- [x] **Finished:** Destructive integration tests include lab-backed multipath
  flush coverage: when `DISK_NIX_MULTIPATH_FLUSH=1` is set, the multipath
  harness applies `multipathMaps.flush.destroy = true` with
  `allowDestructive = true` and `backupVerified = true`, then verifies
  `multipath -ll <map>` and `multipath -f <map>` succeed.
- [x] **Finished:** Destructive integration tests include lab-backed multipath
  path add/remove coverage: when `DISK_NIX_MULTIPATH_ADD_PATH` or
  `DISK_NIX_MULTIPATH_REMOVE_PATH` is set, the multipath harness applies
  `multipathMaps.paths.addDevices` and/or
  `multipathMaps.paths.removeDevices` for the explicit paths and verifies
  `multipathd add path <path>` and `multipathd del path <path>` succeed.
- [x] **Finished:** Destructive integration tests include lab-backed multipath
  resize coverage: when `DISK_NIX_MULTIPATH_RESIZE=1` is set, the multipath
  harness applies `multipathMaps.resize.operation = "grow"` for the selected
  map and verifies `multipath -ll`, `lsscsi -t -s`,
  `multipathd resize map <map>`, and `multipath -r` all succeed.
- [x] **Finished:** Destructive integration tests include lab-backed host-side
  LUN rescan coverage: when `DISK_NIX_LUN_PATH` is set, the iSCSI harness
  applies `luns.<target>:0.operation = "rescan"` for that path and verifies
  `iscsiadm --mode session --rescan`, `disk-nix-scsi-rescan`, `lsscsi -t -s`,
  and `multipath -r` all succeed.
- [x] **Finished:** Destructive integration tests include LIO target-side
  map/unmap coverage: the loop-backed target LUN harness creates a second
  temporary backstore, applies `targetLuns.<iqn>.operation = "attach"` with a
  reviewed initiator ACL, verifies the LUN and ACL are present, applies
  `targetLuns.<iqn>.operation = "detach"`, and verifies the LUN is unmapped
  without deleting the backstore.
- [x] **Finished:** Destructive integration tests include target-side LUN
  destroy refusal coverage: the loop-backed LIO harness submits
  `targetLuns.<iqn>.destroy = true` without `allowDestructive = true`, verifies
  the plan is blocked as destructive before any command is rendered, and checks
  the review-policy recovery guidance prefers non-destructive alternatives.
- [x] **Finished:** Destructive integration tests include real filesystem
  property mutation coverage: the loop-backed ext4 harness applies a disk-nix
  `filesystems.*.properties.label` declaration, executes `e2label`, and verifies
  the resulting label on the disposable loop device.
- [x] **Finished:** Destructive integration tests include real LUKS header
  property mutation coverage: the loop-backed LUKS harness applies a disk-nix
  `luks.devices.*.properties.label` declaration, executes `cryptsetup config`,
  and verifies the resulting label with `cryptsetup luksDump` on the disposable
  loop-backed container.
- [x] **Finished:** Destructive integration tests include real Btrfs filesystem
  property mutation coverage: the loop-backed Btrfs harness applies a disk-nix
  `filesystems.*.properties.label` declaration, executes
  `btrfs filesystem label`, and verifies the resulting label on the mounted
  disposable Btrfs filesystem.
- [x] **Finished:** Destructive integration tests include real swap signature
  property mutation coverage: the loop-backed swap harness applies a disk-nix
  `swaps.*.properties.label` declaration, executes `swaplabel`, and verifies
  the resulting label with `blkid` on the disposable loop-backed swap
  signature.
- [x] **Finished:** Destructive integration tests include real ZFS pool
  property mutation coverage: the loop-backed ZFS harness applies a disk-nix
  `pools.*.properties.autotrim` declaration, executes `zpool set`, and verifies
  the resulting property with `zpool get` on the disposable loop-backed pool.
- [x] **Finished:** Destructive integration tests include real LVM cache
  property mutation coverage: the loop-backed LVM harness creates a disposable
  cached origin LV, applies a disk-nix `lvmCaches.*.properties.lvm.cache-mode`
  declaration, executes `lvchange --cachemode`, and verifies the resulting mode
  with `lvs`.
- [x] **Finished:** Destructive integration tests include real VDO volume
  property mutation coverage: the lab-target VDO harness applies a disk-nix
  `vdoVolumes.*.properties.writePolicy` declaration, executes
  `vdo changeWritePolicy`, and verifies the resulting policy with
  `vdo status --name` on the selected disposable VDO volume.
- [x] **Finished:** Destructive integration tests include real NFS export
  property mutation coverage: the NFS lab harness can opt into a server-side
  temporary export, applies a disk-nix `exports.*.properties.options`
  declaration, executes `exportfs -i`, and verifies the export with
  `exportfs -v`.
- [x] **Finished:** Destructive integration tests include real bcache property
  mutation coverage: the bcache harness creates disposable loop-backed backing
  and cache devices, applies a disk-nix
  `caches.*.properties."bcache.cache-mode"` declaration, executes the
  `disk-nix-bcache-property` sysfs write, and verifies `cache_mode` reports
  `writethrough`.
- [x] **Finished:** Destructive integration tests include real loop-device
  property mutation coverage: the loop harness applies
  `loopDevices.*.properties."loop.read-only"` to a disposable loop device,
  executes `blockdev --setro` and `blockdev --setrw`, and verifies the
  read-only state with `blockdev --getro`.
- [x] **Finished:** Destructive integration tests include real backing-file
  property mutation coverage: the loop harness applies
  `backingFiles.*.properties.mode` to its temporary backing image, executes
  `chmod 0600`, and verifies the mode with `stat`.
- [x] **Finished:** Destructive integration tests include real zram property
  reconciliation coverage: the zram harness applies
  `zram.properties.algorithm` and `zram.properties.priority`, verifies the
  `zram:set-property:*` actions stay non-mutating, executes real
  `zramctl --bytes --raw --noheadings --output-all`, `swapon --show`, and
  `disk-nix zram` inventory commands, and confirms the plan points operators to
  NixOS `zramSwap` reconciliation.
- [x] **Finished:** Destructive integration tests include real target-side LUN
  property mutation coverage: the LIO harness creates a temporary loop-backed
  block backstore and target LUN, applies
  `targetLuns.*.properties."lio.writeCache"`, executes
  `targetcli ... set attribute emulate_write_cache=0`, and removes the
  temporary target state during cleanup.
- [x] **Finished:** Destructive integration tests include VM-backed failure
  injection for a partially completed apply run: the layered VM harness performs
  a real `lvextend --resizefs`, then intentionally fails a real `xfs_growfs`
  against the ext4 mount instead of relying only on fake-tool synthetic command
  failures.
- [x] **Finished:** Default VM suite includes the synthetic failure-recovery
  harness.
- [x] **Finished:** Disposable partitioned loop/LUKS/LVM/ext4 layered VM grow
  harness executes one disk-nix apply run that grows the partition with
  `growpart`, resizes the LUKS mapper with `cryptsetup resize`, grows the LV
  with `lvextend --resizefs`, executes `resize2fs`, remounts with reviewed
  options, then unmounts/deactivates the stack, executes a disk-nix LUKS close
  plan, reopens the mapper, remounts the LV, and verifies sentinel data
  survived.
- [x] **Finished:** Deeper destructive VM tests include a multi-domain mutation
  scenario that combines partition growth, LUKS growth, LVM changes, filesystem
  growth, and mount/remount verification in one apply run.
- [x] **Finished:** Deeper destructive VM tests inject a command failure after a
  successful real mutating command and assert the recovery report includes
  completed action ids, failed action id, failed command, remaining action ids,
  completed mutating command count, recovery actions, and fresh-topology review.
- [x] **Finished:** Deeper destructive VM tests assert rollback-review behavior
  for the layered VM failed apply: read-only rollback precondition commands,
  recovery-point preservation guidance, refused rollback recipe status, required
  topology evidence, empty reversible/destructive mutation sections, and
  operator-only guidance instead of automated unsafe rollback.
- [x] **Finished:** Deeper destructive VM tests assert layered block/filesystem
  data survival across failed and resumed apply runs: after the injected
  `xfs_growfs` failure, the harness runs a resumed remount apply, verifies the
  sentinel remains readable, then closes/reopens the LUKS stack and verifies the
  sentinel again.
- [x] **Finished:** Deeper destructive VM tests include LVM cache data-survival
  assertions: the loop-backed LVM harness formats the cached origin as ext4,
  writes a sentinel file, mutates cache mode with `lvchange --cachemode`, and
  verifies the cache sentinel survives mutation and rescan plans.
- [ ] **Partial:** Deeper destructive VM tests still need data-survival
  assertions across failed and resumed apply runs for network-storage scenarios.
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
