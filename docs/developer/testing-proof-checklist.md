## Testing and proof

- [x] **Finished:** Unit tests across model, probe, plan, exec, and CLI behavior.
- [x] **Finished:** Nix flake checks for package build, tests, clippy, module checks.

Nix flake checks for package build, tests, clippy, module checks, examples, schema
checks, completions, manpage output, docs freshness, and integration harness syntax.

- [x] **Finished:** Root-only opt-in smoke harnesses for loop-backed and selected
  lab-backed storage domains.
- [x] **Finished:** Translated upstream disko example suite covers all 40 generated specs.

Translated upstream disko example suite covers all 40 generated specs with dry-run and
destructive-shape preflight gates requiring zero blocked, unresolved, manual-only, or
non-ready commands.

Guarded destructive execution runs the non-ZFS/non-bcachefs specs on disposable stable
`/dev/disk/by-id` lab disks. ZFS and bcachefs specs are capability-skipped only when
the host kernel lacks those filesystems.

The current lab host maps that stable set to the disks currently enumerated as
`/dev/sda` and `/dev/sdc` through `/dev/sdf`, excluding `/dev/sdb` because it is the
system disk after the reboot.

- [x] **Finished:** Smoke harness coverage for loop devices, Btrfs, bcachefs, LUKS,
  LVM, MD RAID, ZFS, NFS, VDO, iSCSI, multipath, NVMe, and synthetic failed-apply
  recovery.
- [x] **Finished:** Synthetic failed-command recovery covers layered LVM-plus-filesystem.

Synthetic failed-command recovery covers layered LVM-plus-filesystem, LVM grow, LVM
thin-pool create/grow, XFS grow, Btrfs scrub/rebalance/device replacement, bcachefs
replacement, filesystem trim/check/repair/property, swap label, zram rescan/property
inventory, loop rescan, backing-file rescan/grow/create, device-mapper rename, ZFS
dataset rename, Btrfs/ZFS snapshot clone, LVM VG rename/replacement, ZFS pool
replacement, and ZFS rollback paths.

- [x] **Finished:** Synthetic failed-command recovery covers NVMe namespace create/grow/attach/detach/delete.

Synthetic failed-command recovery covers NVMe namespace
create/grow/attach/detach/delete, host-side LUN rescan, target-side LUN LIO/tgt/SCST
lifecycle/property/rescan paths, and multipath add/remove/flush/resize/replace paths.

- [x] **Finished:** Synthetic failed-command recovery covers MD RAID.

Synthetic failed-command recovery covers MD RAID
create/assemble/stop/grow/add-member/remove-member/replace, LUKS
open/format/close/grow/keyslot/token/property, partition grow, NFS
remount/unmount/export/unexport, iSCSI logout/login/rescan, LVM cache
attach/detach/replacement/rescan/property, VDO lifecycle/property, and bcache
replacement/property/rescan paths.

- [x] **Finished:** Destructive integration tests include real or lab-backed device replacement coverage.

Destructive integration tests include real or lab-backed device replacement coverage
for MD RAID, ZFS pools, Btrfs filesystems, bcachefs, bcache, LVM cache, and
multipath-backed stacks.

- [x] **Finished:** Destructive integration tests include real MD RAID member replacement coverage.

Destructive integration tests include real MD RAID member replacement coverage: the
loop-backed MD harness creates a disposable RAID1 array, applies
`mdRaids.*.replaceDevices`, executes `mdadm <array> --replace <old-loop> --with
<new-loop>`, waits for replacement completion with `mdadm --wait`, and verifies the
replacement member appears in `mdadm --detail` before later degraded-array checks.

- [x] **Finished:** Destructive integration tests include real ZFS pool device replacement coverage.

Destructive integration tests include real ZFS pool device replacement coverage: the
loop-backed ZFS harness creates disposable source and replacement vdevs, applies
`pools.*.replaceDevices`, executes `zpool replace <pool> <old-loop> <new-loop>`,
verifies the replacement device appears in `zpool status -P`, and confirms the pool
mountpoint remains active.

- [x] **Finished:** Destructive integration tests include real Btrfs filesystem device replacement coverage.

Destructive integration tests include real Btrfs filesystem device replacement
coverage: the loop-backed Btrfs harness creates disposable source and replacement
devices, writes a sentinel file, applies `filesystems.*.replaceDevices`, executes
`btrfs replace start <old-loop> <new-loop> <mountpoint>`, verifies the replacement
device appears in `btrfs filesystem show`, and confirms the sentinel remains readable
from the mounted filesystem.

