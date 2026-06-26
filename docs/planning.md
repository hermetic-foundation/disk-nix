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

Current planning is intentionally conservative. It classifies filesystem
resize policy, preservation intent, and lifecycle operations for disks,
partitions, swap, LUKS containers, Btrfs subvolumes, VDO volumes, volumes, LVM
thin pools, LVM snapshots, loop-device mappings, MD RAID arrays, multipath
maps, pools, datasets, zvols, LUNs, iSCSI sessions, exports, cache layers, and
snapshots. It reports
destructive or potentially destructive behavior with alternatives instead of
silently accepting unsafe mutation.

Planned actions are ordered by coarse storage dependency layers after parsing.
Create, attach, open, grow, and other build/update operations run from lower
layers toward upper layers; shrink, remove, unmount, detach, close, and destroy
operations run from upper layers back down. Actions in the same layer keep
their declaration order. Plan JSON includes `dependencyOrder`, a
machine-readable audit trail for that ordering with the action id,
build/mutate/teardown phase, lower-first or upper-first direction, collection
layer rank, inferred `dependsOn` and `unblocks` edges where declared identities
tie adjacent layers together, and explanatory notes. This documents the current
ordering rationale and gives automation explicit preflight edges for common
layered changes. When current topology probing is enabled, matched graph paths
also add dependency edges across direct and multi-hop storage relationships.
Lower-to-upper paths such as LUN to multipath to partition to mapper to volume
to filesystem are emitted in build/grow order, while teardown actions reverse
the path so consumers are handled before backing layers. Mixed-direction
actions on the same graph path are reported as warning diagnostics and counted
as `graphDependencyConflictCount` so operators can split the plan or review
ordering before execution. Apply dry-runs preserve those diagnostics for
review, while `--execute` refuses to mutate storage until the conflict count is
zero. This is still conservative: ambiguous current-state recovery and choosing
between competing graph paths remain hardening work.

Examples:

- `resizePolicy = "grow-only"` is classified as online growth intent.
- `desiredSize`, `targetSize`, or `size` is carried into action context so
  command and verification plans can use concrete capacity targets when the
  storage domain supports them.
- `resizePolicy = "shrink-allowed"` is classified as potential data loss and
  recommends migration or backup-first alternatives. Command plans render
  reviewed Btrfs shrink commands when a target size is declared, and ext
  offline shrink steps with unresolved source-device inputs when only a
  mountpoint is known.
- XFS shrink intent is classified as unsupported because XFS does not support
  shrinking in place; the planner and command renderer recommend creating a
  smaller filesystem and migrating data.
- Filesystem `operation = "check"` and `operation = "repair"` are
  offline-required maintenance workflows. Check plans prefer read-only
  filesystem tools; repair plans mutate metadata and recommend backup or clone
  workflows before touching production storage.
- `preserveData = false` is classified as destructive because it permits
  formatting or replacement. Apply plans render reviewed `mkfs` commands for
  common filesystem types only when a concrete backing `device` or `disk` is
  declared; mountpoint-only declarations remain non-ready. Current-topology
  comparison reports when the matched filesystem already has the requested
  type, but destructive format actions remain in the plan for explicit policy
  and confirmation review.
- `backingFiles` declarations model file-backed storage origins. Create plans
  require a concrete file path plus desired size, refuse to overwrite an
  existing path with `test ! -e`, and render `truncate --size` for the new
  sparse file. Rescan plans are read-only and inspect size, sparse allocation,
  and modeled consumers; grow plans require the same concrete inputs before
  rendering `truncate --size`, leaving loop, swap, and filesystem refresh as
  explicit follow-up actions. Current-topology comparison suppresses create
  only when the existing file already has the declared size, suppresses grow
  when current size already satisfies the desired size, and warns when create
  would hit an existing file with a different or unknown size.
- `dmMaps` declarations model device-mapper refreshes, reviewed mapper renames,
  and explicit mapper removal. Rescan plans inspect map identity, dependencies,
  table, live status, and graph consumers; rename plans are offline-required
  because every dependent LUKS, LVM, VDO, multipath, filesystem, mount, or
  service consumer must move to the new mapper name together. Destroy plans are
  destructive and render `dmsetup remove` only after identity, dependency, and
  status inspection. Current-topology comparison suppresses rename actions when
  the old mapper is absent and the new mapper path exists, keeps missing-source
  rename actions actionable with a warning when the destination is also absent,
  suppresses destroy actions when the mapper is already absent, and keeps
  present maps actionable with a warning that includes `dm.open-count` when
  probe data reports it.
  Prefer LUKS, LVM, VDO, multipath, or cache-specific teardown when another
  domain owns the mapper.
- LUKS keyslot and token add/change operations are offline-required header
  updates. Keyslot or token removal is potential-data-loss because deleting the
  last usable unlock path can make encrypted data inaccessible.
- `removeDevices = [ ... ]` is classified as potential data loss and recommends
  replacement capacity, evacuation, and health verification. Btrfs filesystem
  device removal also verifies allocation state with `btrfs filesystem usage`
  before rendering the reviewed `btrfs device remove <device> <mountpoint>`
  command.
- Btrfs filesystem `operation = "rebalance"` renders `btrfs balance start`.
  Optional `properties.balance.data`, `properties.balance.metadata`, and
  `properties.balance.system` values become `-d`, `-m`, and `-s` balance
  filters so operators can prefer scoped balances over a full balance.
- Btrfs filesystem `operation = "scrub"` renders `btrfs scrub start -B`.
  bcachefs filesystem `operation = "scrub"` renders `bcachefs scrub`.
  ZFS pool `operation = "scrub"` renders `zpool scrub`.
- Filesystem `operation = "trim"` renders `fstrim -v <mountpoint>` and
  recommends validating discard passthrough through lower storage layers.
- `replaceDevices = { old = new; }` is classified as reversible because the
  original device can remain available until verification passes.
- Cache `replace-device` is classified as offline-required because dirty or
  writeback data must be flushed or detached cleanly before replacement.
- Cache `remove-device` is classified as offline-required rather than
  destructive; reviewed plans require dirty-data inspection before bcache
  detach and keep the backing storage intact. Current-topology comparison
  suppresses detach only when a concrete `/dev/bcache*` target is already
  absent; present targets stay actionable with a warning that includes dirty
  data, cache mode, and cache-set UUID when probe metadata reports them.
- Cache `operation = "rescan"` is online and non-destructive; it reads bcache
  state, cache mode, dirty-data, and graph relationships before any later
  attach, detach, or replacement.
- disk partition-table creation is classified as destructive because it can
  hide or replace existing storage metadata. When destructive policy permits
  it, apply plans render reviewed `parted mklabel` and table reread commands.
  Current-topology comparison suppresses disk create actions only when the
  matched physical disk already reports the requested partition table label;
  mismatched labels, unknown labels, and matched non-disk nodes stay actionable
  warnings because `mklabel` can hide existing data.
- partition creation and growth are classified as offline-required because the
  kernel partition table reread and dependent consumers must be coordinated.
  Create and growth plans render concrete table rereads when the backing disk is
  declared.
- swap signature creation is classified as destructive; swap growth is
  offline-required because active swap must be disabled before backing storage
  and signatures are changed. Swapfile growth can render a concrete file resize
  command; block-device swap growth must use the backing storage layer first.
  Swap deactivation is offline-required and renders `swapoff` without removing
  the signature. Swap destruction is destructive because it disables active swap
  and removes signature metadata with `wipefs --all`; prefer deactivation when
  the signature should remain available for later reactivation.
  Swap label and UUID property updates are offline-required because they mutate
  swap signature identity used by mounts, resume paths, and automation. Numeric
  priority updates are also offline-required because applying them reactivates
  the swap target with `swapoff` followed by `swapon --priority`.
  Current-topology comparison reports swap format targets that already have swap
  metadata, including size, usage, priority, or type when probe data has it, and
  also warns when a format target matches another current node kind. It does not
  suppress `mkswap`; destructive policy and confirmation gates still apply.
