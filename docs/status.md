# Feature status

`disk-nix` is no longer just a design sketch. The repository contains a working
Rust CLI, storage graph model, probe layer, lifecycle planner, guarded apply
renderer/executor, and NixOS module integration. It is still not feature
complete as a full disko replacement because the remaining work is mostly about
hardening real mutation paths, expanding integration coverage, and proving
behavior across real storage stacks.

## Implemented foundation

- AGPL-3.0-or-later licensing from the beginning.
- Nix flake packaging, development shell, formatting, clippy, tests, NixOS
  module checks, example checks, schema checks, opt-in loop integration harness
  packaging, completions, and manpage output.
- Rust workspace split into model, probe, plan, exec, and CLI crates.
- Read-only topology graph with focused CLI views for devices, partitions,
  filesystems, complex filesystems, Btrfs, bcachefs, ZFS, volumes, pools,
  snapshots, mappings, encryption, caches, LVM, VDO, multipath, NVMe, RAID,
  loop devices, backing files, swap, zram, iSCSI, LUNs, NFS, mounts, network
  storage, identities, usage, and object inspection.
- JSON output contracts for topology, focused views, capabilities, schema,
  plan, apply, validate, and probe-status commands.
- Compatibility policy for spec versions, migration expectations, JSON reports,
  human CLI text, NixOS options, generated artifacts, and safety invariants.
- Policy-classified planning for online, offline-required, destructive,
  potential-data-loss, reversible, safe, and unsupported actions.
- Machine-readable dependency-order metadata for planned actions, including
  build/mutate/teardown phase, lower-first or upper-first direction, and storage
  collection layer rank, plus inferred `dependsOn`/`unblocks` edges for
  declared adjacent-layer identities and direct or multi-hop probed graph paths
  when current topology comparison is enabled, plus warning diagnostics and a
  summary count for mixed-direction actions on the same current-topology graph
  path.
- Guarded apply flow with dry-run reports, script generation, readiness
  summaries, manual-review markers, unresolved-input reporting, policy blocks,
  renderer tool requirement inventories with PATH availability and per-tool
  package remediation hints, optional current-topology probing, missing-tool
  refusal before execution, and sequential execution of ready commands.
- Probe-status reports include structured issue categories and adapter-specific
  remediation hints for missing tools/packages, permission barriers, parse
  failures, inaccessible kernel/service data, and generic command failures.
  Preflight JSON also includes an `adapterRemediation` matrix for built-in
  adapters and sub-adapters with canonical domains, tools, likely Nix packages,
  privilege hints, data hints, parse-fixture hints, and manual command hints.
