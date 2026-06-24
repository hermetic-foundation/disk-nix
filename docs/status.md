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
  when current topology comparison is enabled.
- Guarded apply flow with dry-run reports, script generation, readiness
  summaries, manual-review markers, unresolved-input reporting, policy blocks,
  renderer tool requirement inventories with PATH availability and per-tool
  package remediation hints, optional current-topology probing, missing-tool
  refusal before execution, and sequential execution of ready commands.
- Probe-status reports include structured issue categories and remediation
  hints for missing tools, permission barriers, parse failures, inaccessible
  kernel/service data, and generic command failures.
- Current-topology reconciliation suppresses safe no-op grow, shrink, iSCSI
  login, LVM logical-volume activation, LUKS open, LUKS close, mount, remount,
  NFS export, VDO start, VDO stop, MD assemble, ZFS pool import, LVM volume-group
  import/export, and property actions when the graph proves they are already
  satisfied and no warning diagnostics are present; inactive LVM objects,
  still-exported LVM volume-group imports, still-imported LVM volume-group
  exports, inactive LUKS open targets, active LUKS close targets, non-normal VDO
  start modes, running VDO stop targets, degraded or failed MD arrays, degraded
  ZFS pools, mount source mismatches, remount option differences, export
  client/option differences, and known iSCSI targets without logged-in sessions
  remain actionable warnings.
- NixOS module options for steady-state resources plus imperative lifecycle
  declarations emitted into `/etc/disk-nix/spec.json`, with a generated
  `/etc/disk-nix/steady-state.json` inventory of native NixOS mounts, swaps,
  zram, LUKS, supported filesystems, NFS exports, storage identities,
  network-storage identities, iSCSI settings, and storage service enablement.
- NixOS assertions for duplicate active identities across mountpoints, swaps,
  LUKS mapper names, LUKS keyslot/token selectors, disk and partition targets,
  backing files, Btrfs subvolumes and qgroups, device-mapper maps, MD RAID,
  multipath, ZFS pools/datasets/zvols/snapshots, LVM PV/VG/LV/thin/cache
  identities, VDO volumes, loop devices, cache identities, iSCSI sessions, LUN
  host paths, NVMe namespaces, and NFS export path/client pairs.
- A root-only, explicitly enabled loop-backed smoke integration harness that
  creates a temporary backing file, attaches a loop device, writes an ext4
  signature, verifies real `inspect --json`, executes a safe loop rescan apply,
  and cleans up the temporary device.

## Implemented probe coverage

Probe adapters normalize storage data from `lsblk`, `blkid`, `udevadm`,
`findmnt`, `parted`, `smartctl`, filesystem-specific metadata tools, Btrfs,
bcachefs, ZFS, LVM, VDO, device-mapper, LUKS, loop, zram, SCSI, iSCSI, NFS, MD
RAID, multipath, and NVMe tooling. See [storage-scope.md](storage-scope.md) for
the detailed field-level coverage.

## Implemented lifecycle coverage

Lifecycle planning and command rendering cover creation, growth, shrink where
the storage domain supports it, checks, repair, scrub, trim, remount, mount,
unmount, import, export, login, logout, attach, detach, open, close, start,
stop, assemble, activate, deactivate, add/remove/replace device, add/remove
LUKS keys and tokens, property changes, rename, clone, promote, rollback, and
destroy across the supported domains where those operations make sense.

Unsupported or unsafe requests are kept explicit. Examples include XFS shrink,
unsupported filesystem or Btrfs subvolume properties, unsupported VDO property
values, target-side LUN provisioning, and actions whose concrete identity or
required input is not declared. These produce machine-readable blocked actions,
manual-review guidance, or non-ready command plans instead of guessing.

## Remaining for feature complete

- Broader integration tests that exercise LUKS, LVM, Btrfs, bcachefs, ZFS, MD
  RAID, multipath, iSCSI, NFS, VDO, and NVMe where host support is available.
- A VM-based destructive test harness that validates apply behavior on isolated
  disposable disks before trusting production mutations.
- More reconciliation logic against the current storage graph for additional
  operation types and multi-action groups before command rendering.
- Runtime graph-path dependency ordering for multi-layer changes. The planner
  now applies coarse layer ordering and reports inferred dependency edges from
  declared identities and direct or multi-hop current-topology graph paths, but
  grouped changes such as iSCSI LUN refresh, multipath, partition growth,
  LUKS/LVM resize, and filesystem growth still need conflict handling and
  recovery-aware ordering.
- More NixOS steady-state synthesis for lifecycle-managed resources after
  mutation, especially when imperative changes should update declarative mounts,
  crypttab, swap, NFS exports, iSCSI boot, or generated files.
- Deeper domain-specific recovery and rollback recipes for partially completed
  apply runs. Apply reports now expose generic recovery actions and targeted
  failed-action domain recovery guidance for concrete risky actions, but safe
  rollback and roll-forward recipes still need broader current-topology
  awareness.
- Deeper privilege and tool availability diagnostics for every adapter and
  command renderer, including distributions where tools have different output
  formats. Probe reports now expose structured degradation categories, but
  adapter-specific privilege checks and distribution-specific tool output
  checks still need expansion.
- More real-world fixture coverage from diverse hardware, fabrics, filesystems,
  degraded arrays, encrypted stacks, and clustered or shared-storage setups.
- Implemented migration tooling and tests for future spec versions. The parser
  now validates version `1` and the compatibility policy documents migration and
  deprecation expectations, but no future-version migrator exists yet because no
  version `2` contract exists.