- zram is modeled as generated compressed swap state rather than persistent
  backing storage. NixOS module declarations derive `zramSwap`, while plain
  zram declarations render read-only `zramctl`, `swapon --show`,
  `disk-nix zram`, and graph inspection commands. Explicit `operation = "rescan"` uses the same inventory refresh path. Zram property declarations are
  offline-required generator-reconciliation requests, not direct live mutation
  commands. Current-topology comparison maps declared algorithm, stream count,
  disk size, memory limit, compression ratio, and priority properties onto
  discovered `/dev/zram*` plus active swap metadata before suppressing
  already-satisfied generated-state updates.
  Algorithm, size, priority, and writeback-device changes should be reviewed as
  generator configuration changes because active `/dev/zram*` devices may need
  swapoff/setup coordination to take effect.
- LUKS `operation = "open"` opens an existing encrypted container as a mapper
  and is offline-required. Legacy `operation = "create"` with preserved data
  remains accepted for the same preserved open flow. LUKS `operation = "close"`
  tears down the mapper without removing the header. LUKS format operations or
  `preserveData = false` are destructive. Current-topology comparison matches
  LUKS format actions by their backing `device` first and reports existing
  LUKS header metadata or non-LUKS matched nodes without suppressing the
  destructive format action. It suppresses `operation = "open"` only when
  `cryptsetup.active` proves the mapper is already active and suppresses
  `operation = "close"` when it proves the mapper is inactive or when the
  mapper is already absent. Absent mapper opens remain actionable with a
  LUKS-specific warning. LUKS growth and mapper close are offline-required
  because backing capacity, mapper state,
  and dependent
  consumers must be coordinated. LUKS header label, subsystem, and UUID
  property updates are offline-required identity metadata changes rendered
  through `cryptsetup config` or `cryptsetup luksUUID`. Current-topology
  comparison matches those property actions by backing device and reconciles
  declared aliases against probed LUKS identity plus `cryptsetup.luks-*`
  header metadata before suppressing already-satisfied updates. Mapper close
  keeps the LUKS header and backing data intact unless a separate format action
  is requested. Logical LUKS declaration keys can declare the concrete mapper
  name with `target`, `mapperName`, `mapper`, or `name`.
- Btrfs subvolume creation is online, while destruction is destructive and
  suggests read-only snapshots or rename-first validation. Btrfs subvolume
  `operation = "rescan"` is online and read-only; it refreshes subvolume
  metadata, read-only state, and modeled graph relationships for the declared
  `path`. Current-topology comparison suppresses concrete absolute-path
  subvolume create actions when the matched node is already a Btrfs subvolume,
  and suppresses destroy actions only when the subvolume is already absent.
  Existing non-subvolume path matches stay actionable for create with warnings;
  present subvolumes remain actionable for destroy with warnings that include
  available subvolume id, generation, parent, top-level, and UUID metadata.
  Logical subvolume names remain actionable unless the graph matches them.
- VDO creation and removal are destructive because they write or remove VDO
  metadata on the backing device; VDO growth is online, with `desiredSize`
  rendering logical growth and explicit `physicalSize` rendering physical
  growth after backing storage has already expanded. Plans advise operators to
  distinguish logical growth from physical backing growth and verify
  `vdostats`. VDO `operation = "start"` and `operation = "stop"` are
  offline-required lifecycle actions that activate or deactivate existing VDO
  metadata without recreating or removing it. VDO `operation = "rescan"` is an
  online, read-only status and utilization refresh. Create preflight inspection is
  marked unresolved until a backing device is declared. Supported VDO property
  updates render reviewed `vdo changeWritePolicy`,
  `vdo enableCompression`/`disableCompression`, and
  `vdo enableDeduplication`/`disableDeduplication` commands. Write policy
  updates require `auto`, `sync`, or `async`; unsupported properties and
  invalid values are classified as unsupported before execution. Logical VDO
  volume names can declare the concrete VDO name with `target`.
  Current-topology comparison warns when VDO create targets already have VDO
  metadata or match another current node kind, without suppressing the
  destructive create. It suppresses VDO grow when current byte size or
  VDO logical-size metadata already satisfies `desiredSize`, and keeps
  below-target, unknown, or absent current targets actionable. VDO destroy
  actions are suppressed only when the volume is already absent; present
  targets remain actionable with warnings that include operating mode, size,
  backing-device, write-policy, or LVM VDO utilization metadata when available. Property
  comparison maps declared VDO write policy, compression, and deduplication
  fields onto native `vdo.write-policy`, `vdo.compression`,
  `vdo.deduplication`, and LVM `lvm.vdo-*` metadata when available. Boolean
  spellings such as `enabled`, `true`, `off`, and `0` are normalized for
  compression and deduplication so no-op property changes are suppressed, while
  mismatched values still emit warning diagnostics.
- LVM logical volume creation is online when it allocates from existing volume
  group free extents; LV growth is also online when the volume group has free
  extents; LV removal is destructive because it deletes the volume contents.
  LV `operation = "rescan"` is online and read-only; it refreshes LV size,
  attributes, activation state, and graph relationships. Create command plans
  report missing `vg/lv` target form and size inputs separately.
- LVM thin-pool creation and growth are online allocations inside an existing
  volume group; thin-pool removal is destructive because it removes contained
  thin volumes and their data. LVM logical volume, thin-pool, snapshot, and VG
  activation/deactivation are offline-required but non-destructive because they
  change availability without creating or removing data. Create command plans
  report missing `vg/pool` target form and size inputs separately.
- LVM volume group creation and removal are destructive because they write or
  remove VG metadata on member physical volumes; prefer `vgextend` when
  preserving an existing group is possible. VG growth with an explicit physical
  volume is an online `vgextend` workflow. VG device removal is
  potential-data-loss unless allocated extents are evacuated before `vgreduce`.
  VG import/export operations are offline-required but non-destructive, and are
  preferred over `vgcreate`/`vgremove` when moving existing disks between hosts.
- ZFS pool creation and destruction are destructive because they write labels
  to vdev devices or remove all contained datasets and zvols; create command
  plans accept either a single `device` or an explicit `devices` vdev list, and
  declared pool `properties` render as create-time `zpool create -o key=value`
  options. Preflight inspection targets path-like vdev entries, while topology
  keywords such as `mirror` stay in the rendered `zpool create` command.
  Current-topology comparison suppresses pool create only when the matched pool
  is already visible with `zfs.state` and `zfs.health` both `ONLINE`; visible
  degraded, faulted, or wrong-kind matches stay actionable with warnings.
  Property comparison maps declared pool keys such as `autotrim`,
  `autoExpand`, `altroot`, and `cachefile` onto probed `zfs.*` and
  pool-scoped `zfs.pool-*` metadata, normalizing common on/off spellings before
  suppressing already-satisfied `zpool set` actions.
  Import/export is preferred when moving an existing pool. `operation = "import"` and `operation = "export"` are offline-required, non-destructive
  pool lifecycle operations; `readOnly = true` renders a reviewed read-only
  import. Current-topology comparison suppresses import actions only when
  `zfs.state` and `zfs.health` are both `ONLINE`; visible degraded or faulted pools stay
  planned with a warning. Pool device replacement is
  offline-required, and device removal remains potential-data-loss unless pool
  topology, free space, and evacuation support have been verified.
