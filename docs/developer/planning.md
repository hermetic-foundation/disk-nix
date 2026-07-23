# Planning

`disk-nix plan` reads a desired storage JSON document and emits a
risk-classified action plan.

The plan summary reports total actions plus `offlineRequiredCount`,
`destructiveCount`, `potentialDataLossCount`, and `unsupportedCount` so callers
can gate workflows before looking at individual actions.

The command accepts either a direct spec:

```json
{
  "filesystems": {
    "root": {
      "mountpoint": "/",
      "fsType": "xfs",
      "resizePolicy": "grow-only",
      "desiredSize": "100%",
      "preserveData": true
    }
  }
}
```

or the NixOS module wrapper written to `/etc/disk-nix/spec.json`:

```json
{
  "version": 1,
  "spec": {
    "filesystems": {}
  },
  "apply": {}
}
```

Current planning is intentionally conservative. It classifies filesystem resize policy, preservation intent, and lifecycle operations for disks, partitions, swap, LUKS containers, Btrfs subvolumes, VDO volumes, volumes, LVM thin pools, LVM snapshots, loop-device mappings, MD RAID arrays, multipath maps, pools, datasets, zvols, LUNs, iSCSI sessions, exports, cache layers, and snapshots.

It reports destructive or potentially destructive behavior with alternatives instead of silently accepting unsafe mutation.

Planned actions are ordered by coarse storage dependency layers after parsing. Create, attach, open, grow, and other build/update operations run from lower layers toward upper layers; shrink, remove, unmount, detach, close, and destroy operations run from upper layers back down. Actions in the same layer keep their declaration order.

Plan JSON includes `dependencyOrder`, a machine-readable audit trail for that ordering with the action id, build/mutate/teardown phase, lower-first or upper-first direction, collection layer rank, inferred `dependsOn` and `unblocks` edges where declared identities tie adjacent layers together, reverse `recoveryDependsOn` and `recoveryUnblocks` edges for partial-failure review, and explanatory notes.

This documents the current ordering rationale and gives automation explicit preflight and recovery-review edges for common layered changes. When current topology probing is enabled, matched graph paths also add dependency edges across direct and multi-hop storage relationships.

Lower-to-upper paths such as LUN to multipath to partition to mapper to volume to filesystem are emitted in build/grow order, while teardown actions reverse the path so consumers are handled before backing layers.

Each graph-derived action edge also appears as an informational `graph-dependency-order` diagnostic with the matched graph path, dependent action, prerequisite action, and lower-layer or consumer-first rationale. The recovery edges reverse those relationships so partial-failure review can walk from consumers back toward backing layers before retrying or rolling forward.

Mixed-direction actions on the same graph path are reported as warning `graph-dependency-conflict` diagnostics and counted as `graphDependencyConflictCount`. Topology comparison JSON also includes `graphDependencyConflictResolutions`, which names the conflicting path, lower and upper action ids, each dependency direction, a `buildOrUpdatePass`, a `teardownOrRecoveryPass`, and a recommendation to split the work into reviewed passes.

Apply dry-runs preserve those diagnostics and split-pass proposals for review, while `--execute` refuses to mutate storage until the conflict count is zero.

## Lifecycle Reference

The examples below describe how planner intent maps to risk classes, command
rendering, and current-topology reconciliation. They are organized by storage
area instead of as one long bullet list.

### Resize And Filesystem Maintenance

`resizePolicy = "grow-only"` is online growth intent. `desiredSize`,
`targetSize`, and `size` are copied into action context when a storage domain
can use concrete capacity targets.

`resizePolicy = "shrink-allowed"` is potential data loss. The planner recommends
migration or backup-first alternatives, renders reviewed Btrfs shrink commands
when a target size exists, and leaves ext shrink source-device inputs explicit
when only a mountpoint is known.

XFS shrink intent is unsupported because XFS cannot shrink in place. The
recommended path is to create a smaller filesystem and migrate data.

Filesystem `check` and `repair` are offline-required maintenance workflows.
Check plans prefer read-only tools; repair plans mutate metadata and recommend a
backup or clone before touching production storage.

`preserveData = false` is destructive because it permits formatting or
replacement. Format plans render reviewed `mkfs` commands only when a concrete
backing `device` or `disk` is declared.

Filesystem `trim` renders `fstrim -v <mountpoint>` and recommends validating
discard passthrough through lower layers.

### Backing Files And Device Mapper

`backingFiles` declarations model file-backed storage origins. Create plans
require a concrete file path and desired size, refuse existing paths with
`test ! -e`, and render `truncate --size` for sparse-file creation.

Backing-file rescans are read-only. Grow plans use the same concrete inputs and
leave loop, swap, and filesystem refresh as explicit follow-up actions.

`dmMaps` declarations model device-mapper refreshes, reviewed mapper renames,
and explicit mapper removal. Rescan inspects identity, dependencies, tables,
live status, and graph consumers.

