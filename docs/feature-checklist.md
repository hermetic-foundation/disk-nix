# Feature checklist

This checklist tracks the desired full disko-replacement scope against the
current implementation. Checked items are implemented in the repository today;
partial items have usable support but still need hardening, broader coverage, or
production proof.

## Foundation

- [x] AGPL-3.0-or-later license from project start
- [x] Rust workspace split into model, probe, plan, exec, and CLI crates
- [x] Nix flake with package, dev shell, checks, formatting, tests, clippy,
  NixOS module checks, examples, completions, and manpage generation
- [x] Machine-readable JSON contracts for topology, focused views,
  capabilities, schema, plan, apply, validate, migrate, and probe status
- [x] Spec compatibility policy for JSON reports, NixOS options, generated
  artifacts, safety invariants, and future migrations
- [x] CLI subcommands for focused storage views and object inspection
- [x] Parser and fixture tests for probe adapters
- [ ] Version-to-version spec migration mappings beyond version `1`

## Read-only storage awareness

- [x] Block devices, partitions, partition tables, IDs, labels, UUIDs, sizes,
  capacity, free space, usage, and metadata
- [x] Mounts, filesystems, filesystem-specific metadata, checks, repair
  support surfaces, and usage accounting
- [x] Complex filesystems: Btrfs, bcachefs, and ZFS
- [x] Btrfs filesystems, devices, subvolumes, snapshots, qgroups, properties,
  usage, and device topology
- [x] bcachefs filesystems, member devices, usage, scrub/fsck surfaces, and
  topology updates
- [x] ZFS pools, vdevs, datasets, zvols, snapshots, holds, properties, health,
  cache/log/special vdev roles, and error counters
- [x] LVM PVs, VGs, LVs, thin pools, snapshots, cache, writecache, and VDO
  metadata
- [x] LUKS headers, UUIDs, labels, subsystems, keyslots, tokens, mapper state,
  and device-mapper backing data
- [x] Device-mapper maps, tables, status, and local mappings
- [x] Cache layers including bcache, bcache cache sets, LVM cache, LVM
  writecache, and cache-relevant metadata
- [x] VDO native and LVM-backed metadata, logical/physical sizing,
  compression, deduplication, operating mode, and statistics
- [x] MD RAID arrays, members, states, degraded/failed indicators, and
  replacement surfaces
- [x] Multipath maps, paths, policies, handlers, features, and degraded state
- [x] NVMe controllers, namespaces, attachments, health, formatted LBA, and
  namespace capacity metadata
- [x] SCSI devices, host-visible LUNs, iSCSI sessions, targets, portals, and
  login state
- [x] NFS exports, NFS client mounts, sources, servers, mount options, and
  negotiated state
- [x] Loop devices, backing files, swap, zram, SMART telemetry, and network
  storage identity views
- [ ] Broader real-world fixture coverage from diverse hardware, fabrics,
  degraded arrays, encrypted stacks, clustered storage, and shared storage

## Planning and apply safety

- [x] Policy classification for safe, reversible, online, offline-required,
  destructive, potential-data-loss, and unsupported actions
- [x] Guarded dry-run apply reports with readiness summaries and manual-review
  markers
- [x] Script generation for reviewed command plans
- [x] Missing-tool refusal before execution with package remediation hints
- [x] Per-command mutating/read-only metadata
- [x] Unresolved-input reporting for actions missing concrete required inputs
- [x] Policy blocks for unsupported or unsafe requests instead of guessing
- [x] Receipt files that bind apply reports to invocation metadata
- [x] Sequential execution of ready commands
- [x] Dependency-order metadata for build, mutate, and teardown phases
- [x] Inferred dependency edges from declared adjacent-layer identities and
  probed graph paths
- [ ] Runtime recovery-aware dependency ordering for complex multi-layer
  mutations
- [ ] Stronger conflict resolution for mixed-direction changes on the same
  current-topology graph path
- [ ] Production-grade automatic rollback. Current reports provide guidance;
  safe automated rollback remains intentionally unimplemented.

## Lifecycle operations

- [x] Create, grow, shrink where supported, check, repair, scrub, trim, mount,
  remount, unmount, import, export, login, logout, attach, detach, open, close,
  start, stop, assemble, activate, deactivate, add/remove/replace device,
  add/remove LUKS keys and tokens, property changes, rename, clone, promote,
  rollback, and destroy where those operations make sense
- [x] Filesystem lifecycle for ext, XFS, Btrfs, bcachefs, F2FS, exFAT, NTFS,
  FAT, and swap where supported by the domain
- [x] Filesystem identity updates for labels, UUIDs, FAT volume IDs, NTFS
  serials, exFAT serials, and related metadata
- [x] LVM PV/VG/LV/thin/snapshot/cache/writecache/VDO lifecycle planning and
  command rendering
- [x] ZFS pool, dataset, zvol, snapshot, hold, clone, promote, rollback, and
  property lifecycle planning