- ZFS dataset creation is online, with declared `properties = { ... }`
  rendered as create-time `zfs create -o key=value` options as well as
  explicit property reconciliation actions. Advice still calls out inherited
  mountpoint, quota, reservation, and encryption policy; dataset destruction
  remains destructive and recommends snapshots or rename-first validation.
  Logical declaration names can set `target` or `path` to the concrete
  `pool/name` dataset used by ZFS commands. Current-topology comparison
  suppresses concrete dataset create actions when the matched node is already a
  ZFS dataset, and suppresses destroy actions only when the `pool/name` target
  is already absent. Existing non-dataset matches stay actionable for create
  with warnings; present targets remain actionable for destroy with warnings
  that include available mountpoint, quota, reservation, encryption, key status,
  origin, usage, or compression metadata. Property comparison maps declared
  dataset keys such as `mountpoint`, `compression`, `quota`, `reservation`,
  `atime`, and cache properties onto probed `zfs.*` metadata, normalizing common
  on/off spellings before suppressing already-satisfied `zfs set` actions.
- zvol creation, growth, and property updates are online operations, with
  advice to verify pool capacity, reservation policy, and downstream block
  consumers. zvol `properties = { ... }` render create-time `-o key=value`
  options and `zfs set key=value <zvol>` reconciliation actions. Logical zvol
  names can likewise set `target` or `path` to the concrete `pool/name` zvol.
  Current-topology comparison suppresses concrete zvol create actions when the
  matched node is already a ZFS zvol and any declared desired size is already
  satisfied, and suppresses destroy actions only when the `pool/name` target is
  already absent. Property comparison maps declared zvol keys such as
  `volSize`, `dedup`, `primaryCache`, and `secondaryCache` onto probed
  `zfs.*` metadata and normalizes common on/off spellings before suppressing
  already-satisfied property updates. Existing non-zvol matches or existing
  zvols with different or unknown current size stay actionable for create with
  warnings; present zvols remain actionable for destroy with warnings that
  include available volsize, origin, usage, reservation, encryption, or
  compression metadata.
- MD RAID creation and destruction are destructive because they write array
  metadata or remove array identity. Assemble and stop are offline-required but
  non-destructive: they activate or deactivate existing array metadata while
  preserving member devices. Create command plans identify missing RAID level
  and member-device fields separately. Current-topology comparison suppresses
  MD create only when the matched array is already cleanly active; degraded,
  inactive, or wrong-kind matches stay actionable with warnings. MD stop is
  suppressed only when the array is already absent or inactive; present active,
  unknown-state, or wrong-kind matches stay actionable with warnings.
  Current-topology comparison also suppresses member add when probed
  `MemberOf` edges show the device already attached, and suppresses member
  removal when the device is already absent from the matched array. Member
  replacement is suppressed only after topology shows the old member absent and
  the replacement member attached. Member add is online; replacement and
  grow/reshape are offline-required because redundancy, resync, and dependent
  consumers must be coordinated.
- Multipath map growth and path add are online; path replacement is
  offline-required and path removal is potential-data-loss because at least one
  healthy path must remain active while paths are added and deleted.
- LVM physical volume creation and removal are destructive because they write
  or erase PV metadata. PV growth is online `pvresize` after backing storage
  has already grown.
- NVMe namespace creation and deletion are destructive because they allocate
  or remove controller-managed namespace capacity. Namespace growth is
  offline-required because disk-nix models it as a host rescan after
  controller-side namespace resize or replacement. Namespace attach is an
  online controller visibility change for an existing namespace, while detach
  is offline-required because consumers must be drained before removing access.
- LVM thin pool growth is online, with advice to monitor data and metadata
  utilization, autoextend policy, and thin-volume overcommit. Thin pool
  `operation = "rescan"` is online and read-only; it refreshes data,
  metadata, monitoring, and graph status before later allocation or growth.
- LVM snapshot creation is reversible; snapshot merge rollback is potential
  data loss; snapshot removal is destructive because it deletes a recovery
  point. LVM snapshot `operation = "rescan"` is online and read-only; it
  refreshes origin, COW usage, attributes, size, and graph relationships before
  rollback, activation, or removal decisions.
- Loop-device creation and capacity refresh are online; `operation = "rescan"`
  is online and read-only for mapping inventory refresh. Detach is
  offline-required because mounts, mappers, and other consumers must be stopped
  before the mapping is removed.
- Supported `properties = { ... }` declarations are classified as safe
  property-update intent. Unsupported filesystem and Btrfs subvolume property
  keys are classified as unsupported with alternatives. Btrfs subvolume
  `readonly`, `readOnly`, or `ro` declarations render
  `btrfs property set -ts <path> ro true|false`.
- Btrfs qgroup `properties.limit` or `properties.maxReferenced` render
  `btrfs qgroup limit <size|none> <qgroupid> <path>`;
  `properties.maxExclusive` renders the exclusive limit form with `-e`.
  Current-topology comparison reconciles those aliases against probed
  `btrfs.max-referenced` and `btrfs.max-exclusive` metadata, normalizing
  `none`, `null`, and `unlimited` spellings before suppressing
  already-satisfied limit updates.
  `operation = "create"` and `destroy = true` render Btrfs qgroup create and
  destroy commands when a filesystem path is declared through `target`, `path`,
  or `mountpoint`.
  `operation = "rescan"` is online and read-only; it refreshes qgroup hierarchy,
  referenced/exclusive usage, limits, and graph relationships. Qgroup create,
  destroy, limit, and rescan plans remain non-ready until that mounted
  filesystem path is declared through `target`, `path`, or `mountpoint`.
- Cache attach and cache-mode updates are online or safe when they use an
  existing cache-set identity; cache replacement remains offline-required and
  cache removal is potential-data-loss because dirty writeback data must be
  flushed or detached before media changes.
- LVM cache attach, detach, and replacement are offline-required because
  `lvconvert` changes origin LV I/O paths and dirty cache state must be drained.
  LVM cache mode and policy updates are safe but still include verification
  advice. LVM cache `operation = "rescan"` is online and read-only; it refreshes
  cache mode, policy, utilization, and modeled relationships. Current-topology
  comparison maps declared `cacheMode` and `cachePolicy` aliases onto
  `lvm.cache-mode` and `lvm.cache-policy` metadata, normalizing dashed
  cache-mode spellings such as `write-through` before suppressing no-op
  property updates.
- NFS export publication with `operation = "export"` is online when it
  publishes an existing path to explicit clients and options; unexporting is
  offline-required because remote clients may need to be drained, but it is not
  treated as data destruction. Legacy NFS export `create` and `destroy` still
  map to the same lifecycle paths.
- iSCSI session `operation = "rescan"` is online host refresh work: it asks
  existing sessions to rescan target-side changes, then captures host-visible
  LUN transport and size through `lsscsi -t -s` before graph inspection.
- LUN `operation = "attach"` means host-side attach for an existing target-side
  LUN and is online when it rescans sessions, captures host-visible transport
  and size through `lsscsi -t -s`, rescans declared stable paths, and verifies
  path capacity.
  Legacy LUN `create` maps to the same host attach lifecycle.
  LUN `operation = "rescan"` is online and refreshes existing host-visible
  paths without implying target-side capacity growth.
  LUN `operation = "grow"` is offline-required because the storage target,
  host rescan, multipath, and consumers must be coordinated. LUN
  `operation = "detach"` is modeled as host-side path detach, not target-side
  array deletion, and remains offline-required. Legacy LUN `destroy` maps to
  the same detach lifecycle. When stable paths are declared through `device`,
  `path`, `devices`, `paths`, or `devicePaths`, apply plans render per-path SCSI
  rescans or deletes in addition to broad iSCSI session, `lsscsi`, and
  multipath refreshes.
  Current-topology comparison suppresses attach actions only when the declared
  LUN path is already visible and suppresses detach actions only when the
  declared LUN path is already absent; opposite states stay actionable with a
  warning.
  Executable attach, grow, and detach plans remain non-ready until those stable
  LUN paths are declared.