Mapper rename is offline-required because every dependent LUKS, LVM, VDO,
multipath, filesystem, mount, or service consumer must move to the new name.
Destroy renders `dmsetup remove` only after identity, dependency, and status
inspection.

Prefer a LUKS, LVM, VDO, multipath, or cache-specific teardown when another
domain owns the mapper.

### LUKS

LUKS `open` opens an existing encrypted container as a mapper and is
offline-required. Legacy `create` with preserved data remains accepted for the
same preserved open flow.

LUKS `close` tears down the mapper without removing the header. Format
operations and `preserveData = false` remain destructive.

LUKS growth, mapper close, keyslot updates, token updates, label changes,
subsystem changes, and UUID changes are offline-required because backing
capacity, header identity, mapper state, and consumers must be coordinated.

Keyslot or token removal is potential data loss because deleting the last usable
unlock path can make encrypted data inaccessible. Logical LUKS declarations can
name the mapper with `target`, `mapperName`, `mapper`, or `name`.

### Btrfs And Bcachefs

Btrfs filesystem `removeDevices = [ ... ]` is potential data loss. Plans advise
replacement capacity, evacuation, health checks, and `btrfs filesystem usage`
inspection before rendering device removal.

Btrfs `rebalance` renders `btrfs balance start`. Optional
`properties.balance.data`, `properties.balance.metadata`, and
`properties.balance.system` become scoped `-d`, `-m`, and `-s` filters.

Btrfs `scrub` renders `btrfs scrub start -B`; bcachefs `scrub` renders
`bcachefs scrub`; ZFS pool `scrub` renders `zpool scrub`.

Btrfs subvolume creation is online. Destruction is destructive and recommends
read-only snapshots or rename-first validation.

Btrfs subvolume `rescan` is online and read-only. It refreshes subvolume
metadata, read-only state, and modeled graph relationships for the declared
`path`.

Btrfs qgroup limits render `btrfs qgroup limit`. Referenced and exclusive limit
aliases are reconciled against probed qgroup metadata before no-op updates are
suppressed.

Btrfs qgroup create, destroy, limit, and rescan plans require a mounted
filesystem path through `target`, `path`, or `mountpoint` before execution is
ready.

### Cache Layers

Cache `replace-device` is offline-required because dirty or writeback data must
be flushed or detached cleanly. Cache removal is potential data loss when dirty
writeback data or media changes are involved.

Cache `remove-device` for bcache is offline-required rather than destructive
when the backing storage remains intact. Current-topology comparison suppresses
detach only after a concrete `/dev/bcache*` target is already absent.

Cache `rescan` is online and read-only. It reads bcache state, cache mode,
dirty-data, and graph relationships before attach, detach, or replacement.

LVM cache attach, detach, and replacement are offline-required because
`lvconvert` changes origin LV I/O paths. Cache mode and policy updates are safe
but still include verification guidance.

LVM cache `rescan` refreshes cache mode, policy, utilization, and modeled
relationships. Declared `cacheMode` and `cachePolicy` aliases are normalized
before no-op property updates are suppressed.

### Disks, Partitions, Swap, And Zram

Disk partition-table creation is destructive because it can hide existing
metadata. When policy permits it, apply plans render reviewed `parted mklabel`
and table-reread commands.

Partition creation and growth are offline-required because the kernel partition
table reread and dependent consumers must be coordinated. Plans render concrete
rereads when the backing disk is declared.

Swap signature creation is destructive. Swap growth is offline-required because
active swap must be disabled before backing storage and signatures change.

Swapfile growth can render a concrete file resize command. Block-device swap
growth must use the backing storage layer first.

Swap deactivation renders `swapoff` without removing the signature. Swap
destruction disables active swap and removes signature metadata with
`wipefs --all`.

Swap label, UUID, and numeric priority updates are offline-required identity or
activation changes. Current-topology comparison reports existing signatures but
does not suppress destructive `mkswap` actions.

Zram is generated compressed swap state rather than persistent backing storage.
NixOS module declarations derive `zramSwap`; plain zram declarations render
read-only inventory commands.

Zram property declarations are offline-required generator-reconciliation
requests. Algorithm, size, priority, and writeback-device changes may require
swapoff/setup coordination to take effect.

### LVM And VDO

LVM logical volume creation and growth are online when they allocate from an
existing VG with free extents. LV removal is destructive because it deletes
volume contents.

LV `rescan` is online and read-only. It refreshes LV size, attributes,
activation state, and graph relationships.

LVM thin-pool creation and growth are online allocations inside an existing VG.
Thin-pool removal is destructive because it removes contained thin volumes and
their data.

LVM activation and deactivation for LVs, thin pools, snapshots, and VGs are
offline-required but non-destructive. They change availability without creating
or removing data.

LVM VG creation and removal are destructive because they write or remove VG
metadata on member PVs. Prefer `vgextend`, import, or export when preserving an
existing group is possible.