- [x] **Finished:** Destructive integration tests include real bcachefs member replacement coverage.

Destructive integration tests include real bcachefs member replacement coverage: the
loop-backed bcachefs harness creates disposable source and replacement devices, writes
a sentinel file, applies `filesystems.*.replaceDevices`, executes `bcachefs device
add`, `bcachefs data rereplicate`, and `bcachefs device remove`, verifies
replacement-device superblock metadata with `bcachefs show-super`, and confirms the
sentinel remains readable from the mounted filesystem.

- [x] **Finished:** Destructive integration tests include broader degraded-array variants covering missing members.

Destructive integration tests include broader degraded-array variants covering missing
members, stale superblocks, replacement races, partial rebuilds, failed detach, and
failed reattach behavior.

- [x] **Finished:** Destructive integration tests include MD RAID stale-superblock coverage.

Destructive integration tests include MD RAID stale-superblock coverage: after the
loop-backed MD harness replaces a member and then fails/removes the replacement member,
it runs `mdadm --examine <removed-loop>` and verifies the removed member still exposes
stale array metadata before rerunning degraded-array inspection.

- [x] **Finished:** Destructive integration tests include MD RAID failed-detach coverage.

Destructive integration tests include MD RAID failed-detach coverage: after the
loop-backed MD harness removes a replacement member, it applies
`mdRaids.*.removeDevices` for the already-removed member, verifies the real `mdadm`
detach command fails, and checks the apply report contains partial-execution metadata,
retry review, domain recovery, and roll-forward review.

- [x] **Finished:** Destructive integration tests include MD RAID failed-reattach coverage.

Destructive integration tests include MD RAID failed-reattach coverage: after the
loop-backed MD harness is degraded, it applies `mdRaids.*.addDevices` for a missing
member path, verifies the real `mdadm <array> --add <missing-path>` command fails, and
checks the apply report contains partial-execution metadata, retry review, domain
recovery, and roll-forward review.

- [x] **Finished:** Destructive integration tests include MD RAID partial-rebuild.

Destructive integration tests include MD RAID partial-rebuild and replacement-race
coverage: after stale-superblock and failed-reattach checks, the loop-backed MD harness
bounds the array rebuild window through MD sysfs `sync_max`, applies
`mdRaids.*.addDevices` for the stale removed member, verifies a real `mdadm <array>
--add <stale-loop>` succeeds while rebuild progress is only partial.

The harness then restores the rebuild limit, waits for completion with
`mdadm --wait`, and verifies the member returns to the array.

- [x] **Finished:** Destructive integration tests include MD RAID degraded missing-member coverage.

Destructive integration tests include MD RAID degraded missing-member coverage: the
loop-backed MD harness creates a temporary RAID1 array, fails and removes one member,
verifies `disk-nix inspect` still sees degraded array metadata, and reruns the
read-only MD rescan apply.

- [x] **Finished:** Destructive integration tests include real cache mutation coverage.

Destructive integration tests include real cache mutation coverage for LVM cache
attach/detach/replacement, bcache replacement, and cache-device failure states.

- [x] **Finished:** Destructive integration tests include real LVM cache detach.

Destructive integration tests include real LVM cache detach and reattach data-survival
coverage: the loop-backed LVM harness writes an ext4 sentinel to a cached origin LV,
applies `lvmCaches.*.removeDevices` and verifies `lvconvert --uncache`, verifies the
sentinel remains readable, applies `lvmCaches.*.addDevices` and verifies `lvconvert
--type cache --cachepool`, then verifies the sentinel remains readable after the cache
is restored.

- [x] **Finished:** Destructive integration tests include real LVM cache replacement data-survival coverage.

Destructive integration tests include real LVM cache replacement data-survival
coverage: the loop-backed LVM harness creates a replacement cache pool, applies
`lvmCaches.*.replaceDevices`, verifies the rendered `disk-nix-lvm-cache-replace`
wrapper runs `lvconvert --uncache` before attaching the replacement cache pool, and
confirms the cached-origin ext4 sentinel remains readable after replacement.

- [x] **Finished:** Destructive integration tests include real bcache cache detach/reattach coverage.