- iSCSI session `operation = "login"` discovers/logs into an existing target
  and is online. Legacy `operation = "create"` remains accepted for the same
  login flow. `operation = "logout"` detaches remote LUN paths from the host,
  is offline-required, and preserves target-side data. Legacy `destroy = true`
  remains accepted for the same logout flow. Session `operation = "rescan"` is
  online and refreshes existing target paths. Session `operation = "grow"` is
  offline-required because target growth, session/path rescan, and dependent
  consumers must be coordinated.
- `destroy = true` is classified as destructive and recommends backup,
  migration, snapshot, rename, or unmount-first alternatives depending on the
  target type.
- `operation = "rename"` is offline-required but non-destructive. It carries
  `renameTo`, `renameTarget`, or `newName` as the new reference and renders
  reviewed rename commands for ZFS datasets/zvols/snapshots, Btrfs subvolume
  paths, LVM logical volumes/thin pools, and LVM volume groups. For ZFS
  datasets and zvols, current-topology comparison suppresses rename actions
  when the source no longer exists and the destination already exists with the
  expected ZFS object kind. LVM logical volume, thin-pool, and volume-group
  rename reconciliation applies the same old-identity/new-identity check.
- `operation = "promote"` is offline-required but non-destructive for ZFS
  clone datasets and zvols. It renders reviewed `zfs promote <clone>` commands
  after inspecting the clone origin. Current-topology comparison suppresses
  promote actions when the matched dataset or zvol no longer reports a
  `zfs.origin`; objects that still report an origin stay actionable with a
  warning.
- snapshot creation is reversible; snapshot rollback is potential data loss;
  snapshot destruction is destructive because it removes a recovery point.
  Generic snapshot names such as `pool/dataset@snap` map to ZFS snapshots;
  absolute source and snapshot paths map to Btrfs subvolume snapshots. Btrfs
  snapshot declarations can set `readOnly = true` to render
  `btrfs subvolume snapshot -r`. Snapshot destruction remains destructive, and
  unambiguous ZFS snapshot names or Btrfs absolute snapshot paths render
  reviewed `zfs destroy` or `btrfs subvolume delete` commands. Current-topology
  comparison suppresses already-absent concrete ZFS snapshots and absolute
  Btrfs snapshot paths; present snapshots stay actionable with warnings that
  include available ZFS user-reference/usage metadata or Btrfs subvolume
  metadata. ZFS snapshot `hold` and `releaseHold` declarations are safe
  property actions that render `zfs hold <tag> <snapshot>` and
  `zfs release <tag> <snapshot>`. Current-topology comparison suppresses hold
  declarations when `zfs holds -H` metadata already reports the tag and
  suppresses release declarations when the tag is already absent. ZFS snapshot
  `operation = "rescan"` and absolute Btrfs snapshot rescan declarations are
  online read-only refreshes for snapshot metadata, holds, read-only state, and
  graph relationships. Snapshot declarations can use `name`, `snapshotName`, or
  `snapshot-name` to provide the concrete snapshot identity when the map key is
  a friendly name. Btrfs snapshot rescans can also use `path`, `snapshotPath`,
  or `snapshot-path` to provide the concrete snapshot path. Snapshot clone
  declarations with `cloneTo`, `cloneTarget`, or `clone` render reversible
  `zfs clone <snapshot> <dataset>` plans for ZFS snapshots and
  `btrfs subvolume snapshot <snapshot-path> <clone-path>` plans for absolute
  Btrfs snapshot paths. Btrfs clone declarations with `readOnly = true` render
  read-only `btrfs subvolume snapshot -r` plans. Current-topology comparison
  checks concrete clone sources, warns when a source snapshot is missing, and
  reports available clone sources with snapshot metadata. Friendly Btrfs clone
  declarations can use `snapshotPath` or `snapshot-path` to provide the
  concrete source path. ZFS rollback command rendering is available for review, and `recursiveRollback`, `recursive`, or
  `zfs.rollbackRecursive` render explicit `zfs rollback -r` details for
  recursive rollback review. Apply blocks rollback by default and requires
  explicit `allowPotentialDataLoss=true` policy before execution.
  Current-topology comparison checks the concrete ZFS rollback snapshot itself,
  warns when the rollback point is missing, and keeps available rollback points
  actionable with snapshot metadata because rollback remains potential data
  loss.
  Snapshot rollback remains non-ready when a friendly declaration key does not
  resolve to a concrete ZFS snapshot name. Snapshot clone remains non-ready
  when a friendly declaration key does not resolve to a concrete ZFS snapshot
  name or absolute Btrfs snapshot path.
  Snapshot rename remains non-ready when a friendly declaration key does not
  resolve to a concrete ZFS snapshot name or absolute Btrfs snapshot path.
  Current-topology comparison checks the source snapshot being renamed, warns
  when that concrete source is missing, and keeps present rename sources
  actionable with snapshot metadata because rename remains offline-required.

The checked-in specs under `examples/` are part of `nix flake check`. The
flake validates stable plan summaries, selected action ids, allowed simple
apply output, blocked lifecycle apply output, and review-script generation.
`disk-nix schema` emits a JSON Schema-style contract for direct specs, NixOS
module wrapper specs, lifecycle collections, snapshot declarations, and apply
policy fields. The current supported contract is version `1`. Specs may omit
`version`, but if `version` or `spec.version` is present it must be integer
`1`; unsupported future versions are rejected before planning.

Lifecycle collections currently accepted by the planner:

- `disks`
- `partitions`
- `swaps`
- `luks.devices`
- `luksKeyslots`
- `luksTokens`
- `btrfsSubvolumes`
- `btrfsQgroups`
- `vdoVolumes`
- `physicalVolumes`
- `volumes`
- `volumeGroups`
- `thinPools`
- `lvmSnapshots`
- `lvmCaches`
- `loopDevices`
- `mdRaids`
- `multipathMaps`
- `pools`
- `datasets`
- `zvols`
- `luns`
- `nvmeNamespaces`
- `iscsiSessions`
- `exports`
- `caches`
- `snapshots`

Multipath map lifecycle treats path membership and whole-map removal
separately. Path removal is potential-data-loss because it can break
redundancy, while `multipathMaps.<name>.operation = "destroy"` is
offline-required host map flushing through `multipath -f`; target-side LUN data
is not deleted, but filesystems, LVM, dm, and services must move away first.
Current-topology comparison suppresses destroy actions when the map is already
absent and keeps present maps actionable with a warning that includes the WWID
or dm map name when probe metadata reports it. It also suppresses path add when
probed `Backs` edges show the path already feeds the map, and suppresses path
removal when the path is already absent from the matched map. Path add requests
against an absent map remain actionable with a warning, because the target map
must be reviewed or recreated before path membership can be confirmed.

ZFS dataset and zvol `operation = "rescan"` actions are online read-only
refreshes. Dataset rescan renders `zfs list -t filesystem`, `zfs get`, and
graph inspection for mountpoint, quota, reservation, snapshot, clone, mount,
and export relationships. Zvol rescan renders the equivalent
`zfs list -t volume`, `zfs get`, and graph inspection for volsize,
reservation, and block consumers. Logical declaration keys can use `target` or
`path` for the concrete dataset or zvol name. Use property updates or grow only
when state must actually change. Current-topology comparison suppresses
concrete dataset create actions when the matched node is already a ZFS dataset,
and concrete zvol create actions when the matched node is already a ZFS zvol
and any declared desired size is already satisfied. Existing wrong-kind or
wrong-size create targets stay actionable with warnings. Dataset and zvol
destroy actions are suppressed only when the declared `pool/name` target is
already absent; present targets stay actionable with ZFS metadata warnings.