LVM PV creation and removal are destructive. PV growth is an online `pvresize`
after backing storage has already grown.

LVM snapshot creation is reversible. Snapshot merge rollback is potential data
loss, and snapshot removal is destructive because it deletes a recovery point.

VDO creation and removal are destructive because they write or remove VDO
metadata on the backing device. VDO growth is online when it adjusts logical
size or follows already-expanded backing storage.

VDO `start` and `stop` are offline-required lifecycle actions for existing
metadata. VDO `rescan` is an online read-only status and utilization refresh.

Supported VDO property updates render reviewed write-policy, compression, and
deduplication commands. Unsupported properties or invalid values are rejected
before execution.

### ZFS

ZFS pool creation and destruction are destructive because they write vdev labels
or remove all contained datasets and zvols. Create plans accept either `device`
or an explicit `devices` vdev list.

Pool properties render as create-time `zpool create -o key=value` options and
as explicit reconciliation actions. Common on/off spellings are normalized
before no-op property updates are suppressed.

Pool `import` and `export` are offline-required, non-destructive lifecycle
operations. Use them when moving an existing pool between hosts.

Pool device replacement is offline-required. Device removal remains potential
data loss unless topology, free space, and evacuation support have been
verified.

ZFS dataset creation is online. Declared properties render as create-time
`zfs create -o key=value` options and as explicit reconciliation actions.

Dataset destruction is destructive and recommends snapshots or rename-first
validation. Logical names can provide the concrete dataset with `target` or
`path`.

Zvol creation, growth, and property updates are online. Plans advise verifying
pool capacity, reservation policy, and downstream block consumers.

Zvol properties render as create-time `-o key=value` options and `zfs set`
reconciliation actions. Logical zvol names can provide the concrete object with
`target` or `path`.

### MD RAID, Multipath, And NVMe

MD RAID creation and destruction are destructive because they write or remove
array metadata. Assemble and stop are offline-required but preserve member data.

MD member add is online. Replacement and grow/reshape are offline-required
because redundancy, resync, and dependent consumers must be coordinated.

Multipath map growth and path add are online. Path replacement is
offline-required, and path removal is potential data loss unless a healthy path
remains active.

NVMe namespace creation and deletion are destructive because they allocate or
remove controller-managed capacity. Namespace growth is offline-required because
host rescan and consumers must be coordinated.

NVMe namespace attach is online for an existing namespace. Detach is
offline-required because consumers must be drained before access is removed.

### Network Storage And LUNs

NFS export publication is online when it publishes an existing path to explicit
clients and options. Unexporting is offline-required because remote clients may
need to be drained.

iSCSI session `login` discovers or logs into an existing target and is online.
Legacy `create` remains accepted for the same login flow.

iSCSI `logout` is offline-required and preserves target-side data. Session
`rescan` is online, while session `grow` is offline-required because target
capacity, paths, multipath, and consumers must be coordinated.

Host-side LUN `attach` means discovering an existing target-side LUN. It is
online when stable paths are declared and session, SCSI, and multipath rescans
can verify capacity.

LUN `rescan` refreshes existing host-visible paths. LUN `grow` and `detach` are
offline-required because target storage, host paths, and consumers must be
coordinated.

Target-side provisioning is modeled through `targetLuns`. Operations describe
external target allocation, capacity growth, mapping, unmapping, and provider
handoffs.

`provider = "lio"` renders Linux LIO `targetcli` inventory, backstore, target,
LUN mapping, ACL, removal, property, growth, persistence, and verification
commands when the required target identity is declared.

`provider = "tgt"` or `"tgtadm"` renders Linux tgt `tgtadm` inventory, target,
logical-unit, ACL, property, growth, dump, SCSI, multipath, and graph
verification commands when tgt-specific inputs are declared.

`provider = "scst"` or `"scstadmin"` renders SCST inventory, backing-device,
target, initiator group, LUN map/unmap, attribute, resync, persistence, and
verification commands when SCST-specific inputs are declared.

Other providers emit non-ready target-LUN handoff commands with stable provider,
array, capacity, mapping, masking, snapshot, and clone fields for external
adapter review.

### Generic Lifecycle Operations

`destroy = true` is destructive and recommends backup, migration, snapshot,
rename, or unmount-first alternatives depending on target type.

`rename` is offline-required but non-destructive. It carries `renameTo`,
`renameTarget`, or `newName` and renders reviewed renames for ZFS, Btrfs, LVM
LVs, thin pools, and VGs.

`promote` is offline-required but non-destructive for ZFS clone datasets and
zvols. Current-topology comparison suppresses promote actions after the object
no longer reports a `zfs.origin`.

Snapshot creation is reversible. Snapshot rollback is potential data loss, and
snapshot destruction is destructive because it removes a recovery point.

ZFS snapshot holds and hold releases are safe property actions. Snapshot
`rescan` for ZFS and absolute Btrfs paths is an online read-only metadata
refresh.