- Current-topology reconciliation suppresses safe no-op grow, shrink, iSCSI
  login/logout, disk partition-table create actions that already match the
  requested table label, existing partition creates that match declared size,
  partition growth with parseable byte-sized end targets, LVM
  logical-volume activation/deactivation, LUKS open, LUKS close, loop
  create/destroy, LUN attach/detach, NVMe namespace attach/detach,
  backing-file create/grow, LVM physical-volume create, swap
  deactivate/destroy, mount, unmount, remount, NFS export/unexport, VDO grow,
  VDO start, VDO stop, MD create/assemble/stop/member add/member removal/member
  replacement, multipath path add/removal, ZFS pool create/import, ZFS
  dataset/zvol create, Btrfs subvolume create, Btrfs qgroup create, LVM
  volume/thin-pool create, LVM volume-group create/import/export, LVM
  logical-volume/thin-pool/volume-group rename when the destination already
  exists and the source is absent, LVM rename sources whose destinations are
  also absent remain actionable as metadata review work, device-mapper rename
  when the destination mapper exists and the source is absent, device-mapper
  rename sources whose destinations are also absent remain actionable as mapper
  review work, ZFS dataset/zvol promote when no clone origin remains, ZFS
  dataset/zvol rename when the destination already exists and the source is
  absent, ZFS dataset/zvol rename sources whose destinations are also absent
  remain actionable as ZFS metadata review work, and property actions when the
  graph proves they are already satisfied and no warning diagnostics are
  present, including VDO property declarations reconciled
  against native `vdo.*` and LVM `lvm.vdo-*` metadata with boolean
  compression/deduplication normalization, and cache property declarations
  reconciled against bcache `bcache.*`, cache-set `bcache.set-*`, and LVM cache
  `lvm.*` metadata with cache-mode spelling normalization, bcache cache-set
  property plans render `/sys/fs/bcache/<set>` writes when `cacheSetUuid` is
  declared, bcache probes include UUID, block/bucket sizing, btree cache size,
  and cache read race counters when sysfs exposes them, Btrfs qgroup
  referenced/exclusive limit declarations reconciled against
  probed `btrfs.max-*` metadata with unlimited-value normalization, and ZFS
  pool/dataset/zvol property
  declarations reconciled against probed `zfs.*` and pool-scoped `zfs.pool-*`
  metadata with common on/off normalization, ZFS snapshot hold/release
  declarations reconciled against probed hold tag metadata,
  and filesystem identity property declarations reconciled against probed node
  identity plus filesystem label, UUID, FAT volume-ID, NTFS serial, and exFAT
  serial metadata aliases, swap identity property declarations reconciled
  against probed swap label and UUID metadata, and LUKS identity property
  declarations reconciled against probed LUKS label, subsystem, and UUID
  header metadata;
  absent LVM activation targets remain actionable while absent deactivation
  targets are suppressed as already inactive;
  absent NFS exports remain actionable as export-required work instead of
  generic missing-target diagnostics;
  absent mountpoints for local and NFS mount actions remain actionable as
  mount-required work instead of generic missing-target diagnostics;
  absent LUKS mapper opens remain actionable as LUKS open work while absent
  mapper closes are suppressed as already satisfied;
  matching filesystem format types are reported without suppressing destructive
  format actions so policy and confirmation gates still apply;
  swap format targets report existing swap metadata or non-swap node kinds
  without suppressing destructive format actions;
  LUKS format targets are matched by backing device and report existing header
  metadata or non-LUKS node kinds without suppressing destructive format
  actions;
  absent or inactive LVM activation targets, still-active LVM deactivation targets,
  physical-volume create targets without matching PV metadata or with duplicate
  or missing PV metadata, existing exported, partial, or missing-PV
  volume-group create targets, existing LVM volume, thin-pool, or ZFS zvol
  create targets with different or unknown current size, VDO create targets that
  already have VDO metadata or match another node kind, MD create targets that
  are not cleanly active, ZFS pool create targets that are not online and
  healthy, ZFS pool/dataset/zvol or Btrfs
  subvolume/qgroup create targets that match a different node kind,
  still-exported LVM volume-group imports, still-imported LVM volume-group
  exports, absent or inactive LUKS open targets, active LUKS close targets,
  absent LUKS keyslot/token removal containers, loop devices mapped to
  different backing files, backing-file create targets with different
  or unknown current size, still-mapped loop detach targets, absent LUN attach
  paths, visible LUN detach paths, absent NVMe namespace attach
  paths, visible NVMe namespace detach paths, absent, unknown, or below-target
  VDO grow targets, absent or non-normal VDO start modes, running VDO stop
  targets, active swap teardown targets, active or
  unknown-state MD stop targets, absent LVM cache origins, absent MD member-add
  devices, still-attached MD member-removal devices, incomplete MD member
  replacements, degraded or failed
  MD arrays, absent multipath path-add maps, still-attached multipath path
  removals, degraded ZFS pools,
  mount source mismatches, currently mounted unmount targets, published
  unexport targets, remount option differences, export client/option
  differences, and known iSCSI targets without logged-in sessions remain
  actionable warnings.