- [x] Btrfs device, subvolume, snapshot, qgroup, rebalance, scrub, property,
  and filesystem resize lifecycle planning
- [x] bcachefs device resize/add/remove, rereplicate, scrub, and fsck planning
- [x] LUKS format/open/close/header/keyslot/token lifecycle planning
- [x] MD RAID create/assemble/stop/member add/removal/replacement planning
- [x] NVMe namespace attach/detach and rescan planning
- [x] iSCSI discovery, login, logout, rescan, and session planning
- [x] Host-side LUN attach/detach/grow/rescan planning
- [x] NFS export/unexport and client mount/remount/unmount planning
- [x] VDO create/remove/grow/start/stop/property planning
- [x] Multipath map/path add/removal/rescan planning
- [x] Loop, backing-file, swap, zram, cache, and device-mapper lifecycle
  planning
- [ ] Target-side LUN provisioning
- [ ] Unsupported domain operations such as XFS shrink or unsupported property
  values remain blocked unless a safe implementation is added

## Current-topology reconciliation

- [x] Suppression of many already-satisfied create, grow, import, export,
  login, logout, attach, detach, mount, unmount, remount, start, stop, open,
  close, activate, deactivate, rename, promote, and property actions
- [x] Reconciliation for LVM activation/deactivation, PV/VG/LV/thin/cache
  state, VDO grow/start/stop/properties, cache properties, ZFS properties and
  holds, Btrfs qgroups, filesystem identities, swap identities, LUKS header
  identities, loop devices, backing files, MD members, multipath paths, NFS
  exports/mounts, iSCSI sessions, LUNs, and NVMe namespace visibility
- [x] Actionable warnings for unsafe or ambiguous current state instead of
  silently suppressing mutations
- [ ] More reconciliation for multi-action groups before command rendering
- [ ] More cross-domain reconciliation for grouped updates such as iSCSI LUN
  refresh, multipath refresh, partition growth, LUKS/LVM resize, and
  filesystem growth

## Recovery guidance

- [x] Generic recovery actions for failed apply runs
- [x] Targeted failed-action domain recovery guidance
- [x] Current-topology roll-forward review commands
- [x] Read-only rollback precondition review commands for concrete risky
  actions
- [x] Recovery inspection for ZFS/Btrfs snapshots, ZFS pools/datasets/zvols,
  LVM PV/VG/LV/thin, LUKS mapper/header/keyslot/token, filesystem lifecycle,
  caches, swap, disks, partition tables, MD member replacement, NVMe
  namespaces, iSCSI sessions, VDO, multipath, loop devices, backing files,
  device-mapper maps, NFS exports/client mounts, and host-visible LUN detach
- [ ] Deeper domain-specific recovery recipes for partially completed
  multi-layer apply runs
- [ ] Proven automatic rollback recipes per topology and domain

## NixOS integration

- [x] NixOS module exposed by the flake
- [x] Module options for steady-state resources plus imperative lifecycle
  declarations
- [x] Generated `/etc/disk-nix/spec.json`
- [x] Generated `/etc/disk-nix/steady-state.json`
- [x] Declarative handoff index for native NixOS mounts, swap, LUKS, NFS
  exports, iSCSI boot/session state, and generated artifacts
- [x] Module-managed apply and validate services with review scripts, JSON
  reports, and receipt files
- [x] Assertions for duplicate active identities across supported storage
  domains
- [x] Service enablement and boot/initrd integration hints for supported
  storage declarations
- [ ] Automated editing or generation of declarative NixOS configuration after
  successful imperative mutation
- [ ] More steady-state synthesis for lifecycle-managed resources after
  mutation

## Testing and proof

- [x] Unit tests across model, probe, plan, exec, and CLI behavior
- [x] Nix flake checks for package build, tests, clippy, module checks,
  examples, schema checks, completions, and manpage output
- [x] Root-only opt-in smoke harnesses for loop-backed and selected lab-backed
  storage domains
- [x] Smoke harness coverage for loop devices, Btrfs, bcachefs, LUKS, LVM, MD
  RAID, ZFS, NFS, VDO, iSCSI, multipath, and NVMe
- [ ] Broader destructive and failure-path integration tests for device
  replacement, rollback, failed-command recovery, degraded arrays, cache
  attach/detach, namespace creation/deletion, LUN login/logout flows, and
  property mutation
- [ ] VM-based destructive test harness for multi-layer apply behavior on
  isolated disposable disks
- [ ] Live preflight checks across distributions and tool-output variants

## Documentation

- [x] README with project goal, current status, CLI overview, and NixOS module
  entry point
- [x] Architecture, CLI, planning, compatibility, status, storage scope, and
  integration-test documentation
- [x] Field-level probe coverage documentation
- [x] Feature status documentation
- [x] Feature checklist for finished, partial, and desired work
- [ ] Operator runbooks for high-risk workflows such as replacement, rollback,
  recovery, degraded-array handling, and shared-storage changes