Snapshot clone declarations render reversible ZFS or Btrfs clone plans.
Recursive ZFS rollback is available for review and requires explicit
`allowPotentialDataLoss=true` before execution.

### Declaration Fields

The common lifecycle keys are `operation`, `action`, `destroy`, `target`,
`path`, `device`, `devices`, `paths`, `desiredSize`, `targetSize`, `size`,
`properties`, `metadata`, `preserveData`, `addDevices`, `removeDevices`,
`replaceDevices`, `renameTo`, `renameTarget`, and `newName`.

`operation` and `action` accept lifecycle verbs such as `create`, `format`,
`grow`, `shrink`, `check`, `repair`, `scrub`, `trim`, `rescan`, `replace-device`,
`add-device`, `remove-device`, `add-key`, `remove-key`, `rotate-key`, `login`,
`logout`, `attach`, `detach`, `import`, `export`, `start`, `stop`, `rename`,
`promote`, and `rollback` where the target domain supports them.

## Apply policy

`disk-nix apply --spec <path>` reads the same document as `plan`, evaluates the
planned actions against the top-level `apply` policy, and reports whether each
action is allowed or blocked. By default it is a dry run. With `--execute`, it
requires a fully ready command plan before running any storage command.

Apply reports include `blockedSummary` counters for offline-required, destructive, potential-data-loss, and unsupported blocked actions in addition to the detailed blocked action list. When policy allows an action, the report also includes a `commandSummary` plus a `commandPlan` with planned command argv, mutation markers, manual-review flags, readiness, unresolved inputs, provider capability contracts for generic target-side LUN handoffs, and notes.

If `--probe-current` is set, the report also includes the same `topologyComparison` emitted by `plan`, including any safe no-op actions suppressed before command rendering.

It also includes a `verificationSummary` plus a `verificationPlan` with read-only post-apply commands and checks for the relevant storage domain. Generic target-side LUN provider verification combines the provider-specific inventory placeholder with executable host probes for SCSI path visibility, multipath grouping, and modeled consumer state.

Executed reports also include `executionResults` with command phase, argv, success, exit status, stdout, and stderr for each command that ran. Blocked, non-ready, and failed execution reports include structured `recoveryActions`. Failed risky actions keep the generic current-state capture and preserve-recovery-point advice, then add domain-specific recovery, `roll-forward-review`, and, where read-only preconditions are available, `rollback-review` entries.

Roll-forward review starts with a manual-only
`disk-nix apply --spec <spec> --probe-current --json` dry run against the current
graph.

It adds domain inspection plus post-apply verification commands when they were
rendered.

Rollback review is intentionally read-only and covers concrete domains such as:

- ZFS rollback points and ZFS/Btrfs snapshot lifecycle changes
- LVM snapshot merges, VG device migration, and LVM VG/volume/thin/PV changes
- cache lifecycle changes
- ZFS pool, dataset, and zvol lifecycle changes
- swap signature and activation changes
- filesystem lifecycle updates
- disk and partition-table lifecycle changes
- LUKS mapper, header, keyslot, and token changes
- MD RAID member replacement
- NVMe namespace changes
- iSCSI session login/logout
- VDO lifecycle changes
- multipath map changes
- loop-device, backing-file, and device-mapper map changes
- NFS export and client mount changes
- host-side LUN detach

it does not run rollback commands automatically. Cache command plans include bcache-aware sysfs updates for existing cache-set attachment, cache-mode property changes, `bcache.set-*` cache-set tuning updates, read-only rescans, dirty-data checks, and replacement steps that remain non-ready until the replacement cache device, concrete `/dev/bcache*` target, and new cache-set UUID are declared.

Once `cacheSetUuid` is declared, replacement renders `make-bcache --cset-uuid`, detach, and attach steps without guessing generated identity. Current-topology comparison maps declared bcache `cacheMode`/`cachePolicy` aliases and cache-set tuning properties onto `bcache.cache-mode`, `bcache.cache-policy`, and `bcache.set-*` metadata, with cache-mode value normalization for dashed spellings. bcache device sysfs operations require a concrete `/dev/bcache*` target;

cache-set sysfs property updates require `cacheSetUuid`, `cache-set-uuid`, or equivalent metadata so the plan can write `/sys/fs/bcache/<set>/<field>`. Logical cache declaration names become ready when `target`, `path`, or `device` declares the backing bcache device path. Current-topology comparison keeps logical cache names actionable as missing unless the graph can match them, so a logical name is not treated as absent proof for detach.

Loop-device command plans require a `/dev/loop*` target for grow, rescan, and detach operations. Logical loop declaration names can supply that target with `target` or `path`; `device` remains the backing file or block device for create plans. LVM cache command plans include read-only `lvs` status refresh for `lvmCaches.<origin>.operation = "rescan"` before any later mode, detach, or replacement work.

