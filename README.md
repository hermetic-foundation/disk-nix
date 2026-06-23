# disk-nix

`disk-nix` is planned as a NixOS-native storage lifecycle manager: a
read-only storage topology engine first, and a safe imperative planner/apply
engine second.

The long-term goal is a full disko replacement that understands modern Linux
storage stacks:

- block devices, partitions, filesystems, mounts, swap, loop devices
- LUKS headers, keyslots, tokens, and device-mapper mappings
- LVM PVs, VGs, LVs, thin pools, snapshots, cache, and VDO
- Btrfs filesystems, devices, subvolumes, snapshots, qgroups, and usage
- ZFS pools, vdevs, datasets, zvols, snapshots, snapshot hold reference counts,
  compression/quota/reservation/encryption properties, cache, log, and special
  vdevs
- MD RAID, multipath, NVMe namespaces, iSCSI sessions/targets/LUNs, and NFS
- safe lifecycle operations such as grow, replace, rebalance, filesystem checks,
  property updates, and migration advice

The project is licensed under AGPL-3.0-or-later from the beginning.

## Current status

This repository is an active implementation. The CLI provides read-only storage
topology views, adapter status reporting, lifecycle planning, policy-gated apply
evaluation, NixOS module integration, and fixture-backed parser coverage for the
storage graph.

## Development

Use the flake:

```sh
nix develop
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Or run all configured checks:

```sh
nix flake check
```

The flake checks build the CLI, run workspace tests, validate the NixOS module,
and execute the checked-in example specs through `plan`, `apply`, and
`--script-out` so JSON contracts and review-script generation stay covered.

## CLI

```sh
disk-nix topology
disk-nix topology --json
disk-nix probe-status
disk-nix probe-status --json
disk-nix capabilities
disk-nix capabilities --json
disk-nix devices
disk-nix partitions
disk-nix filesystems
disk-nix volumes
disk-nix pools
disk-nix snapshots
disk-nix mappings
disk-nix mounts
disk-nix network-storage
disk-nix ids
disk-nix ids --json
disk-nix usage
disk-nix usage --json
disk-nix inspect /dev/nvme0n1
disk-nix inspect /
disk-nix inspect / --json
disk-nix plan --spec ./examples/simple-root.json
disk-nix plan --spec ./examples/lifecycle-update.json
disk-nix plan --spec ./examples/simple-root.json --json
disk-nix plan --spec ./examples/simple-root.json --probe-current --json
disk-nix apply --spec ./examples/lifecycle-update.json
disk-nix apply --spec ./examples/lifecycle-update.json --json
disk-nix apply --spec ./examples/lifecycle-update.json --probe-current --json
disk-nix apply --spec ./examples/simple-root.json --script-out ./disk-nix-apply.sh
disk-nix apply --spec ./examples/lifecycle-update.json --report-out ./apply-report.json
disk-nix validate --spec ./examples/lifecycle-update.json --json
disk-nix schema
disk-nix completions bash
disk-nix manpage
```

The canonical interface is intended to be stable JSON. Human tables and tree
views are presentation layers over the same model. Focused JSON commands such
as `devices --json`, `partitions --json`, `pools --json`,
`snapshots --json`, `network-storage --json`, `ids --json`, and
`usage --json` return subgraphs and preserve relationships between nodes
included in the result. `usage` summarizes size, used, free, allocated,
utilization, and selected metadata details across graph nodes that expose
capacity data.
exFAT probing uses `tune.exfat` and `dump.exfat` to add label, GUID, serial,
sector, cluster, size, and free-space metadata when exfatprogs is available.
`inspect --json` returns matched nodes plus their direct neighbors and
relationship edges. `capabilities --json` returns the modeled operation/risk
matrix.
The Nix package installs generated bash, zsh, and fish completions plus a
`disk-nix(1)` manpage. The `completions` and `manpage` commands can also emit
those artifacts directly.
`schema` emits the supported desired-spec JSON contract for editor integration
and automation; the Nix package also installs it at
`share/disk-nix/schema/disk-nix-spec.schema.json`.

See [docs/cli.md](docs/cli.md) for the command reference and JSON contracts.

## NixOS module

The flake exposes a NixOS module:

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;
    apply.mode = "activation";
    apply.probeCurrent = true;
    apply.failOnBlocked = true;
    apply.execute = false;
    apply.scriptOut = "/run/disk-nix/apply.sh";
    apply.reportOut = "/run/disk-nix/apply-report.json";
  };
}
```

