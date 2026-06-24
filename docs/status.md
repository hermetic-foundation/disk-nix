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
  module checks, example checks, schema checks, completions, and manpage output.
- Rust workspace split into model, probe, plan, exec, and CLI crates.
- Read-only topology graph with focused CLI views for devices, partitions,
  filesystems, complex filesystems, Btrfs, bcachefs, ZFS, volumes, pools,
  snapshots, mappings, encryption, caches, LVM, VDO, multipath, NVMe, RAID,
  loop devices, backing files, swap, zram, iSCSI, LUNs, NFS, mounts, network
  storage, identities, usage, and object inspection.
- JSON output contracts for topology, focused views, capabilities, schema,
  plan, apply, validate, and probe-status commands.
- Policy-classified planning for online, offline-required, destructive,
  potential-data-loss, reversible, safe, and unsupported actions.
- Guarded apply flow with dry-run reports, script generation, readiness
  summaries, manual-review markers, unresolved-input reporting, policy blocks,
  optional current-topology probing, and sequential execution of ready commands.
- Current-topology reconciliation suppresses safe no-op grow, shrink, and
  property actions when the graph proves they are already satisfied and no
  warning diagnostics are present.
- NixOS module options for steady-state resources plus imperative lifecycle
  declarations emitted into `/etc/disk-nix/spec.json`.
- NixOS assertions for duplicate active identities across mountpoints, swaps,
  LUKS mapper names, LUKS keyslot/token selectors, disk and partition targets,
  backing files, Btrfs subvolumes and qgroups, device-mapper maps, MD RAID,
  multipath, ZFS pools/datasets/zvols/snapshots, LVM PV/VG/LV/thin/cache
  identities, VDO volumes, loop devices, cache identities, iSCSI sessions, LUN
  host paths, NVMe namespaces, and NFS export path/client pairs.

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

- Integration tests that exercise real loop-backed block devices, LUKS, LVM,
  Btrfs, bcachefs, ZFS, MD RAID, multipath, iSCSI, NFS, VDO, and NVMe where
  host support is available.
- A VM-based destructive test harness that validates apply behavior on isolated
  disposable disks before trusting production mutations.
- More reconciliation logic against the current storage graph for additional
  operation types and multi-action groups before command rendering.
- Graph-derived dependency ordering for multi-layer changes. The planner now
  applies coarse layer ordering, but grouped changes such as iSCSI LUN refresh,
  multipath, partition growth, LUKS/LVM resize, and filesystem growth still need
  explicit dependency edges.
- More NixOS steady-state synthesis for lifecycle-managed resources after
  mutation, especially when imperative changes should update declarative mounts,
  crypttab, swap, NFS exports, iSCSI boot, or generated files.
- Recovery and rollback recipes for partially completed apply runs.
- Better privilege and tool availability diagnostics for every adapter and
  command renderer, including distributions where tools have different output
  formats.
- More real-world fixture coverage from diverse hardware, fabrics, filesystems,
  degraded arrays, encrypted stacks, and clustered or shared-storage setups.
- Deeper stability policy for JSON contracts, NixOS option compatibility, and
  migration between schema versions. The parser now validates version `1`, but
  future migration and deprecation policy still needs to be documented.