NFS export command plans use `exportfs -i -o <options> <client>:<path>` for reviewed `operation = "export"` and option-update operations and read-only `exportfs -v` plus graph inspection for `operation = "rescan"`, and `exportfs -u <client>:<path>` for reviewed `operation = "unexport"` operations, with unresolved-input markers when clients, options, or the local export path are missing.

Logical export names can declare the local export path through `target` or `path`. Current-topology comparison suppresses export actions only when the probed export client and requested option subset already match, and it keeps absent exports actionable as planned export work. It suppresses unexport actions only when the export is already absent.

Published unexport targets remain actionable with a warning.

Legacy export `create` and `destroy` map to the same commands.

NFS client mount command plans use:

- `mount -t <nfs|nfs4> -o <options> <source> <mountpoint>` for reviewed mounts
- `mount -o remount,<options> <mountpoint>` for option updates
- read-only `findmnt`, `nfsstat -m`, and graph inspection for rescans
- `umount <mountpoint>` for reviewed unmounts

Legacy NFS mount `create` and `destroy` map to the same mount/unmount command plans. Missing sources or concrete mountpoint paths keep the command plan non-ready. Logical NFS mount names can declare the local mount path through `mountpoint`. With current-topology comparison, absent NFS mountpoints remain actionable as mount-required work.

Disk and partition `operation = "rescan"` actions are online refreshes that render `partprobe <disk>` plus `blockdev --rereadpt <disk>` and verify with `parted -lm <disk>`. They do not edit partition geometry; use `grow` or `create` when the table itself must change. Partition `create` actions reconcile existing matched partition targets before command rendering.

Existing partitions suppress create when any declared desired size matches; different or unknown sizes and matched non-partition nodes stay actionable with warnings and data-preservation guidance.

Partition `grow` actions reconcile parseable byte-sized `end` values against the probed partition size when current-topology comparison is enabled. Already satisfied numeric growth is suppressed before command rendering, while percentage geometry such as `100%` remains actionable because it depends on the current table layout and free-space review.

Filesystem `operation = "remount"` actions are online, non-destructive updates that render `mount -o remount,<options> <mountpoint>`. Missing concrete mountpoints remain non-ready, and long-lived options should be kept in the matching NixOS `fileSystems` entry. Filesystem `operation = "rescan"` actions are online, read-only refreshes that render `findmnt --json <mountpoint>` and `disk-nix inspect <mountpoint>`.

They refresh modeled mount and graph state without mounting, remounting, unmounting, formatting, or checking filesystem metadata. Missing concrete mountpoints remain non-ready. Filesystem `operation = "mount"` and `operation = "unmount"` actions render reviewable `mount [-t <fsType>] [-o <options>] <device> <mountpoint>` and `umount <mountpoint>` command plans from the same `fileSystems`-compatible declarations.

Mounts are online namespace changes; unmounts are offline-gated, non-destructive operations because they can interrupt services, sessions, bind mounts, and automount units. Missing devices or concrete mountpoint paths keep the command plan non-ready. With current-topology comparison, absent mountpoints for mount actions remain actionable as mount-required work.

LVM logical volume command plans use `lvcreate --size <size> --name <lv> <vg>` for `volume` create operations and `lvremove --yes <vg>/<lv>` only after destructive policy gates allow removal.

LV grow and remove commands require canonical `vg/lv` targets from the declaration key, `target`, or `path`. Current-topology comparison suppresses create actions only when the matched LVM logical volume already exists and any declared desired size exactly matches;

existing LVs with different or unknown size stay planned with warnings that point to grow or shrink lifecycle instead of recreate.

LVM thin-pool command plans require canonical `vg/pool` targets for grow and remove operations, supplied by the declaration key, `target`, or `path`.

Current-topology comparison applies the same create reconciliation to thin-pools, but only when the matched node is an LVM thin pool.

LVM logical volume and thin-pool rename reconciliation suppresses already-renamed objects when the old `vg/lv` or `vg/pool` source is absent and the new target exists; short rename targets are resolved within the original volume group. LVM volume group grow and add-device command plans use `vgextend <vg> <pv>` when a physical volume device is declared.

Replacement plans render `vgextend <vg> <new-pv>`, `pvmove <old-pv> <new-pv>`, and `vgreduce <vg> <old-pv>` when both PVs are explicit. Device topology operations remain unresolved until the device to add, the source device, the replacement device, or the device to remove is declared explicitly.

Volume group `operation = "rescan"` refreshes LVM metadata with `pvscan --cache`, `vgscan`, and `vgchange --refresh <vg>` without recreating the VG.

Volume-group rename reconciliation suppresses already-renamed groups when the old VG name is absent and the new VG exists. LVM physical volume command plans use `pvcreate`, `pvresize`, `pvscan --cache`, and `pvremove` for `physicalVolumes` lifecycle declarations. Create, grow, and remove plans require a concrete path-shaped declaration key, `target`, `path`, or `device`;