Destructive integration tests include real bcache cache detach/reattach coverage: the
loop-backed bcache harness derives the live cache-set UUID, applies
`caches.*.removeDevices`, verifies the `disk-nix-bcache-detach` sysfs write, applies
`caches.*.addDevices`, verifies the `disk-nix-bcache-attach` sysfs write, reapplies
cache mode, and confirms the generated bcache device remains readable.

- [x] **Finished:** Destructive integration tests include real bcache cache-device failure-state coverage.

Destructive integration tests include real bcache cache-device failure-state coverage:
after detaching the live cache set, the loop-backed bcache harness applies
`caches.*.addDevices` with an invalid cache-set UUID, verifies the
`disk-nix-bcache-attach` sysfs write fails, and checks the failed report contains
partial-execution metadata, retry review, domain recovery, and roll-forward review
before reattaching the valid cache.

- [x] **Finished:** Destructive integration tests include real bcache cache replacement coverage.

Destructive integration tests include real bcache cache replacement coverage: the
loop-backed bcache harness creates a replacement cache loop, applies
`caches.*.replaceDevices` with the live cache-set UUID, verifies the rendered
`disk-nix-bcache-replace` wrapper initializes the replacement cache, detaches the prior
cache, attaches the replacement cache set, and confirms the generated bcache device
remains readable after replacement.

- [x] **Finished:** Destructive integration tests include real bcache read-only rescan coverage.

Destructive integration tests include real bcache read-only rescan coverage: the
loop-backed bcache harness applies `caches.bcacheSmoke.operation = "rescan"` against
the generated bcache device and verifies `disk-nix inspect` plus `disk-nix-bcache-read`
checks for `state`, `cache_mode`, and `dirty_data` all succeed.

- [x] **Finished:** Destructive integration tests include lab-backed NVMe namespace coverage.

Destructive integration tests include lab-backed NVMe namespace coverage for create,
grow, attach, detach, delete, controller reconnect, and namespace identity drift.

- [x] **Finished:** Destructive integration tests include lab-backed NVMe namespace create/delete coverage.

Destructive integration tests include lab-backed NVMe namespace create/delete coverage:
when `DISK_NIX_NVME_CREATE_DELETE=1` is set with explicit namespace size, namespace id,
and controller list, the NVMe harness applies `nvmeNamespaces.<controller>.operation =
"create"`, verifies `nvme create-ns`, `nvme attach-ns`, and namespace rescan, then
applies a destructive cleanup plan and verifies `nvme detach-ns`, `nvme delete-ns`, and
final namespace inventory.

- [x] **Finished:** Destructive integration tests include lab-backed NVMe namespace identity-drift assertions.

Destructive integration tests include lab-backed NVMe namespace identity-drift
assertions for create/delete: the harness verifies the selected namespace id appears in
`nvme list-ns --all --output-format=json` after create and is absent after delete.

- [x] **Finished:** Destructive integration tests include lab-backed NVMe namespace grow coverage: when.

Destructive integration tests include lab-backed NVMe namespace grow coverage: when
`DISK_NIX_NVME_GROW=1` is set, the NVMe harness applies
`nvmeNamespaces.<controller>.operation = "grow"` with reviewed grow policy and verifies
`nvme list-subsys`, `nvme ns-rescan`, report persistence, and post-grow namespace
inventory.

- [x] **Finished:** Destructive integration tests include lab-backed NVMe namespace attach/detach coverage.

Destructive integration tests include lab-backed NVMe namespace attach/detach coverage:
when `DISK_NIX_NVME_ATTACH_DETACH=1` is set with an explicit disposable namespace id
and controller list, the NVMe harness applies `nvmeNamespaces.<controller>.operation =
"attach"`, verifies `nvme attach-ns` and `nvme ns-rescan`, applies the matching detach
plan, and verifies `nvme detach-ns` and a final namespace rescan.

- [x] **Finished:** Destructive integration tests include lab-backed NVMe controller reconnect coverage: when.

Destructive integration tests include lab-backed NVMe controller reconnect coverage:
when `DISK_NIX_NVME_RECONNECT=1` is set with an explicit NQN, transport, target
address, optional service id, and expected controller path, the NVMe harness runs `nvme
disconnect`, reconnects with `nvme connect`, waits for the expected controller,
verifies `disk-nix inspect` sees it, and reruns the namespace rescan apply.

- [x] **Finished:** Destructive integration tests include lab-backed multipath flush coverage: when.