- NixOS module options for steady-state resources plus imperative lifecycle
  declarations emitted into `/etc/disk-nix/spec.json`, with a generated
  `/etc/disk-nix/steady-state.json` inventory of native NixOS mounts, swaps,
  zram, LUKS, supported filesystems, NFS exports, storage identities,
  network-storage identities, iSCSI settings, and storage service enablement.
  The steady-state inventory also includes a `declarativeHandoff` index for
  post-mutation review of native NixOS mount, swap, LUKS, NFS export, iSCSI,
  and generated artifact surfaces, plus `/etc/disk-nix/declarative-handoff.nix`
  as a reviewable Nix module snippet for copying steady-state declarations into
  real NixOS configuration.
  Module-managed apply and validate services can emit review scripts, JSON
  reports, and invocation-bound receipt files.
- Current-topology reconciliation for generated zram properties, including
  algorithm, stream count, disk size, memory limit, compression ratio, and
  active swap priority when `/dev/zram*` metadata is available. Zram property
  declarations are offline-required generator-reconciliation requests rather
  than direct live mutation commands.
- LUKS keyslot property updates distinguish key-file rotation from keyslot
  priority metadata. Priority changes render `cryptsetup config` with
  `prefer`, `normal`, or `ignore`, and current-topology reconciliation
  suppresses the action when probed keyslot priority already matches.
- NixOS assertions for duplicate active identities across mountpoints, swaps,
  LUKS mapper names, LUKS keyslot/token selectors, disk and partition targets,
  backing files, Btrfs subvolumes and qgroups, device-mapper maps, MD RAID,
  multipath, ZFS pools/datasets/zvols/snapshots, LVM PV/VG/LV/thin/cache
  identities, VDO volumes, loop devices, cache identities, iSCSI sessions, LUN
  host paths, NVMe namespaces, and NFS export path/client pairs.
- Explicitly enabled smoke integration harnesses for loop devices, Btrfs,
  bcachefs, LUKS, LVM, MD RAID, ZFS, NFS, VDO, iSCSI, multipath, NVMe, and
  synthetic failed-apply recovery. The self-contained loop-backed harnesses
  create disposable backing files or arrays, verify real `inspect --json`,
  execute reviewed apply plans, and clean up temporary devices. Lab-hardware
  harnesses for NFS, VDO, iSCSI, multipath, and NVMe require explicit
  environment-selected existing targets and exercise non-destructive refresh
  or remount paths. The failure-recovery harness uses fake storage tools to
  prove `partialExecutionRecovery`, failed-command receipts, roll-forward
  review, rollback review, snapshot-preservation, and domain-recovery
  reporting for both a layered LVM-plus-filesystem apply failure and a ZFS
  rollback failure, plus NVMe namespace create, grow, attach, detach, and
  delete failures, target-side LUN LIO create, attach, and destroy failures,
  Linux tgt create, attach, and destroy failures, multipath resize and
  replacement failures, MD RAID member replacement failure, LUKS open failure,
  partition grow failure, NFS remount failure, swap label mutation failure,
  device-mapper rename failure, ZFS dataset rename failure, Btrfs snapshot
  clone failure, ZFS snapshot clone failure, LVM VG rename failure, iSCSI
  logout and login failures, LVM cache attach and detach failures, VDO grow
  failure, VDO and bcache property mutation failures, and an LVM cache property
  mutation failure, without touching real block devices.

## Implemented probe coverage

Probe adapters normalize storage data from `lsblk`, `blkid`, `udevadm`,
`findmnt`, `parted`, `smartctl`, filesystem-specific metadata tools, Btrfs,
bcachefs, ZFS, LVM, VDO, device-mapper, LUKS, loop, zram, SCSI, iSCSI, NFS, MD
RAID, multipath, and NVMe tooling. Probe fixture coverage includes isolated
adapter fixtures and a cross-adapter shared-storage regression fixture that
combines iSCSI session/node data, host-visible SCSI LUNs, and multipath path
state in one normalized graph. Fixture coverage also includes a merged
degraded-MD-with-LUKS stack covering recovering array state, failed member
metadata, active encrypted mapper status, backing edges, and LUKS header
metadata, plus a clustered LVM over NVMe-oF fixture covering shared/clustered
VG metadata, sanlock lock hints, remote LV activity, NVMe fabrics controller
paths, ANA state, and namespace-to-controller edges. See
[storage-scope.md](storage-scope.md) for the
detailed field-level coverage.
See [feature-checklist.md](feature-checklist.md) for a checklist view of
finished, partial, and desired features.
See [operator-runbooks.md](operator-runbooks.md) for high-risk replacement,
rollback, recovery, degraded-array, and shared-storage workflows.