The module installs the CLI plus default storage tooling, writes a normalized
storage spec to `/etc/disk-nix/spec.json`, derives typed NixOS `fileSystems`,
`swapDevices`, and initrd LUKS options, and keeps lifecycle domains available
in the same planner spec. Override `toolPackages` to pin alternate tool builds
or trim unused domains. Explicit non-destroy `exports` declarations with
`client` and `options` also derive NixOS NFS server export lines. When
typed `nfs.mounts` declarations are marked for destroy they stay in the
disk-nix spec for reviewed unmount planning but are not re-added to NixOS
`fileSystems`.
`apply.scriptOut` is set, activation validation asks the CLI to write the
allowed command plan and post-apply verification plan to that reviewable shell
script path. When `apply.reportOut` is set, activation also writes the JSON
report before returning blocked-policy failures. Set
`apply.failOnBlocked = false` to use report-only validation during activation;
blocked actions are still reported, but the unit exits successfully. Set
`apply.execute = true` only when activation should run ready, policy-allowed
commands through `disk-nix apply --execute`; this requires
`apply.failOnBlocked = true` and still writes the requested review artifacts.

## Safety model

`disk-nix` treats all mutation as planned work:

1. discover current topology
1. normalize it into a typed graph
1. compare it with desired state
1. classify every action by risk
1. recommend non-destructive alternatives where possible
1. require explicit policy before mutation
1. verify after execution

No destructive operation should be implicit.