Destructive integration tests include lab-backed multipath flush coverage: when
`DISK_NIX_MULTIPATH_FLUSH=1` is set, the multipath harness applies
`multipathMaps.flush.destroy = true` with `allowDestructive = true` and `backupVerified
= true`, then verifies `multipath -ll <map>` and `multipath -f <map>` succeed.

- [x] **Finished:** Destructive integration tests include lab-backed multipath path add/remove coverage: when.

Destructive integration tests include lab-backed multipath path add/remove coverage:
when `DISK_NIX_MULTIPATH_ADD_PATH` or `DISK_NIX_MULTIPATH_REMOVE_PATH` is set, the
multipath harness applies `multipathMaps.paths.addDevices` and/or
`multipathMaps.paths.removeDevices` for the explicit paths and verifies `multipathd add
path <path>` and `multipathd del path <path>` succeed.

- [x] **Finished:** Destructive integration tests include lab-backed multipath path replacement coverage: when.

Destructive integration tests include lab-backed multipath path replacement coverage:
when `DISK_NIX_MULTIPATH_REPLACE_OLD_PATH` and `DISK_NIX_MULTIPATH_REPLACE_NEW_PATH`
are set, the multipath harness applies `multipathMaps.paths.replaceDevices` for the
explicit path pair and verifies `multipathd add path <new-path>` succeeds before
`multipathd del path <old-path>` succeeds.

- [x] **Finished:** Destructive integration tests include lab-backed multipath resize coverage: when.

Destructive integration tests include lab-backed multipath resize coverage: when
`DISK_NIX_MULTIPATH_RESIZE=1` is set, the multipath harness applies
`multipathMaps.resize.operation = "grow"` for the selected map and verifies `multipath
-ll`, `lsscsi -t -s`, `multipathd resize map <map>`, and `multipath -r` all succeed.

- [x] **Finished:** Destructive integration tests include lab-backed host-side LUN rescan coverage: when.

Destructive integration tests include lab-backed host-side LUN rescan coverage: when
`DISK_NIX_LUN_PATH` is set, the iSCSI harness applies `luns.<target>:0.operation =
"rescan"` for that path and verifies `iscsiadm --mode session --rescan`,
`disk-nix-scsi-rescan`, `lsscsi -t -s`, and `multipath -r` all succeed.

- [x] **Finished:** Destructive integration tests include LIO target-side map/unmap coverage: the loop-backed.

Destructive integration tests include LIO target-side map/unmap coverage: the
loop-backed target LUN harness creates a second temporary backstore, applies
`targetLuns.<iqn>.operation = "attach"` with a reviewed initiator ACL, verifies the LUN
and ACL are present, applies `targetLuns.<iqn>.operation = "detach"`, and verifies the
LUN is unmapped without deleting the backstore.

- [x] **Finished:** Destructive integration tests include target-side LUN destroy refusal coverage.

Destructive integration tests include target-side LUN destroy refusal coverage: the
loop-backed LIO harness submits `targetLuns.<iqn>.destroy = true` without
`allowDestructive = true`, verifies the plan is blocked as destructive before any
command is rendered, and checks the review-policy recovery guidance prefers
non-destructive alternatives.

- [x] **Finished:** Destructive integration tests include real filesystem property mutation coverage.

Destructive integration tests include real filesystem property mutation coverage: the
loop-backed ext4 harness applies a disk-nix `filesystems.*.properties.label`
declaration, executes `e2label`, and verifies the resulting label on the disposable
loop device.

- [x] **Finished:** Destructive integration tests include real LUKS header property mutation coverage.

Destructive integration tests include real LUKS header property mutation coverage: the
loop-backed LUKS harness applies a disk-nix `luks.devices.*.properties.label`
declaration, executes `cryptsetup config`, and verifies the resulting label with
`cryptsetup luksDump` on the disposable loop-backed container.

- [x] **Finished:** Destructive integration tests include real Btrfs filesystem property mutation coverage.

Destructive integration tests include real Btrfs filesystem property mutation coverage:
the loop-backed Btrfs harness applies a disk-nix `filesystems.*.properties.label`
declaration, executes `btrfs filesystem label`, and verifies the resulting label on the
mounted disposable Btrfs filesystem.

- [x] **Finished:** Destructive integration tests include real swap signature property mutation coverage.

Destructive integration tests include real swap signature property mutation coverage:
the loop-backed swap harness applies a disk-nix `swaps.*.properties.label` declaration,
executes `swaplabel`, and verifies the resulting label with `blkid` on the disposable
loop-backed swap signature.