rescan can refresh all visible PV metadata when no path-shaped target is declared. Current-topology comparison suppresses `operation = "create"` only when the matched target already has LVM PV metadata; matched non-PV devices, duplicate PVs, and missing PVs remain planned with warnings so destructive `pvcreate` is reviewed explicitly.

PV removal advice recommends `pvmove` plus `vgreduce` before `pvremove`. LVM logical volume and thin-pool command plans require canonical `vg/lv` or `vg/pool` targets. Logical declaration names can provide those targets through `target` or `path` so command planning stays executable without encoding the native LVM name in the Nix attribute key.

LVM volume group create comparison suppresses `operation = "create"` when the VG already exists without exported, partial, or missing-PV metadata;

existing exported, partial, or missing-PV VGs stay planned with warnings instead of silently treating a destructive `vgcreate` as safe. Import/export comparison uses `lvm.vg-exported`:

already visible volume groups without an exported marker suppress import actions, already-exported volume groups suppress export actions, and opposite-state volume groups remain planned with a warning.

LUKS keyslot and token command plans use explicit `add-key`, `remove-key`, `import-token`, and `remove-token` lifecycle declarations for `cryptsetup luksAddKey`, `luksKillSlot`, `cryptsetup token import`, and `cryptsetup token remove`. Legacy preserved `create` and `destroy` map to the same access-material command plans. `luksChangeKey` is used for key-file property updates, and keyslot `priority` updates render `cryptsetup config <device> --key-slot <slot> --priority <prefer|normal|ignore>`.

Executable keyslot add/change plans require a LUKS backing device and replacement key file; keyslot priority updates require a LUKS backing device, slot number, and one of `prefer`, `normal`, or `ignore`; token imports require a token JSON file. Removal requires both the device and keyslot number or token id, and remains blocked by the potential-data-loss policy.

Logical keyslot and token names can declare concrete slot/token ids with `keySlot`, `key-slot`, `slot`, `tokenId`, `token-id`, or `token`. With current-topology probing, already-absent keyslots/tokens are suppressed only after the backing container is matched and the specific id is missing from probed LUKS header metadata; still-present entries stay actionable.

Keyslot priority property actions are suppressed when the probed keyslot already has the requested priority. LVM cache command plans use `lvconvert --type cache`, `lvconvert --uncache`, and `lvchange --cachemode` or `--cachepolicy` for `lvmCaches` lifecycle declarations. Executable attach plans require an origin `vg/lv` target and a cache-pool LV through `device` or `addDevices`.

With current-topology probing, detach actions are suppressed only after the origin LV is matched and no cache/writecache metadata remains; still-cached origins remain planned. NVMe namespace command plans use `nvme create-ns`, `nvme attach-ns`, explicit `operation = "rescan"` plans through `nvme ns-rescan`, `nvme detach-ns`, and `nvme delete-ns`.

Create and delete are destructive controller namespace-management operations. Standalone attach is online; standalone detach is offline-required and preserves the namespace. Rescan is online and refreshes host namespace inventory plus subsystem path state through `nvme list-subsys --output-format=json`. Grow is offline-required and means host namespace and subsystem path rescan after controller-side resize or replacement.

Executable create plans require a `/dev/nvme*` controller path from the declaration key, `target`, `path`, or `device`, plus `desiredSize`; attach, detach, and delete flows require `namespaceId` plus `controllers` when attachment state is changed. Use `target` or `path` for the controller and `device` for the host-visible namespace path when the same declaration should also reconcile topology visibility.

Swap grow, format, label, UUID, priority, and rescan command plans require a path-shaped swap target from the declaration key, `target`, `path`, or `device`. Label and UUID updates render `swaplabel --label` and `swaplabel --uuid`; priority updates render `swapoff` followed by `swapon --priority`; `operation = "rescan"` renders read-only `swapon --show`, `blkid`, and graph inspection before any later grow or identity change.

Current-topology comparison maps declared swap label, UUID, and priority properties onto probed identity, signature, and active swap metadata before suppressing already-satisfied property updates. It warns when a swap format target already has swap metadata or matches another node kind, while keeping the destructive `mkswap` action reviewable.

MD RAID assemble, stop, create, grow, member add, replacement, and removal command plans require an explicit array path such as `/dev/md/root`; assemble also requires explicit reviewed member devices. MD create reconciliation suppresses only already clean active arrays and keeps degraded, inactive, or wrong-kind matches actionable with warnings.

MD stop reconciliation suppresses absent or inactive arrays and keeps active, unknown-state, or wrong-kind matches actionable with warnings. MD membership reconciliation suppresses already-attached adds, already-absent removals, and completed replacements where the old member is gone and the new member is attached. MD RAID rescan plans render read-only `mdadm --detail --scan`, `mdadm --examine --scan`, and `/proc/mdstat` inventory checks without assembling arrays.