Btrfs qgroup `operation = "rescan"` actions are online read-only refreshes for
quota hierarchy, referenced/exclusive usage, and limits. Qgroup create,
destroy, limit, and rescan declarations use the qgroup id as the graph identity
and the mounted filesystem path as executor context. Current-topology
comparison suppresses concrete numeric qgroup create actions when the matched
node is already a Btrfs qgroup, and suppresses destroy actions such as `0/257`
only when that qgroup is already absent. Existing non-qgroup matches stay
actionable for create with warnings; present qgroups stay actionable for destroy
with warnings that include usage, limit, parent, or child metadata when
available. Logical qgroup names remain actionable unless a graph node actually
matches them.

Lifecycle objects may use:

- `operation` or `action`: `create`, `format`, `grow`, `shrink`, `check`,
  `repair`, `scrub`, `trim`, `rescan`, `replace-device`, `add-device`,
  `remove-device`, `add-key`, `remove-key`, `import-token`, `remove-token`,
  `set-property`, `snapshot`, `promote`, `import`, `export`, `unexport`,
  `attach`, `detach`, `activate`, `deactivate`, `assemble`, `start`, `stop`,
  `login`, `logout`, `open`, `close`, `mount`, `unmount`, `remount`, `rename`,
  `rebalance`, `rollback`, or `destroy`
- `addDevices`: list of devices to attach
- `devices`, `paths`, or `devicePaths`: member devices for arrays and pools, or
  explicit LUN paths that should receive per-path host rescans
- `removeDevices`: list of devices to remove
- `renameTo`, `renameTarget`, or `newName`: new name or path for rename
  lifecycle operations
- `replaceDevices`: object mapping old device to replacement device
- `properties`: object of properties to set
- `desiredSize`, `targetSize`, or `size`: desired capacity for grow, shrink,
  or create plans
- `physicalSize`: explicit VDO physical backing-size intent for
  `vdo growPhysical` planning
- `target`, `path`, or `mountpoint`: explicit target path or object identity
  when it differs from the attribute name
- `name`, `snapshotName`, or `snapshot-name`: explicit snapshot identity when a
  snapshot declaration uses a friendly attribute name
- `device` or `disk`: backing device path for disk, partition, and LUN operations
- `level` or `raidLevel`: MD RAID level for reviewed array creation
- `client`: NFS export client or network selector
- `portal`: iSCSI target portal such as `192.0.2.10:3260`; `metadata.portal`
  is also accepted for NixOS-module-derived session declarations
- `options`: NFS export options used for reviewed `exportfs` export commands
- `start` or `startOffset`, and `end` or `endOffset`: partition geometry for
  partition creation or resizing
- `partitionNumber` or `number`: partition number for concrete partition
  resize commands
- `partitionType` or `type`: partition type/name metadata for partition
  lifecycle plans
- `destroy`: boolean destructive intent
- `preserveData`: boolean preservation policy

Plan actions include typed `context` when a desired object provides useful
executor inputs. Context fields can include collection, name, target, device,
replacement, property, property value, filesystem type, mountpoint, and
desired or physical size, plus partition start, end, and type. Apply reports
use this context to build command plans without relying on action-id parsing.

`disk-nix plan --probe-current --spec <path>` probes the current host and adds
a `topologyComparison` section to the plan. The comparison matches action
targets against the storage graph and reports missing targets, current size
state versus `desiredSize`, filesystem type conflicts, matching filesystem
format types that still require destructive review, and already-satisfied
mount, remount, NFS export, iSCSI login, Btrfs subvolume or qgroup destroy,
generic snapshot destroy, or property updates where the current graph has
enough data. Remount
reconciliation treats declared options as a required subset of the current
mount options, allowing kernel-added defaults to remain. Filesystem and NFS
unmount reconciliation treats an absent mountpoint as already satisfied and
keeps currently mounted targets actionable with a warning.
LVM activation/deactivation reconciliation uses `lvm.active` topology metadata
to suppress already-active `volumes`, `thinPools`, and `lvmSnapshots`
activation actions and already-inactive deactivation actions, and to warn when
a matched LVM object is in the opposite state. Absent activation targets remain
actionable with an LVM warning, while absent deactivation targets are treated
as already inactive.
LUKS open/close reconciliation uses `cryptsetup.active` topology metadata to
suppress mapper opens that are already active and mapper closes that are
already inactive or absent; opposite-state mappers and absent mapper opens
remain actionable with a warning.
LUKS keyslot and token removal reconciliation matches the declared backing
device to the LUKS container and uses `cryptsetup.luks-keyslots` or
`cryptsetup.luks-tokens` from `luksDump` metadata to suppress only removals
whose slot or token id is already absent. Present slots and tokens remain
actionable with warnings that include keyslot priority, cipher, PBKDF, token
type, or token keyslot binding metadata when available. Missing LUKS containers
for keyslot or token removals remain actionable with header review warnings.
LVM cache detach reconciliation matches the origin LV and suppresses detach
only when the current LV no longer reports cache or writecache metadata.
Still-attached cache origins remain actionable with warnings that include cache
pool, mode, policy, dirty blocks, and utilization when available. Absent cache
origins remain actionable with LVM metadata review warnings instead of generic
missing-target diagnostics.
LVM logical-volume, thin-pool, and volume-group rename reconciliation compares
source and destination identities. Source-present renames remain actionable with
current LVM metadata, destination-present renames are suppressed as already
reflected in topology, and source-absent/destination-absent renames stay
actionable with LVM review warnings.
Loop-device create/destroy reconciliation uses `loop.back-file` topology
metadata to suppress create actions only when the loop device already maps the
declared backing file and suppress destroy/detach actions only when the loop
device is already absent; occupied loop devices with different backing files
stay actionable with a warning.
NVMe namespace attach/detach reconciliation suppresses attach actions only
when the declared namespace path is already visible and suppresses detach
actions only when the declared namespace path is already absent; opposite
states stay actionable with a warning.
VDO create reconciliation warns when the target already has VDO metadata or
matches another current node kind. VDO grow reconciliation suppresses growth
when current byte size or VDO logical-size metadata already satisfies
`desiredSize`; below-target, unknown, or absent current targets stay
actionable with VDO grow diagnostics. VDO destroy reconciliation suppresses
removal only when the VDO volume is already absent. Present VDO removal
targets stay actionable with warnings that include available operating-mode,
size, backing-device, write-policy, and LVM VDO utilization metadata. VDO start/stop
reconciliation uses
`vdo.operating-mode` topology metadata to suppress start actions only when the
volume is already in `normal` mode and stop actions only when the mode
explicitly reports stopped, not-running, or inactive, or when the volume is
already absent; absent starts and opposite states stay actionable with a warning.
VDO property reconciliation compares declared
`writePolicy`, `compression`, and `deduplication` settings with native
`vdo.*` metadata and LVM `lvm.vdo-*` metadata, including boolean normalization
for compression and deduplication aliases, before deciding whether to suppress
an already-satisfied property update.
Btrfs subvolume destroy reconciliation suppresses concrete absolute-path
targets only when they are already absent. Present subvolumes stay actionable
with warnings that include available subvolume id, generation, parent,
top-level, and UUID metadata.
ZFS dataset and zvol destroy reconciliation suppresses concrete `pool/name`
targets only when they are already absent. Present targets stay actionable with
warnings that include available mountpoint, quota, reservation, encryption, key
status, origin, usage, volsize, or compression metadata.
ZFS dataset and zvol promote reconciliation compares probed `zfs.origin`
metadata. Clones with an origin remain actionable for reviewed promotion, while
already-promoted objects without origin metadata are suppressed as satisfied.
ZFS dataset and zvol rename reconciliation compares the old and new
`pool/name` identities. Source-present renames remain actionable with current
metadata, while source-absent and destination-present renames are suppressed as
already reflected in topology. Source-absent and destination-absent renames
stay actionable with ZFS metadata review warnings.
Generic snapshot destroy reconciliation suppresses concrete ZFS snapshot names
and absolute Btrfs snapshot paths only when they are already absent. Present
snapshots stay actionable with warnings that include available ZFS
user-reference, hold tag, usage, compression, or encryption metadata, or Btrfs
subvolume id, generation, parent, top-level, and UUID metadata.
ZFS snapshot hold and release reconciliation compares requested tags against
probed `zfs.holds` and `zfs.hold.<tag>` metadata so already-present holds and
already-absent releases are treated as satisfied instead of reissued.
Generic snapshot clone reconciliation compares the concrete source ZFS
snapshot name or absolute Btrfs snapshot path before considering the clone
destination. Missing clone sources stay actionable with warning diagnostics;
available clone sources are reported with snapshot metadata and the requested
destination.
Generic snapshot rename reconciliation compares the concrete source ZFS
snapshot name or absolute Btrfs snapshot path, not just the source dataset or
subvolume. Missing and present rename sources both stay actionable with warning
diagnostics; present sources include snapshot metadata and the requested
destination.
ZFS snapshot rollback reconciliation compares the concrete rollback snapshot,
not just the target dataset. Missing rollback points and available rollback
points both stay actionable with warning diagnostics; available points include
snapshot metadata so reviewers can inspect usage, holds, and recursive rollback
intent before policy-gated execution.
NFS export reconciliation compares the declared client and options against
`nfs.export-client` and `nfs.export-option-*` topology properties. Absent
exports remain actionable as domain-specific export work rather than generic
missing storage targets; matching exports are suppressed, and differing
client/options stay actionable for review.
iSCSI session reconciliation checks all matching target and session nodes so an
active session is not hidden by a configured but disconnected target. Login
actions are suppressed only when a logged-in session is present; logout actions
are suppressed only when the target is known and no logged-in session is
present.
Already-satisfied grow, shrink, device-mapper destroy, multipath destroy,
bcache detach, iSCSI login/logout, LVM activation/deactivation, LUKS open, loop
create/destroy, LUN attach/detach, NVMe namespace attach/detach, mount,
unmount, remount, NFS export/unexport, VDO destroy, VDO start, VDO stop, MD
assemble, Btrfs subvolume destroy, ZFS dataset/zvol destroy, ZFS pool import,
LVM volume-group import/export, and set-property actions with no warning diagnostics are
suppressed from the actionable plan and counted in
`topologyComparison.summary.suppressedActionCount`.
Absent mountpoints for mount actions remain actionable with mount-required
diagnostics rather than generic missing-target diagnostics.