- [x] **Finished:** Destructive integration tests include real ZFS pool property mutation coverage.

Destructive integration tests include real ZFS pool property mutation coverage: the
loop-backed ZFS harness applies a disk-nix `pools.*.properties.autotrim` declaration,
executes `zpool set`, and verifies the resulting property with `zpool get` on the
disposable loop-backed pool.

- [x] **Finished:** Destructive integration tests include real LVM cache property mutation coverage.

Destructive integration tests include real LVM cache property mutation coverage: the
loop-backed LVM harness creates a disposable cached origin LV, applies a disk-nix
`lvmCaches.*.properties.lvm.cache-mode` declaration, executes `lvchange --cachemode`,
and verifies the resulting mode with `lvs`.

- [x] **Finished:** Destructive integration tests include real VDO volume property mutation coverage.

Destructive integration tests include real VDO volume property mutation coverage: the
lab-target VDO harness applies a disk-nix `vdoVolumes.*.properties.writePolicy`
declaration, executes `vdo changeWritePolicy`, and verifies the resulting policy with
`vdo status --name` on the selected disposable VDO volume.

- [x] **Finished:** Destructive integration tests include real NFS export property mutation coverage: the NFS.

Destructive integration tests include real NFS export property mutation coverage: the
NFS lab harness can opt into a server-side temporary export, applies a disk-nix
`exports.*.properties.options` declaration, executes `exportfs -i`, and verifies the
export with `exportfs -v`.

- [x] **Finished:** Destructive integration tests include real bcache property mutation coverage: the bcache.

Destructive integration tests include real bcache property mutation coverage: the
bcache harness creates disposable loop-backed backing and cache devices, applies a
disk-nix `caches.*.properties."bcache.cache-mode"` declaration, executes the
`disk-nix-bcache-property` sysfs write, and verifies `cache_mode` reports
`writethrough`.

- [x] **Finished:** Destructive integration tests include real loop-device property mutation coverage.

Destructive integration tests include real loop-device property mutation coverage: the
loop harness applies `loopDevices.*.properties."loop.read-only"` to a disposable loop
device, executes `blockdev --setro` and `blockdev --setrw`, and verifies the read-only
state with `blockdev --getro`.

- [x] **Finished:** Destructive integration tests include real backing-file property mutation coverage.

Destructive integration tests include real backing-file property mutation coverage: the
loop harness applies `backingFiles.*.properties.mode` to its temporary backing image,
executes `chmod 0600`, and verifies the mode with `stat`.

- [x] **Finished:** Destructive integration tests include real zram property reconciliation coverage: the zram.

Destructive integration tests include real zram property reconciliation coverage: the
zram harness applies `zram.properties.algorithm` and `zram.properties.priority`,
verifies the `zram:set-property:*` actions stay non-mutating, executes real `zramctl
--bytes --raw --noheadings --output-all`, `swapon --show`, and `disk-nix zram`
inventory commands, and confirms the plan points operators to NixOS `zramSwap`
reconciliation.

- [x] **Finished:** Destructive integration tests include real target-side LUN property mutation coverage.

Destructive integration tests include real target-side LUN property mutation coverage:
the LIO harness creates a temporary loop-backed block backstore and target LUN, applies
`targetLuns.*.properties."lio.writeCache"`, executes `targetcli ... set attribute
emulate_write_cache=0`, and removes the temporary target state during cleanup.

- [x] **Finished:** Destructive integration tests include VM-backed failure injection.

Destructive integration tests include VM-backed failure injection for a partially
completed apply run: the layered VM harness performs a real `lvextend --resizefs`, then
intentionally fails a real `xfs_growfs` against the ext4 mount instead of relying only
on fake-tool synthetic command failures.

- [x] **Finished:** Default VM suite includes the synthetic failure-recovery harness.
- [x] **Finished:** Disposable partitioned loop/LUKS/LVM/ext4 layered VM grow harness executes one disk-nix.

Disposable partitioned loop/LUKS/LVM/ext4 layered VM grow harness executes one disk-nix
apply run that grows the partition with `growpart`, resizes the LUKS mapper with
`cryptsetup resize`, grows the LV with `lvextend --resizefs`, executes `resize2fs`,
remounts with reviewed options, then unmounts/deactivates the stack, executes a
disk-nix LUKS close plan, reopens the mapper, remounts the LV, and verifies sentinel
data survived.