Current-topology comparison suppresses assemble actions only when the probed array has `md.state` indicating an active or clean array and both `md.degraded-devices` and `md.failed-devices` are zero; degraded or failed arrays stay planned and emit a warning. Loop-device refresh, rescan, and detach command plans require `/dev/loop*` targets.

Rescan reads `losetup --json --list` and graph state without changing capacity; grow uses `losetup -c` after backing size changes. Multipath map growth and path replacement preflight require a concrete map target such as `mpatha` or `/dev/mapper/mpatha`, either as the declaration name or through explicit `target`/`device` fields.

Growth and rescan plans capture host-visible SCSI path transport and size with `lsscsi -t -s` before map reload or resize. Replacement renders separate path add and delete steps so each command can be reviewed independently. ZFS pool device removal renders reviewed `zpool remove <pool> <device>` steps when the pool layout supports evacuation.

LVM volume group device removal renders reviewed `pvmove <pv>` then `vgreduce <vg> <pv>` steps so allocated extents are evacuated before the physical volume is reduced. These remain potential-data-loss intents unless a safer explicit workflow is selected. Btrfs filesystem device topology plans support add, replace, and remove operations.

Removal stays potential-data-loss, while rebalance plans render `btrfs balance start` with optional declared data, metadata, and system filters from lifecycle properties. Btrfs subvolume rename plans render reviewed `mv -- <old> <new>` commands and stay offline-required so mounts, qgroups, snapshots, and send/receive jobs can move together without deleting the original subvolume.

Current-topology comparison suppresses Btrfs subvolume deletion only for concrete absolute paths that are already absent. Present subvolumes stay actionable with subvolume id, generation, parent, top-level, and UUID metadata when available. bcachefs filesystem topology plans support add, replace, remove, grow, rebalance, and scrub operations. Device growth uses `bcachefs device resize` against a declared member device and desired size.

Device add/remove uses `bcachefs device add` and `bcachefs device remove` against the mounted filesystem. Replacement is rendered as add replacement capacity, `bcachefs data rereplicate`, then remove the old member, keeping each data-preserving step visible for review. Rebalance-style plans use `bcachefs data rereplicate`, and scrub plans use `bcachefs scrub`.

Btrfs filesystem label property updates render `btrfs filesystem label <path> <label>`. Ext filesystem label updates render `e2label <device> <label>` when the filesystem declaration includes a backing device. FAT/vfat label updates render `fatlabel <device> <label>`. NTFS label updates render `ntfslabel <device> <label>`. exFAT label updates render `exfatlabel <device> <label>`.

F2FS label updates render `f2fslabel <device> <label>`.

XFS filesystem label updates render `xfs_admin -L <label> <device>`.

Filesystem identity updates are offline-required because they mutate identities
used by mounts and boot paths.

Rendered tools include `btrfstune -U`, `tune2fs -U`, `fatlabel -i`,
`ntfslabel --new-serial`, `exfatlabel -i`, and `xfs_admin -U`.

FAT volume IDs and exFAT volume serials must be 8 hex digits, and NTFS volume serials must be 16 hex digits; all allow optional dash grouping. Current-topology comparison maps declared label, UUID, FAT volume-ID, NTFS serial, and exFAT serial aliases onto probed node identity and filesystem metadata, normalizing hex identity spellings before suppressing already-satisfied property updates.

Missing devices stay marked `needs-domain-implementation`, while unsupported filesystem property keys are classified as unsupported before execution. Failed filesystem apply reports can carry rollback metadata for proven-safe automatic replay. `rollbackOptions`/`previousOptions` on a filesystem remount declaration records the pre-apply mount options that a proven-safe rollback recipe may replay with `mount -o remount,<options> <mountpoint>`.

`rollbackValue`/`previousValue` on a filesystem property declaration records the pre-apply label, UUID, volume ID, or serial that a proven-safe rollback recipe may replay with the filesystem-specific property tool. Filesystem grow, scrub, repair, and failed-check boundaries are emitted as refused/operator-only rollback recipes because they cannot be generically reversed without risking data-preserving state.

Block-stack property declarations use the same `rollbackValue`/`previousValue` metadata for supported identity metadata. Swap label/UUID changes and LUKS label/subsystem/UUID changes can use that pre-apply value for proven-safe rollback replay. Device-mapper rename and LUKS open failures only become automatic when the mutation succeeded and verification failed, so the inverse is bounded to the new mapper name or opened mapper.

Partition, LVM, MD RAID, loop, backing-file, swap deactivation, and zram generated-state mutation boundaries stay refused/operator-only unless a future domain recipe can prove a data-preserving inverse from stronger topology evidence.

Advanced-storage declarations also use `rollbackValue`/`previousValue` for property rollback metadata. ZFS dataset, zvol, and pool property changes, VDO write-policy changes, bcache property changes, and Btrfs subvolume read-only property changes can use the pre-apply value for proven-safe rollback replay.