## Apply policy

`disk-nix apply --spec <path>` reads the same document as `plan`, evaluates the
planned actions against the top-level `apply` policy, and reports whether each
action is allowed or blocked. By default it is a dry run. With `--execute`, it
requires a fully ready command plan before running any storage command.

Apply reports include `blockedSummary` counters for offline-required,
destructive, potential-data-loss, and unsupported blocked actions in addition
to the detailed blocked action list. When policy allows an action, the report
also includes a `commandSummary` plus a `commandPlan` with planned command
argv, mutation markers, manual-review flags, readiness, unresolved inputs, and
notes. If `--probe-current` is set, the report also includes the same
`topologyComparison` emitted by `plan`, including any safe no-op actions
suppressed before command rendering. It also includes a
`verificationSummary` plus a `verificationPlan` with read-only post-apply
commands and checks for the relevant storage domain. Executed reports also
include `executionResults` with command phase, argv, success, exit status,
stdout, and stderr for each command that ran.
Blocked, non-ready, and failed execution reports include structured
`recoveryActions`. Failed risky actions keep the generic current-state capture
and preserve-recovery-point advice, then add domain-specific recovery,
`roll-forward-review`, and, where read-only preconditions are available,
`rollback-review` entries. Roll-forward review starts with a manual-only
`disk-nix apply --spec <spec> --probe-current --json` dry run against the
current graph and adds domain inspection plus post-apply verification commands
when they were rendered. Rollback review is intentionally read-only and covers
concrete domains such as ZFS rollback points, LVM snapshot merges, VG device
migration, cache detach, and host-side LUN detach; it does not run rollback
commands automatically.
Cache command plans include bcache-aware sysfs updates for existing cache-set
attachment, cache-mode property changes, `bcache.set-*` cache-set tuning
updates, read-only rescans, dirty-data checks, and replacement steps that remain
non-ready until the replacement cache device, concrete `/dev/bcache*` target,
and new cache-set UUID are declared. Once `cacheSetUuid` is declared,
replacement renders `make-bcache --cset-uuid`, detach, and attach steps without
guessing generated identity. Current-topology comparison maps declared bcache
`cacheMode`/`cachePolicy` aliases and cache-set tuning properties onto
`bcache.cache-mode`, `bcache.cache-policy`, and `bcache.set-*` metadata, with
cache-mode value normalization for dashed spellings. bcache device sysfs
operations require a concrete `/dev/bcache*` target; cache-set sysfs property
updates require `cacheSetUuid`, `cache-set-uuid`, or equivalent metadata so the
plan can write `/sys/fs/bcache/<set>/<field>`. Logical cache declaration names
become ready when `target`, `path`, or `device` declares the backing bcache
device path. Current-topology comparison keeps logical cache names actionable as
missing unless the graph can match them, so a logical name is not treated as
absent proof for detach.
Loop-device command plans require a `/dev/loop*` target for grow, rescan, and
detach operations. Logical loop declaration names can supply that target with
`target` or `path`; `device` remains the backing file or block device for
create plans.
LVM cache command plans include read-only `lvs` status refresh for
`lvmCaches.<origin>.operation = "rescan"` before any later mode, detach, or
replacement work.
NFS export command plans use `exportfs -i -o <options> <client>:<path>` for
reviewed `operation = "export"` and option-update operations and
read-only `exportfs -v` plus graph inspection for `operation = "rescan"`, and
`exportfs -u <client>:<path>` for reviewed `operation = "unexport"` operations,
with unresolved-input markers when clients, options, or the local export path
are missing. Logical export names can declare the local export path through
`target` or `path`. Current-topology comparison suppresses export actions only
when the probed export client and requested option subset already match, and it
keeps absent exports actionable as planned export work. It suppresses unexport
actions only when the export is already absent. Published unexport targets
remain actionable with a warning. Legacy export `create` and `destroy` map to
the same commands.
NFS client mount command plans use
`mount -t <nfs|nfs4> -o <options> <source> <mountpoint>` for reviewed
`operation = "mount"` actions, `mount -o remount,<options> <mountpoint>` for
reviewed option updates, read-only `findmnt`, `nfsstat -m`, and graph
inspection for `operation = "rescan"`, and `umount <mountpoint>` for reviewed
`operation = "unmount"` actions. Legacy NFS mount `create` and `destroy` map to
the same mount/unmount command plans. Missing sources or concrete mountpoint
paths keep the command plan non-ready. Logical NFS mount names can declare the
local mount path through `mountpoint`. With current-topology comparison, absent
NFS mountpoints remain actionable as mount-required work.
Disk and partition `operation = "rescan"` actions are online refreshes that
render `partprobe <disk>` plus `blockdev --rereadpt <disk>` and verify with
`parted -lm <disk>`. They do not edit partition geometry; use `grow` or
`create` when the table itself must change.
Partition `create` actions reconcile existing matched partition targets before
command rendering. Existing partitions suppress create when any declared
desired size matches; different or unknown sizes and matched non-partition
nodes stay actionable with warnings and data-preservation guidance.
Partition `grow` actions reconcile parseable byte-sized `end` values against
the probed partition size when current-topology comparison is enabled. Already
satisfied numeric growth is suppressed before command rendering, while
percentage geometry such as `100%` remains actionable because it depends on the
current table layout and free-space review.
Filesystem `operation = "remount"` actions are online, non-destructive updates
that render `mount -o remount,<options> <mountpoint>`. Missing concrete
mountpoints remain non-ready, and long-lived options should be kept in the
matching NixOS `fileSystems` entry.
Filesystem `operation = "rescan"` actions are online, read-only refreshes that
render `findmnt --json <mountpoint>` and `disk-nix inspect <mountpoint>`.
They refresh modeled mount and graph state without mounting, remounting,
unmounting, formatting, or checking filesystem metadata. Missing concrete
mountpoints remain non-ready.
Filesystem `operation = "mount"` and `operation = "unmount"` actions render
reviewable `mount [-t <fsType>] [-o <options>] <device> <mountpoint>` and
`umount <mountpoint>` command plans from the same `fileSystems`-compatible
declarations. Mounts are online namespace changes; unmounts are offline-gated,
non-destructive operations because they can interrupt services, sessions, bind
mounts, and automount units. Missing devices or concrete mountpoint paths keep
the command plan non-ready. With current-topology comparison, absent
mountpoints for mount actions remain actionable as mount-required work.
LVM logical volume command plans use `lvcreate --size <size> --name <lv> <vg>`
for `volume` create operations and `lvremove --yes <vg>/<lv>` only after
destructive policy gates allow removal. LV grow and remove commands require
canonical `vg/lv` targets from the declaration key, `target`, or `path`.
Current-topology comparison suppresses create actions only when the matched LVM
logical volume already exists and any declared desired size exactly matches;
existing LVs with different or unknown size stay planned with warnings that
point to grow or shrink lifecycle instead of recreate.
LVM thin-pool command plans require canonical `vg/pool` targets for grow and
remove operations, supplied by the declaration key, `target`, or `path`.
Current-topology comparison applies the same create reconciliation to
thin-pools, but only when the matched node is an LVM thin pool. LVM logical
volume and thin-pool rename reconciliation suppresses already-renamed objects
when the old `vg/lv` or `vg/pool` source is absent and the new target exists;
short rename targets are resolved within the original volume group.
LVM volume group grow and add-device command plans use `vgextend <vg> <pv>`
when a physical volume device is declared. Replacement plans render
`vgextend <vg> <new-pv>`, `pvmove <old-pv> <new-pv>`, and
`vgreduce <vg> <old-pv>` when both PVs are explicit. Device topology operations
remain unresolved until the device to add, the source device, the replacement
device, or the device to remove is declared explicitly.
Volume group `operation = "rescan"` refreshes LVM metadata with
`pvscan --cache`, `vgscan`, and `vgchange --refresh <vg>` without recreating
the VG. Volume-group rename reconciliation suppresses already-renamed groups
when the old VG name is absent and the new VG exists.
LVM physical volume command plans use `pvcreate`, `pvresize`,
`pvscan --cache`, and `pvremove` for `physicalVolumes` lifecycle declarations.
Create, grow, and remove plans require a concrete path-shaped declaration key,
`target`, `path`, or `device`; rescan can refresh all visible PV metadata when
no path-shaped target is declared. Current-topology comparison suppresses
`operation = "create"` only when the matched target already has LVM PV metadata;
matched non-PV devices, duplicate PVs, and missing PVs remain planned with
warnings so destructive `pvcreate` is reviewed explicitly. PV removal advice
recommends `pvmove` plus `vgreduce` before `pvremove`.
LVM logical volume and thin-pool command plans require canonical `vg/lv` or
`vg/pool` targets. Logical declaration names can provide those targets through
`target` or `path` so command planning stays executable without encoding the
native LVM name in the Nix attribute key.
LVM volume group create comparison suppresses `operation = "create"` when the
VG already exists without exported, partial, or missing-PV metadata; existing
exported, partial, or missing-PV VGs stay planned with warnings instead of
silently treating a destructive `vgcreate` as safe. Import/export comparison
uses `lvm.vg-exported`: already visible volume groups without an exported
marker suppress import actions, already-exported volume groups suppress export
actions, and opposite-state volume groups remain planned with a warning.
LUKS keyslot and token command plans use explicit `add-key`, `remove-key`,
`import-token`, and `remove-token` lifecycle declarations for
`cryptsetup luksAddKey`, `luksKillSlot`, `cryptsetup token import`, and
`cryptsetup token remove`. Legacy preserved `create` and `destroy` map to the
same access-material command plans. `luksChangeKey` is used for key-file
property updates, and keyslot `priority` updates render
`cryptsetup config <device> --key-slot <slot> --priority <prefer|normal|ignore>`.
Executable keyslot add/change plans require a LUKS backing device and
replacement key file; keyslot priority updates require a LUKS backing device,
slot number, and one of `prefer`, `normal`, or `ignore`; token imports require
a token JSON file. Removal requires both the device and keyslot number or token
id, and remains blocked by the potential-data-loss policy. Logical keyslot and
token names can declare concrete slot/token ids with `keySlot`, `key-slot`,
`slot`, `tokenId`, `token-id`, or `token`. With current-topology probing,
already-absent keyslots/tokens are suppressed only after the backing container
is matched and the specific id is missing from probed LUKS header metadata;
still-present entries stay actionable. Keyslot priority property actions are
suppressed when the probed keyslot already has the requested priority.
LVM cache command plans use `lvconvert --type cache`, `lvconvert --uncache`,
and `lvchange --cachemode` or `--cachepolicy` for `lvmCaches` lifecycle
declarations. Executable attach plans require an origin `vg/lv` target and a
cache-pool LV through `device` or `addDevices`. With current-topology probing,
detach actions are suppressed only after the origin LV is matched and no
cache/writecache metadata remains; still-cached origins remain planned.
NVMe namespace command plans use `nvme create-ns`, `nvme attach-ns`,
explicit `operation = "rescan"` plans through `nvme ns-rescan`,
`nvme detach-ns`, and `nvme delete-ns`. Create and delete are destructive
controller namespace-management operations. Standalone attach is online;
standalone detach is offline-required and preserves the namespace. Rescan is
online and refreshes host namespace inventory plus subsystem path state through
`nvme list-subsys --output-format=json`. Grow is offline-required and means
host namespace and subsystem path rescan after controller-side resize or
replacement.
Executable create plans require a `/dev/nvme*` controller path from the
declaration key, `target`, `path`, or `device`, plus `desiredSize`; attach,
detach, and delete flows require `namespaceId` plus `controllers` when
attachment state is changed. Use `target` or `path` for the controller and
`device` for the host-visible namespace path when the same declaration should
also reconcile topology visibility.
Swap grow, format, label, UUID, priority, and rescan command plans require a
path-shaped swap target from the declaration key, `target`, `path`, or `device`.
Label and UUID updates render `swaplabel --label` and `swaplabel --uuid`;
priority updates render `swapoff` followed by `swapon --priority`;
`operation = "rescan"` renders read-only `swapon --show`, `blkid`, and graph
inspection before any later grow or identity change. Current-topology comparison
maps declared swap label, UUID, and priority properties onto probed identity,
signature, and active swap metadata before suppressing already-satisfied
property updates. It warns when a swap format target already has swap metadata
or matches another node kind,
while keeping the destructive `mkswap` action reviewable. MD RAID
assemble, stop, create, grow, member add,
replacement, and removal command plans require an explicit array path such as
`/dev/md/root`; assemble also requires explicit reviewed member devices. MD
create reconciliation suppresses only already clean active arrays and keeps
degraded, inactive, or wrong-kind matches actionable with warnings. MD stop
reconciliation suppresses absent or inactive arrays and keeps active,
unknown-state, or wrong-kind matches actionable with warnings. MD membership
reconciliation suppresses already-attached adds, already-absent removals, and
completed replacements where the old member is gone and the new member is
attached. MD
RAID rescan plans render read-only `mdadm --detail --scan`,
`mdadm --examine --scan`, and `/proc/mdstat` inventory checks without
assembling arrays. Current-topology comparison suppresses assemble actions only
when the probed array has `md.state` indicating an active or clean array and
both `md.degraded-devices` and `md.failed-devices` are zero; degraded or failed
arrays stay planned and emit a warning.
Loop-device refresh, rescan, and detach command plans require `/dev/loop*`
targets. Rescan reads `losetup --json --list` and graph state without changing
capacity; grow uses `losetup -c` after backing size changes.
Multipath map growth and path replacement preflight require a concrete map
target such as `mpatha` or `/dev/mapper/mpatha`, either as the declaration name
or through explicit `target`/`device` fields. Growth and rescan plans capture
host-visible SCSI path transport and size with `lsscsi -t -s` before map reload
or resize. Replacement renders separate path add and delete steps so each
command can be reviewed independently.
ZFS pool device removal renders reviewed `zpool remove <pool> <device>` steps
when the pool layout supports evacuation. LVM volume group device removal
renders reviewed `pvmove <pv>` then `vgreduce <vg> <pv>` steps so allocated
extents are evacuated before the physical volume is reduced. These remain
potential-data-loss intents unless a safer explicit workflow is selected.
Btrfs filesystem device topology plans support add, replace, and remove
operations. Removal stays potential-data-loss, while rebalance plans render
`btrfs balance start` with optional declared data, metadata, and system filters
from lifecycle properties.
Btrfs subvolume rename plans render reviewed `mv -- <old> <new>` commands and
stay offline-required so mounts, qgroups, snapshots, and send/receive jobs can
move together without deleting the original subvolume.
Current-topology comparison suppresses Btrfs subvolume deletion only for
concrete absolute paths that are already absent. Present subvolumes stay
actionable with subvolume id, generation, parent, top-level, and UUID metadata
when available.
bcachefs filesystem topology plans support add, replace, remove, grow,
rebalance, and scrub operations. Device growth uses `bcachefs device resize`
against a declared member device and desired size. Device add/remove uses
`bcachefs device add` and `bcachefs device remove` against the mounted
filesystem. Replacement is rendered as add replacement capacity, `bcachefs data rereplicate`, then remove the old member, keeping each data-preserving step
visible for review. Rebalance-style plans use `bcachefs data rereplicate`, and
scrub plans use `bcachefs scrub`.
Btrfs filesystem label property updates render
`btrfs filesystem label <path> <label>`. Ext filesystem label updates render
`e2label <device> <label>` when the filesystem declaration includes a backing
device. FAT/vfat label updates render `fatlabel <device> <label>`. NTFS label
updates render `ntfslabel <device> <label>`. exFAT label updates render
`exfatlabel <device> <label>`. F2FS label updates render
`f2fslabel <device> <label>`. XFS filesystem label updates render
`xfs_admin -L <label> <device>`. Btrfs, ext, FAT/vfat, NTFS, exFAT, and XFS
filesystem UUID, volume-ID, or volume-serial updates render
`btrfstune -U <uuid> <device>`, `tune2fs -U <uuid> <device>`,
`fatlabel -i <device> <volume-id>`, `ntfslabel --new-serial=<serial> <device>`,
`exfatlabel -i <device> <serial>`, and `xfs_admin -U <uuid> <device>` and are
offline-required because they mutate filesystem identity used by mounts and boot
paths. FAT volume IDs and exFAT volume serials must be 8 hex digits, and NTFS
volume serials must be 16 hex digits; all allow optional dash grouping.
Current-topology comparison maps declared label, UUID, FAT volume-ID, NTFS
serial, and exFAT serial aliases onto probed node identity and filesystem
metadata, normalizing hex identity spellings before suppressing
already-satisfied property updates. Missing devices stay marked
`needs-domain-implementation`, while unsupported filesystem property keys are
classified as unsupported before execution.
Btrfs subvolume property updates only treat read-only aliases (`readOnly`,
`readonly`, `ro`, `btrfs.readonly`, and `btrfs.ro`) as safe planned property
changes. Other Btrfs subvolume property keys are classified as unsupported so
apply policy blocks them before command execution.
Ext filesystem grow and shrink actions also carry the declared filesystem
`device` or `disk` into `resize2fs` and `e2fsck` command plans. Mountpoint-only
ext declarations keep source-device mutations marked unresolved until the block
device is explicitly selected. F2FS grow actions render `resize.f2fs <device>`
or `resize.f2fs -t <sectors> <device>` when a target sector count is declared,
and keep mountpoint-only plans unresolved until a source device is selected.
Filesystem check and repair actions carry the declared `device` or `disk` into
read-only and mutating maintenance command plans. Ext uses `e2fsck`, XFS uses
`xfs_repair`, Btrfs uses `btrfs check`, FAT/vfat uses `fsck.fat`, exFAT uses
`fsck.exfat`, F2FS uses `fsck.f2fs`, bcachefs uses `bcachefs fsck`, and NTFS
uses `ntfsfix`; repair variants remain offline-required and should be reviewed
after a read-only check. NTFS repair is limited Linux-side remediation and not a
replacement for Windows `chkdsk`.
Btrfs scrub actions use the mounted path and render `btrfs scrub start -B`;
bcachefs scrub actions render `bcachefs scrub`; ZFS pool scrub actions render
`zpool scrub`.
Filesystem trim actions render `fstrim -v` against the mounted target and remain
online maintenance operations.
`disk-nix apply --script-out <path>` writes those allowed command and
verification plans as a reviewable bash script after policy validation passes.
Commands with unresolved inputs remain commented as not ready.
`disk-nix apply --report-out <path>` writes the JSON report before returning a
blocked-policy, not-ready, or failed-execution error, preserving the decision
record for automation and review.
`disk-nix apply --receipt-out <path>` writes an audit receipt that wraps the
same report with receipt version, command name, spec path, probe-current flag,
execute flag, and generation timestamp. This is the preferred artifact for
apply journals and recovery handoff because it preserves how the report was
produced.
`disk-nix validate --spec <path>` emits the same dry-run report but treats
blocked policy as a successful command result, making it the better fit for
CI, preflight checks, and NixOS validation paths that need to inspect blocked
details. `validate --report-out <path>` writes the same report to disk, while
`validate --receipt-out <path>` writes a receipt with `command = "validate"`
and `executeRequested = false`.

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

The default policy allows online grow and property-change intents, but blocks
offline-required, destructive, irreversible, format, shrink, and
potential-data-loss actions. `allowPotentialDataLoss=true` is the explicit
policy override for reviewed rollback, shrink, device removal, and similar
actions.
Unsupported actions are always blocked, even if permissive destructive or
shrink policy flags are enabled.
`allowDeviceReplacement=false` blocks device add, replacement, and removal
actions. `allowRebalance=false` blocks rebalance actions. `requireBackup=true`
requires `backupVerified=true` for destructive or potential-data-loss actions.
`requireConfirmation=true` requires `confirmation=true` for high-risk or
offline actions. `requireConfirmationFile` points at an operator-controlled
file; the CLI treats it as confirmed only when the file contains a standalone
line equal to `disk-nix confirm`, and otherwise leaves the action blocked.
`--execute` requires policy validation and a fully ready command plan. It runs
planned commands sequentially, stops on the first command failure, records
stdout, stderr, and exit status, and only runs verification commands after the
planned command phase succeeds.