## Implemented lifecycle coverage

Lifecycle planning and command rendering cover creation, growth, shrink where
the storage domain supports it, checks, repair, scrub, trim, remount, mount,
unmount, import, export, login, logout, attach, detach, open, close, start,
stop, assemble, activate, deactivate, add/remove/replace device, add/remove
LUKS keys and tokens, property changes, rename, clone, promote, rollback, and
destroy across the supported domains where those operations make sense.
File-backed storage origins include guarded backing-file creation that refuses
to overwrite an existing path before rendering sparse-file growth.

Unsupported or unsafe requests are kept explicit. Examples include XFS shrink,
unsupported filesystem or Btrfs subvolume properties, unsupported VDO property
values, target-side LUN providers without concrete adapters, and actions whose
concrete identity or required input is not declared. These produce
machine-readable blocked actions, manual-review guidance, or non-ready command
plans instead of guessing. Target-side LUN provisioning is modeled through
`targetLuns`;
`provider = "lio"` now renders concrete Linux LIO `targetcli` inventory,
backstore, target, LUN mapping, ACL, target removal, reviewed backstore removal,
and persistence commands. `provider = "tgt"` or `"tgtadm"` renders concrete
Linux tgt `tgtadm` inventory, target creation/removal, logical-unit
creation/removal, and initiator-address bind/unbind commands when the reviewed
`targetId`/`tid`, `lun`, backing object, and ACL values are declared. Other
providers still use provider-labeled handoff commands and verification
placeholders until concrete adapters are added.

## Remaining for feature complete

- Broader destructive and failure-path integration tests beyond the smoke
  suite, including additional device replacement domains, broader degraded-array
  variants,
  additional cache variants, additional NVMe namespace variants, additional LUN
  flows, property mutation across more supported domains, and failed-command
  recovery beyond the synthetic LVM-plus-filesystem, swap label, device-mapper
  rename, ZFS dataset rename, Btrfs snapshot clone, ZFS snapshot clone, LVM VG
  rename, ZFS rollback, NVMe namespace create, NVMe namespace grow, NVMe
  namespace attach, NVMe namespace detach, NVMe namespace delete, target-side
  LUN LIO create, target-side LUN LIO attach, target-side LUN LIO destroy,
  target-side LUN tgt create, target-side LUN tgt attach, target-side LUN tgt
  destroy, multipath resize, multipath replace, MD RAID replace, LUKS open,
  partition grow, NFS remount, iSCSI logout, iSCSI login, LVM cache attach, LVM
  cache detach, VDO grow, VDO property, bcache property, and LVM cache property
  paths.
- A deeper VM-based destructive test harness that validates multi-layer apply
  behavior on isolated disposable disks before trusting production mutations.
- More reconciliation logic against the current storage graph for additional
  operation types and multi-action groups before command rendering. Topology
  comparison now emits `reconciliationGroups` so related actions sharing a
  target, backing object, portal, path, mountpoint, or parent identity expose
  planned and suppressed action ids plus partially-suppressed group flags before
  commands are rendered. Grouping also normalizes NFS client sources back to
  exported paths and device-mapper consumers back to mapper names so export/mount
  and dm/filesystem reconciliation are visible together. Partially suppressed
  groups now gate command rendering: dry-run reports remain reviewable, but
  generated shell scripts and `apply --execute` are refused until the plan is
  re-reviewed against fresh topology or split.