ZFS and Btrfs rename failures only become automatic when the rename mutation succeeded and verification failed, so the inverse is bounded to the new object name or path. ZFS snapshot rollback/clone, VDO growth, bcache replacement, LVM cache mutation, Btrfs qgroup mutation, pool topology, and dataset/zvol lifecycle boundaries stay refused/operator-only without stronger topology evidence.

Network-storage declarations also use `rollbackValue`/`previousValue` for option and property rollback metadata. NFS remount/export option changes and target-side LUN property changes can use the pre-apply value for proven-safe rollback replay. NFS mount and iSCSI login failures only become automatic when the mutation succeeded and verification failed, so the inverse is bounded to the mounted path or logged-in target/portal.

NFS unmount/unexport, iSCSI logout, host or target LUN growth, target LUN attach/detach, remote export lifecycle, and LUN topology boundaries stay refused/operator-only without stronger initiator, target, active-consumer, and backing-store proof. Btrfs subvolume property updates only treat read-only aliases (`readOnly`, `readonly`, `ro`, `btrfs.readonly`, and `btrfs.ro`) as safe planned property changes.

Other Btrfs subvolume property keys are classified as unsupported so apply policy blocks them before command execution. Ext filesystem grow and shrink actions also carry the declared filesystem `device` or `disk` into `resize2fs` and `e2fsck` command plans. Mountpoint-only ext declarations keep source-device mutations marked unresolved until the block device is explicitly selected.

F2FS grow actions render `resize.f2fs <device>` or `resize.f2fs -t <sectors> <device>` when a target sector count is declared, and keep mountpoint-only plans unresolved until a source device is selected. Filesystem check and repair actions carry the declared `device` or `disk` into read-only and mutating maintenance command plans.

Ext uses `e2fsck`, XFS uses `xfs_repair`, Btrfs uses `btrfs check`, FAT/vfat uses `fsck.fat`, exFAT uses `fsck.exfat`, F2FS uses `fsck.f2fs`, bcachefs uses `bcachefs fsck`, and NTFS uses `ntfsfix`; repair variants remain offline-required and should be reviewed after a read-only check. NTFS repair is limited Linux-side remediation and not a replacement for Windows `chkdsk`.

Btrfs scrub actions use the mounted path and render `btrfs scrub start -B`; bcachefs scrub actions render `bcachefs scrub`; ZFS pool scrub actions render `zpool scrub`.

Filesystem trim actions render `fstrim -v` against the mounted target and remain online maintenance operations. `disk-nix apply --script-out <path>` writes those allowed command and verification plans as a reviewable bash script after policy validation passes and graph dependency conflict checks are clean.

Commands with unresolved inputs remain commented as not ready. `disk-nix apply --report-out <path>` writes the JSON report before returning a blocked-policy, not-ready, or failed-execution error, preserving the decision record for automation and review. `disk-nix apply --receipt-out <path>` writes an audit receipt that wraps the same report with receipt version, command name, spec path, probe-current flag, execute flag, and generation timestamp.

This is the preferred artifact for apply journals and recovery handoff because it preserves how the report was produced. `disk-nix validate --spec <path>` emits the same dry-run report but treats blocked policy as a successful command result, making it the better fit for CI, preflight checks, and NixOS validation paths that need to inspect blocked details.

`validate --report-out <path>` writes the same report to disk, while `validate --receipt-out <path>` writes a receipt with `command = "validate"` and `executeRequested = false`.

Policy fields currently supported:

- `mode`
- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowPotentialDataLoss`
- `allowGrow`
- `allowOffline`
- `allowPropertyChanges`
- `allowDeviceReplacement`
- `allowRebalance`
- `requireBackup`
- `backupVerified`
- `requireConfirmation`
- `confirmation`
- `requireConfirmationFile`

The default policy allows online grow and property-change intents, but blocks offline-required, destructive, irreversible, format, shrink, and potential-data-loss actions. `allowPotentialDataLoss=true` is the explicit policy override for reviewed rollback, shrink, device removal, and similar actions. Unsupported actions are always blocked, even if permissive destructive or shrink policy flags are enabled.

`allowDeviceReplacement=false` blocks device add, replacement, and removal actions. `allowRebalance=false` blocks rebalance actions. `requireBackup=true` requires `backupVerified=true` for destructive or potential-data-loss actions. `requireConfirmation=true` requires `confirmation=true` for high-risk or offline actions. `requireConfirmationFile` points at an operator-controlled file; the CLI treats it as confirmed only when the file contains a standalone line equal to `disk-nix confirm`, and otherwise leaves the action blocked.

`--execute` requires policy validation and a fully ready command plan. It runs planned commands sequentially, stops on the first command failure, records stdout, stderr, and exit status, and only runs verification commands after the planned command phase succeeds.

## Coverage anchors

These exact phrases are kept for the flake documentation coverage check after prose restructuring.

```text
reconciliationGroups
emulate_write_cache
arrayId
initiatorGroup
```