- [x] **Finished:** Deeper destructive VM tests include a multi-domain mutation scenario.

Deeper destructive VM tests include a multi-domain mutation scenario that combines
partition growth, LUKS growth, LVM changes, filesystem growth, and mount/remount
verification in one apply run.

- [x] **Finished:** Deeper destructive VM tests inject a command failure after a successful real mutating command.

Deeper destructive VM tests inject a command failure after a successful real mutating
command and assert the recovery report includes completed action ids, failed action id,
failed command, remaining action ids, completed mutating command count, recovery
actions, and fresh-topology review.

- [x] **Finished:** Deeper destructive VM tests assert rollback-review behavior.

Deeper destructive VM tests assert rollback-review behavior for the layered VM failed
apply: read-only rollback precondition commands, recovery-point preservation guidance,
refused rollback recipe status, required topology evidence, empty
reversible/destructive mutation sections, and operator-only guidance instead of
automated unsafe rollback.

- [x] **Finished:** Deeper destructive VM tests assert layered block/filesystem data survival across failed.

Deeper destructive VM tests assert layered block/filesystem data survival across failed
and resumed apply runs: after the injected `xfs_growfs` failure, the harness runs a
resumed remount apply, verifies the sentinel remains readable, then closes/reopens the
LUKS stack and verifies the sentinel again.

- [x] **Finished:** Deeper destructive VM tests include LVM cache data-survival assertions: the loop-backed.

Deeper destructive VM tests include LVM cache data-survival assertions: the loop-backed
LVM harness formats the cached origin as ext4, writes a sentinel file, mutates cache
mode with `lvchange --cachemode`, detaches the cache with `lvconvert --uncache`,
reattaches the cache with `lvconvert --type cache --cachepool`, and verifies the cache
sentinel survives mutation, detach, reattach, and rescan plans.

- [x] **Finished:** Deeper destructive VM tests include data-survival assertions across
  failed and resumed apply runs for network-storage scenarios.
- [x] **Finished:** Deeper destructive tests include NFS failed-and-resumed remount data-survival coverage.

Deeper destructive tests include NFS failed-and-resumed remount data-survival coverage:
when `DISK_NIX_NFS_DATA_SURVIVAL=1` is set, the NFS lab harness writes a sentinel file
to the mounted export, injects a failed remount apply, verifies
`partialExecutionRecovery` and `resume-after-fix` guidance, verifies the sentinel
remains readable, reruns a clean remount apply, and verifies the sentinel remains
readable after the resumed network-storage operation.

- [x] **Finished:** Deeper destructive tests include iSCSI host-LUN failed-and-resumed rescan data-survival.

Deeper destructive tests include iSCSI host-LUN failed-and-resumed rescan data-survival
coverage: when `DISK_NIX_LUN_DATA_SURVIVAL=1` is set with an already-mounted
`DISK_NIX_LUN_MOUNTPOINT`, the iSCSI harness writes a sentinel file to the mounted LUN
filesystem, injects a failed host-side LUN rescan, verifies `partialExecutionRecovery`,
`resume-after-fix`, and domain recovery guidance.

The harness verifies the sentinel remains readable, reruns a clean LUN rescan apply,
and verifies the sentinel remains readable after the resumed network-storage operation.

- [x] **Finished:** Deeper destructive tests include target-side LUN failed-and-resumed detach data-survival.

Deeper destructive tests include target-side LUN failed-and-resumed detach
data-survival coverage: the loop-backed LIO target harness formats the mapped LUN
backing device, writes a sentinel file, injects a failed target-side detach apply
before target state is mutated, verifies `partialExecutionRecovery`,
`resume-after-fix`, and domain recovery guidance.

The harness verifies the sentinel remains readable, reruns a clean detach apply, and
verifies the sentinel remains readable after the resumed network-storage operation.

- [x] **Finished:** Probe-status diagnostics include adapter remediation, structured OS.

Probe-status diagnostics include adapter remediation, structured OS, kernel, effective
UID, tool-version context, and preflight checks for root privilege plus missing,
failing, stderr-only, and empty-output storage tool version probes. Preflight JSON
includes an `adapterRemediation` matrix for built-in adapters and sub-adapters with
canonical domains, tools, likely Nix packages, privilege hints, data hints,
parse-fixture hints, and manual command hints.