- Runtime graph-path dependency ordering for multi-layer changes. The planner
  now applies coarse layer ordering and reports inferred dependency edges from
  declared identities and direct or multi-hop current-topology graph paths,
  emits `graph-dependency-order` diagnostics for matched graph-derived action
  edges, warns when matched actions on the same graph path require mixed
  dependency directions, and emits `graphDependencyConflictResolutions` with
  build/update and teardown/recovery split-pass proposals. Dependency-order
  entries also include `recoveryDependsOn` and `recoveryUnblocks` reverse edges
  so grouped changes such as iSCSI LUN refresh, multipath, partition growth,
  LUKS/LVM resize, and filesystem growth have explicit partial-failure review
  ordering.
- Current-topology lifecycle grouping for multi-layer updates. The comparison
  report emits `lifecycleGroups` for connected action sets derived from
  dependency edges, including graph-derived LUN-to-filesystem growth paths. Each
  group records action ids, edge counts, graph-derived edge counts, phases,
  directions, and review guidance for applying the connected mutation as one
  ordered group or splitting it into independently verified passes.
- Guarded automatic editing of declarative NixOS configuration after successful
  mutation. The module emits a
  `declarativeHandoff` index for mounts, crypttab/LUKS, swap, NFS exports,
  iSCSI boot/session state, generated files, and a reviewable
  `/etc/disk-nix/declarative-handoff.nix` Nix module snippet plus a reviewable
  `/etc/disk-nix/declarative-handoff-import.patch` skeleton. When
  `services.disk-nix.apply.declarativeHandoff.autoImport.enable = true` and
  `apply.execute = true`, the apply service backs up the configured NixOS
  configuration file, skips already-imported configurations, and applies the
  generated import patch only after `disk-nix apply --execute` succeeds. It also emits a
  `lifecycleManaged` steady-state index for active disk-nix lifecycle
  declarations across filesystems, swap, LUKS, NFS, LVM, VDO, dm, MD RAID,
  multipath, ZFS, Btrfs, caches, LUNs, iSCSI sessions, disks, partitions, and
  NVMe namespaces with stable identities, operations, and available target or
  desired-size details for post-mutation review.
- Deeper domain-specific recovery and rollback recipes for partially completed
  apply runs. Apply reports now expose generic recovery actions, targeted
  failed-action domain recovery guidance, current-topology roll-forward review,
  read-only rollback precondition review for concrete risky actions, and
  `partialExecutionRecovery` sequencing with completed actions, failed action,
  failed command, retry/review actions, remaining actions, completed mutating
  command counts, and fresh-topology review notes. They also include
  ZFS/Btrfs snapshot lifecycle changes, ZFS pool/dataset/zvol lifecycle
  changes, LVM VG/volume/thin/PV changes, LUKS mapper/header/keyslot/token
  changes, filesystem lifecycle updates, cache lifecycle changes, swap
  signature/activation changes, disk and partition-table lifecycle changes, MD
  RAID member replacement, NVMe namespace, iSCSI session, VDO lifecycle, and
  multipath map recovery inspection, loop-device, backing-file, and
  device-mapper map recovery inspection, NFS export and client mount recovery
  inspection, plus receipt files that bind reports to their invocation
  metadata, but safe automated rollback remains out of scope until broader
  topology-specific recovery proofs exist.
- Live probe-status preflight validation against every distribution and
  privilege/container profile still needs broader validation. Probe reports now
  expose structured degradation categories, adapter-specific tool/package,
  privilege-surface, inaccessible-data, parse-fixture, and manual-command
  hints, plus opt-in `probe-status --preflight` OS, kernel, effective UID,
  storage tool version context, structured preflight checks for root privilege
  plus missing, failing, stderr-only, or empty-output storage tool version
  probes, and a machine-readable adapter remediation matrix for built-in
  adapters and sub-adapters.
- More real-world fixture coverage from diverse hardware, additional fabric
  variants, filesystems, degraded arrays, encrypted stacks, and clustered
  storage setups.
- Future incompatible spec versions are intentionally blocked until their
  contract exists. The parser validates version `1`, and `disk-nix migrate`
  emits a reviewable current-version normalization report, maps documented
  pre-version legacy aliases into version `1`, reports the supported legacy
  mapping matrix, records the mappings applied in a specific run, and exposes a
  machine-readable `versionMigrations` contract for supported source and target
  version paths.