`disk-nix apply` defaults to a policy-gated dry run. It evaluates the planned
actions against the `apply` policy in the spec, reports blocked operations,
emits advisory command and verification plans, and can write those plans to a
reviewable shell script with `--script-out`. With `--execute`, disk-nix runs
only policy-allowed plans where every command is ready, records each command
result, stops on the first failure, and runs verification commands only after
the planned command phase succeeds.
Planner coverage includes filesystem resize intent, disk and partition
lifecycle declarations, swap signature/resize workflows, LUKS format/resize/close/keyslot/token
workflows, Btrfs subvolume creation/deletion, VDO create/grow/remove, LVM
physical-volume create/grow/remove, logical-volume growth/removal,
LVM volume-group extension/device removal, LVM thin-pool create/grow/remove,
LVM snapshot create/merge/remove, LVM cache attach/detach/property updates,
loop-device mapping updates, MD RAID lifecycle/member updates, multipath map
updates, NVMe namespace create/attach/rescan/detach/delete workflows, ZFS pool
topology updates, dataset and zvol updates including zvol property changes,
volume updates, network LUN growth, snapshots, and cache
attach/detach/replacement workflows.
Cache apply plans include bcache-aware attach, detach, cache-mode, dirty-data,
and replacement review steps instead of a generic cache placeholder. bcache
sysfs commands require a concrete `/dev/bcache*` target; logical cache names
remain non-ready until the backing bcache device path is declared.
LVM cache apply plans use separate `lvmCaches` declarations and render
`lvconvert --type cache`, `lvconvert --uncache`, and `lvchange --cachemode` or
`--cachepolicy` commands when an origin `vg/lv` and cache-pool LV are declared.
Btrfs filesystem device topology plans render `btrfs device add`,
`btrfs replace start`, and allocation-inspected `btrfs device remove` commands
for review. Removal remains blocked by the current potential-data-loss policy
gate.
Btrfs filesystem rebalance plans render `btrfs balance start` and use declared
data, metadata, and system balance filters from lifecycle properties when set.
Btrfs scrub plans render `btrfs scrub start -B`; ZFS pool scrub plans render
`zpool scrub`.
Filesystem trim plans render reviewed `fstrim -v <mountpoint>` commands for
mounted filesystems.
Regular Btrfs filesystem label updates render
`btrfs filesystem label <path> <label>`. Ext filesystem label updates render
`e2label <device> <label>` when the declaration includes an explicit backing
device. FAT/vfat label updates render `fatlabel <device> <label>`. NTFS label
updates render `ntfslabel <device> <label>`. exFAT label updates render
`exfatlabel <device> <label>`. XFS filesystem label updates render
`xfs_admin -L <label> <device>`. Btrfs, ext, FAT/vfat, NTFS, exFAT, and XFS
filesystem UUID, volume-ID, or volume-serial updates render
`btrfstune -U <uuid> <device>`, `tune2fs -U <uuid> <device>`,
`fatlabel -i <device> <volume-id>`, `ntfslabel --new-serial=<serial> <device>`,
`exfatlabel -i <device> <serial>`, and `xfs_admin -U <uuid> <device>` as
offline-required identity changes.
Missing backing devices keep the command non-ready until the source device is
resolved.
Unsupported filesystem properties are classified as unsupported so apply policy
blocks them until a domain-specific command mapping exists.
Filesystem shrink plans render Btrfs usage checks and `btrfs filesystem resize`
commands when a desired size is declared. Ext shrink plans render source
resolution, unmount, `e2fsck`, and `resize2fs` steps. Ext grow and shrink
commands use a declared filesystem `device` or `disk` when present, and leave
source-device commands unresolved when only a mountpoint is declared. XFS shrink
remains manual-only migration guidance.
Filesystem check and repair plans render `e2fsck -n`/`e2fsck -f -y`,
`xfs_repair -n`/`xfs_repair`, and `btrfs check --readonly`/`--repair` command
plans for ext, XFS, and Btrfs. Repair is offline-required and mutates
filesystem metadata; mountpoint-only declarations remain non-ready until the
source block device is selected.
Btrfs subvolume property updates render read-only toggles with
`btrfs property set -ts <path> ro true|false`; unsupported Btrfs subvolume
properties are classified as unsupported with manual-review alternatives.
Btrfs qgroup lifecycle plans render `btrfs qgroup create`, policy-gated
`btrfs qgroup destroy`, and `btrfs qgroup limit` updates for referenced and
exclusive byte limits from `btrfsQgroups` declarations. Executable qgroup
create, destroy, and limit plans require a mounted filesystem `target` path.
Swapfile grow plans render reviewed `swapoff`, `fallocate --length`, `mkswap`,
and `swapon` steps while keeping block-device backing growth explicit. Swap
grow and format commands require a path-shaped swap target such as `/swapfile`
or `/dev/disk/by-*`.
LUKS open plans render reviewed `cryptsetup open` commands for preserved
existing containers; close plans render offline-policy-gated `cryptsetup close`
commands and verify the topology without erasing the backing LUKS header or
encrypted data.
LUKS keyslot and token plans render `cryptsetup luksAddKey`, `luksChangeKey`,
`luksKillSlot`, `cryptsetup token import`, and `cryptsetup token remove` with
header verification. Keyslot and token removal are potential data loss because
they can remove the last working unlock path.
Disk initialization plans render destructive-policy-gated `parted mklabel` and
partition table reread steps after disk identity inspection.
Partition create plans render reviewed `parted mkpart`, `partprobe`, and
`blockdev --rereadpt` commands when `device`, `partitionType`, `start`, and
`end` are declared.
Partition grow plans render reviewed `parted resizepart` commands and partition
table rereads when `device`, `partitionNumber`, and `end` or `desiredSize` are
declared.
MD RAID create plans render destructive-policy-gated `mdadm --create` commands
from explicit member devices and RAID level declarations, with exact
unresolved-input markers when either field is missing. MD create, grow, and
member-removal command plans require an explicit array path such as
`/dev/md/root`; logical array names remain non-ready.
VDO apply plans render gated `vdo create` and `vdo remove` commands, plus
online `vdo growLogical` and physical growth review steps. Create preflight is
marked unresolved until a backing device is declared. VDO property updates
cover `auto`, `sync`, and `async` write policy changes, compression, and
deduplication with concrete `vdo` commands; unsupported properties and invalid
property values are classified as unsupported before execution.
NFS export apply plans render reviewed `exportfs` create, option update, and
unexport commands from explicit client and option declarations. Export
mutations require a path-shaped local export target such as `/srv/share`.
NFS client mount apply plans render reviewed `mount` create commands and
`umount` destroy commands from `nfs.mounts`; missing sources or concrete
mountpoints remain non-ready.
iSCSI session apply plans render reviewed `iscsiadm` discovery, login, logout,
and rescan commands from explicit target IQN and portal declarations. LUN apply
plans model host-side attach, growth rescan, and detach: attach and grow rescan
sessions, grow rescans declared SCSI paths, and detach deletes only declared
stable path devices before refreshing multipath. Executable LUN attach, grow,
and detach plans require declared stable `device` or `devices` paths.
Generic snapshot plans render reviewed ZFS `zfs snapshot` and Btrfs
`subvolume snapshot` commands when the snapshot naming clearly identifies the
domain. Btrfs snapshot declarations with `readOnly = true` render
`btrfs subvolume snapshot -r`.
Snapshot deletion plans render policy-gated `zfs destroy` and
`btrfs subvolume delete` commands for unambiguous ZFS snapshot names and Btrfs
absolute snapshot paths.
ZFS snapshot hold plans render safe `zfs hold <tag> <snapshot>` and
`zfs release <tag> <snapshot>` updates from `hold` and `releaseHold`
declarations so retention can be changed without deleting recovery points.
ZFS snapshot rollback plans render reviewed `zfs rollback` details while
remaining blocked by the potential-data-loss policy gate.
ZFS dataset apply plans render reviewed `zfs create` commands with declared
properties as create-time `-o key=value` options, plus policy-gated
`zfs destroy` commands.
LVM logical volume apply plans render reviewed `lvcreate` and gated
`lvremove` steps for volume lifecycle declarations, with unresolved markers for
missing `vg/lv` targets or sizes. Grow and remove commands also require that
canonical `vg/lv` target form before they are executable.
LVM thin-pool apply plans render reviewed `lvcreate --type thin-pool`,
`lvextend`, and gated `lvremove` steps, with unresolved markers for missing
`vg/pool` targets or sizes. Thin-pool grow and remove commands likewise
require the canonical `vg/pool` target form.
LVM volume group apply plans render gated `vgcreate` and `vgremove` steps for
volume group lifecycle declarations, reviewed `vgextend` steps for grow
operations with an explicit physical volume, and reviewed `pvmove` then
`vgreduce` steps for explicit physical-volume removal.
Generic add-device, replace-device, and remove-device operations stay non-ready
until the device to add, source device, replacement device, or device to remove
is declared explicitly.
Loop-device refresh and detach commands require `/dev/loop*` targets. Multipath
map growth requires a concrete map target such as `mpatha` or
`/dev/mapper/mpatha`; arbitrary logical map names remain non-ready.
ZFS pool apply plans render gated `zpool create` commands from a single
`device` or an explicit `devices` vdev list, gated `zpool destroy` commands,
and reviewed topology updates such as `zpool add`, `zpool replace`, and
`zpool remove`. Pool create preflight inspects path-like vdev entries before
rendering the mutating command.
`disk-nix validate` emits the same dry-run report but exits successfully when
policy blocks actions, which makes it suitable for CI and NixOS config checks.
Use `--report-out` with either command to persist the JSON report for review
even when policy blocks the operation.
