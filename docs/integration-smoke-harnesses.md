# Integration smoke harnesses

This document contains the host-backed and VM-backed smoke harness details. Use [Integration tests](integration-tests.md) for opt-in entrypoints and destructive-suite policy.

## Loop-backed smoke test

The repository includes a root-only loop-backed smoke harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-loop-smoke
```

The harness refuses to run unless `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set.
When enabled, it:

- creates a temporary 64 MiB backing file
- applies `backingFiles.<path>.properties.mode = "0600"` and verifies the rendered
  `chmod 0600 <path>` command changed the temporary backing file mode
- attaches it to the next available `/dev/loop*`
- applies `loopDevices.<loop>.properties."loop.read-only" = true`, verifies the rendered.

applies `loopDevices.<loop>.properties."loop.read-only" = true`, verifies the rendered
`blockdev --setro <loop>` command succeeded, then applies `false` and verifies
`blockdev --setrw <loop>`

- formats the temporary loop device with ext4
- verifies `disk-nix inspect <loop> --json` can see the real loop node
- executes a safe `loopDevices.<loop>.operation = "rescan"` apply plan
- grows the temporary backing file, refreshes the loop device capacity, and executes an
  ext4 `resizePolicy = "grow-only"` apply plan
- executes an ext4 filesystem property apply plan that sets
  `filesystems.loopSmokeLabel.properties.label`
- verifies the rendered `e2label <loop> disknix-loop` command succeeded and the loop
  device reports the new label
- verifies the generated JSON report was written and all executed commands succeeded
- detaches the loop device and removes the backing file during cleanup

The test intentionally formats only the temporary backing file it creates. It
must still be treated as destructive because it uses real kernel loop devices
and filesystem tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-loop-smoke.sh
```

## Btrfs loop-backed smoke test

The repository also includes a root-only Btrfs loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-btrfs-smoke
```

When enabled, it:

- creates a temporary 128 MiB backing file
- attaches it to the next available `/dev/loop*`
- formats the temporary loop device with Btrfs
- mounts the filesystem in the temporary directory
- verifies `disk-nix inspect <mountpoint> --json` sees Btrfs topology
- applies `filesystems.btrfsSmokeLabel.properties.label = "disknix-btrfs"` against the
  mounted Btrfs filesystem
- verifies the generated JSON report was written, the rendered `btrfs filesystem label.

verifies the generated JSON report was written, the rendered `btrfs filesystem label
<mountpoint> disknix-btrfs` command succeeded, and `btrfs filesystem label
<mountpoint>` reports the new label

- executes a `filesystems.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered `btrfs scrub start -B
  <mountpoint>` command succeeded
- writes a sentinel file, applies a `filesystems.<name>.replaceDevices` plan from the original.

writes a sentinel file, applies a `filesystems.<name>.replaceDevices` plan from the
original loop device to a second loop device, verifies the rendered `btrfs replace
start <old-loop> <new-loop> <mountpoint>` command succeeded, confirms the replacement
device appears in `btrfs filesystem show`, and checks the sentinel remains readable
from the mounted filesystem

- unmounts, detaches both loop devices, and removes the backing files during cleanup

This test intentionally formats and mounts only the temporary backing files it
creates. It still requires destructive opt-in because it uses real loop, mount,
and Btrfs tooling, including a real filesystem device replacement.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-btrfs-smoke.sh
```

## bcachefs loop-backed smoke test

The repository also includes a root-only bcachefs loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-bcachefs-smoke
```

When enabled, it:

- creates a temporary 512 MiB backing file
- attaches it to the next available `/dev/loop*`
- formats the temporary loop device with bcachefs
- mounts the filesystem in the temporary directory
- verifies `disk-nix inspect <mountpoint> --json` sees bcachefs topology
- executes a `filesystems.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered `bcachefs scrub
  <mountpoint>` command succeeded
- writes a sentinel file, applies a `filesystems.<name>.replaceDevices` plan from the original.

writes a sentinel file, applies a `filesystems.<name>.replaceDevices` plan from the
original loop device to a second loop device, verifies the rendered `bcachefs device
add`, `bcachefs data rereplicate`, and `bcachefs device remove` commands succeeded,
confirms replacement-device superblock metadata with `bcachefs show-super`, and checks
the sentinel remains readable from the mounted filesystem

- unmounts, detaches both loop devices, and removes the backing files during cleanup

This test intentionally formats and mounts only the temporary backing files it
creates. It still requires destructive opt-in because it uses real loop, mount,
and bcachefs tooling, including real member replacement.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-bcachefs-smoke.sh
```

## bcache loop-backed smoke test

The repository also includes a root-only bcache property mutation harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-bcache-smoke
```

When enabled, it:

- creates temporary backing, cache, and replacement-cache image files
- attaches all three files to disposable `/dev/loop*` devices
- loads the `bcache` kernel module and initializes a real bcache backing/cache pair
  with `make-bcache`
- finds the generated `/dev/bcache*` device for the temporary backing loop
- applies `caches.bcacheSmoke.properties."bcache.cache-mode" = "writethrough"`
- verifies the rendered `disk-nix-bcache-property` sysfs write succeeded
- checks `/sys/block/<bcache>/bcache/cache_mode` reports `writethrough`
- derives the live cache-set UUID from `/sys/block/<bcache>/bcache/cache`
- applies `caches.bcacheSmoke.removeDevices = [ "<cache-set-uuid>" ]` and verifies the
  rendered `disk-nix-bcache-detach` sysfs write succeeded
- applies `caches.bcacheFailedAttach.addDevices = [ "<invalid-cache-set-uuid>" ]`

applies `caches.bcacheFailedAttach.addDevices = [ "<invalid-cache-set-uuid>" ]` while
detached, verifies the rendered `disk-nix-bcache-attach` sysfs write fails, and checks
the failed-attach recovery report includes partial-execution metadata, retry review,
domain recovery, and roll-forward review

- applies `caches.bcacheSmoke.addDevices = [ "<cache-set-uuid>" ]`, verifies the rendered.

applies `caches.bcacheSmoke.addDevices = [ "<cache-set-uuid>" ]`, verifies the rendered
`disk-nix-bcache-attach` sysfs write succeeded, reapplies `bcache.cache-mode =
"writethrough"`, and checks the cache mode again

- applies `caches.bcacheReplacement.replaceDevices` from the original cache loop to the.

applies `caches.bcacheReplacement.replaceDevices` from the original cache loop to the
replacement cache loop with the live `cacheSetUuid`, verifies the rendered
`disk-nix-bcache-replace` wrapper succeeded, and confirms the generated bcache device
remains readable after replacement

- executes `caches.bcacheSmoke.operation = "rescan"` against the same generated bcache
  device
- verifies the read-only rescan ran `disk-nix inspect <bcache>` and
  `disk-nix-bcache-read` checks for `state`, `cache_mode`, and `dirty_data`
- stops the generated bcache device, detaches the loops, and removes the backing files
  during cleanup

This test intentionally writes bcache metadata only to the temporary backing
files it creates. It is VM-callable through `DISK_NIX_VM_HARNESSES=bcache`, but
it is not in the default VM suite because bcache kernel support varies by
runner.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-bcache-smoke.sh
```

## LUKS loop-backed smoke test

The repository also includes a root-only LUKS loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-luks-smoke
```

When enabled, it:

- creates a temporary 64 MiB backing file and temporary keyfile
- attaches the file to the next available `/dev/loop*`
- formats the temporary loop device as a LUKS container
- opens it as a temporary `/dev/mapper/*` mapping
- verifies `disk-nix inspect <mapper> --json` sees the mapping
- applies `luks.devices.luksSmokeLabel.properties.label = "disknix-luks"` against the
  real LUKS backing device
- verifies the generated JSON report was written, the rendered `cryptsetup config <loop>.

verifies the generated JSON report was written, the rendered `cryptsetup config <loop>
--label disknix-luks` command succeeded, and `cryptsetup luksDump <loop>` reports the
new header label

- executes a `luks.devices.<name>.operation = "close"` apply plan with `allowOffline =
  true`
- verifies the generated JSON report was written and the rendered `cryptsetup close
  <mapper>` command succeeded
- detaches the loop device and removes the backing file and key material during cleanup

This test intentionally formats only the temporary backing file it creates. It
still requires destructive opt-in because it uses real loop, device-mapper, and
LUKS tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-luks-smoke.sh
```

## Swap loop-backed smoke test

The repository also includes a root-only swap loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-swap-smoke
```

When enabled, it:

- creates a temporary 64 MiB backing file
- attaches the file to the next available `/dev/loop*`
- formats the temporary loop device with a swap signature and initial label
- verifies `disk-nix inspect <loop> --json` sees the swap metadata
- applies `swaps.swapSmokeLabel.properties.label = "disknix-swap"` against the real
  loop-backed swap signature
- verifies the generated JSON report was written, the rendered `swaplabel --label disknix-swap.

verifies the generated JSON report was written, the rendered `swaplabel --label
disknix-swap <loop>` command succeeded, and `blkid -s LABEL -o value <loop>` reports
the new label

- detaches the loop device and removes the backing file during cleanup

This test intentionally formats only the temporary backing file it creates. It
still requires destructive opt-in because it uses real loop and swap-signature
tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-swap-smoke.sh
```

## zram property reconciliation smoke test

The repository also includes a root-only zram property reconciliation harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-zram-smoke
```

When enabled, it:

- applies `zram.properties.algorithm` and `zram.properties.priority` declarations
- verifies `zram:set-property:algorithm` and `zram:set-property:priority` render only
  read-only inventory commands
- verifies the rendered `zramctl --bytes --raw --noheadings --output-all`, `swapon
  --show --bytes --raw`, and `disk-nix zram` commands succeeded
- verifies the command-plan notes direct operators to `services.disk-nix.zram` and
  NixOS `zramSwap` reconciliation
- writes and compares the generated JSON report

This harness intentionally does not recreate active `/dev/zram*` devices.
Changing live zram algorithm, size, priority, or writeback settings is modeled
as generator reconciliation through NixOS service options because active zram
swap may require coordinated `swapoff` and device recreation.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-zram-smoke.sh
```

## LVM loop-backed smoke test

The repository also includes a root-only LVM loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-lvm-smoke
```

When enabled, it:

- creates a temporary 512 MiB backing file
- attaches it to the next available `/dev/loop*`
- creates a temporary LVM physical volume, volume group, logical volume, thin pool,
  thin volume, snapshot, cache pool, and cached origin volume
- formats the cached origin as ext4, mounts it, and writes a sentinel file
- verifies `disk-nix inspect <vg> --json` sees the volume group
- applies `lvmCaches.<vg/lv>.properties.lvm.cache-mode = "writethrough"` against the
  real cached origin logical volume
- verifies the generated JSON report was written, the rendered `lvchange --cachemode.

verifies the generated JSON report was written, the rendered `lvchange --cachemode
writethrough <vg/lv>` command succeeded, and `lvs -o cache_mode <vg/lv>` reports
`writethrough`

- applies `lvmCaches.<vg/lv>.removeDevices = [ "<vg/cachepool>" ]`, verifies `lvconvert
  --uncache <vg/lv>` succeeds, and checks the origin is no longer cached
- applies `lvmCaches.<vg/lv>.addDevices = [ "<vg/cachepool>" ]`, verifies `lvconvert --type.

applies `lvmCaches.<vg/lv>.addDevices = [ "<vg/cachepool>" ]`, verifies `lvconvert
--type cache --cachepool <vg/cachepool> <vg/lv>` succeeds, and checks the cache mode is
restored to `writethrough`

- creates a replacement cache pool, applies `lvmCaches.<vg/lv>.replaceDevices = {.

creates a replacement cache pool, applies `lvmCaches.<vg/lv>.replaceDevices = {
"<vg/cachepool>" = "<vg/cachepool_replacement>"; }`, verifies the rendered
`disk-nix-lvm-cache-replace` wrapper runs `lvconvert --uncache <vg/lv>` before
attaching the replacement cache pool with `lvconvert --type cache --cachepool`, and
checks the sentinel again after replacement

- verifies the cached-origin ext4 cache sentinel survives the cache-mode mutation,
  cache detach, cache reattach, cache replacement, and LVM rescan plans
- executes `volumeGroups.<name>.operation = "rescan"`, `volumes.<vg/lv>.operation = "rescan"`

executes `volumeGroups.<name>.operation = "rescan"`, `volumes.<vg/lv>.operation =
"rescan"`, `thinPools.<vg/pool>.operation = "rescan"`, and
`lvmSnapshots.<vg/snapshot>.operation = "rescan"` apply plans

- verifies the generated JSON report was written and the rendered `pvscan --cache`,
  `vgscan`, `vgchange --refresh <vg>`, and LVM `lvs` inventory commands succeeded
- unmounts the cached origin, removes the temporary volume group, wipes the physical
  volume metadata, detaches the loop device, and removes the backing file during
  cleanup

This test intentionally writes LVM metadata only to the temporary backing file
it creates. It still requires destructive opt-in because it uses real loop and
LVM tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-lvm-smoke.sh
```

## MD RAID loop-backed smoke test

The repository also includes a root-only MD RAID loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-mdraid-smoke
```

When enabled, it:

- creates three temporary 64 MiB backing files
- attaches them to the next available `/dev/loop*` devices
- creates a temporary RAID1 MD array with `mdadm`
- verifies `disk-nix inspect <array> --json` sees the array
- executes an `mdRaids.<name>.operation = "rescan"` apply plan
- verifies the generated JSON report was written and the rendered `mdadm --detail`, `mdadm.

verifies the generated JSON report was written and the rendered `mdadm --detail`,
`mdadm --detail --scan`, `mdadm --examine --scan`, and `/proc/mdstat` inventory
commands succeeded

- applies an `mdRaids.<name>.replaceDevices` plan, verifies `mdadm <array> --replace.

applies an `mdRaids.<name>.replaceDevices` plan, verifies `mdadm <array> --replace
<old-loop> --with <new-loop>` succeeds, waits for replacement completion with `mdadm
--wait <array>`, and verifies the replacement member appears in `mdadm --detail`

- fails and removes one RAID1 member from the temporary array, using the replacement
  member to prove the degraded path after replacement
- verifies stale member metadata remains inspectable with `mdadm --examine
  <removed-loop>`
- verifies `disk-nix inspect <array> --json` still sees the degraded array and the
  degraded rescan apply succeeds
- applies an `mdRaids.<name>.removeDevices` plan for the already-removed member, verifies the.

applies an `mdRaids.<name>.removeDevices` plan for the already-removed member, verifies
the real `mdadm` command fails, and checks the failed-detach recovery report includes
partial-execution metadata, retry review, domain recovery, and roll-forward review

- applies an `mdRaids.<name>.addDevices` plan for a missing member path, verifies the real.

applies an `mdRaids.<name>.addDevices` plan for a missing member path, verifies the
real `mdadm <array> --add <missing-path>` command fails, and checks the failed-reattach
recovery report includes partial-execution metadata, retry review, domain recovery, and
roll-forward review

- bounds MD rebuild progress through sysfs `sync_max`, applies an `mdRaids.<name>.addDevices`

bounds MD rebuild progress through sysfs `sync_max`, applies an
`mdRaids.<name>.addDevices` plan for the stale removed member, verifies the real `mdadm
<array> --add <stale-loop>` command succeeds while rebuild progress is partial,
restores the rebuild limit, waits with `mdadm --wait <array>`, and verifies the member
returns to the array

- stops the array, wipes member superblocks, detaches the loop devices, and removes
  backing files during cleanup

This test intentionally writes MD RAID metadata only to the temporary backing
files it creates. It still requires destructive opt-in because it uses real
loop and MD RAID tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-mdraid-smoke.sh
```

## ZFS loop-backed smoke test

The repository also includes a root-only ZFS loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-zfs-smoke
```

When enabled, it:

- creates a temporary 512 MiB backing file
- attaches it to the next available `/dev/loop*`
- creates a temporary ZFS pool mounted in the temporary directory
- verifies `disk-nix inspect <pool> --json` sees ZFS pool topology
- applies `pools.<name>.properties.autotrim = "on"` against the real loop-backed ZFS
  pool
- verifies the generated JSON report was written, the rendered `zpool set autotrim=on
  <pool>` command succeeded, and `zpool get -H -o value autotrim <pool>` reports `on`
- executes a `pools.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered `zpool scrub <pool>`
  command succeeded
- applies a `pools.<name>.replaceDevices` plan from the original loop vdev to a second loop.

applies a `pools.<name>.replaceDevices` plan from the original loop vdev to a second
loop vdev, verifies the rendered `zpool replace <pool> <old-loop> <new-loop>` command
succeeded, confirms the replacement vdev appears in `zpool status -P`, and checks the
mountpoint still remains active

- destroys the temporary pool, detaches both loop devices, and removes the backing
  files during cleanup

This test intentionally writes ZFS pool labels only to the temporary backing
files it creates. It still requires destructive opt-in because it uses real
loop and ZFS tooling, including a real pool-device replacement. The host or
guest must already have working ZFS kernel support; on NixOS this usually also
means a configured `networking.hostId`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-zfs-smoke.sh
```

## NFS client smoke test

The repository also includes a root-only NFS client harness for lab exports:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NFS_SOURCE=server.example.com:/srv/disk-nix-smoke \
  nix run .#integration-nfs-smoke
```

When enabled, it:

- creates a temporary mountpoint
- mounts the NFS source from `DISK_NIX_NFS_SOURCE`
- verifies `disk-nix inspect <mountpoint> --json` sees NFS topology
- executes an `nfs.mounts.<mountpoint>.operation = "rescan"` apply plan
- verifies the rendered `findmnt --json <mountpoint>` and `nfsstat -m <mountpoint>`
  commands succeeded
- executes an `nfs.mounts.<mountpoint>.operation = "remount"` apply plan
- verifies the rendered `mount -o remount,<options> <mountpoint>` command succeeded
- when `DISK_NIX_NFS_DATA_SURVIVAL=1` is set, writes `disk-nix-nfs-sentinel.txt` to the.

when `DISK_NIX_NFS_DATA_SURVIVAL=1` is set, writes `disk-nix-nfs-sentinel.txt` to the
mounted export, injects a failed remount apply, verifies the partial-execution recovery
report includes `resume-after-fix`, verifies the sentinel remains readable, reruns a
clean remount apply, and verifies the sentinel remains readable after the resumed apply

- when `DISK_NIX_NFS_EXPORT_PROPERTY=1` is set, creates a temporary local export path, applies.

when `DISK_NIX_NFS_EXPORT_PROPERTY=1` is set, creates a temporary local export path,
applies `exports.<path>.properties.options`, verifies the rendered `exportfs -i -o
<options> <client>:<path>` command succeeded, and checks `exportfs -v` lists the
temporary export

- unmounts the temporary client mount during cleanup

This test does not provision an NFS server or export. It requires a disposable export provided by the operator because server behavior, export policy, network reachability, NFS version, and authentication vary by lab.

The default filesystem type is `nfs4`, the default mount options are `vers=4.2`, and the default remount options reuse the mount options.

Override them with `DISK_NIX_NFS_FSTYPE`, `DISK_NIX_NFS_MOUNT_OPTIONS`, and `DISK_NIX_NFS_REMOUNT_OPTIONS`. For server-side export option testing, set `DISK_NIX_NFS_EXPORT_PROPERTY=1`; the harness exports a temporary directory to `DISK_NIX_NFS_EXPORT_CLIENT` with `DISK_NIX_NFS_EXPORT_OPTIONS`, then unexports it during cleanup.

The defaults are `127.0.0.1` and `ro,sync,no_subtree_check`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NFS_SOURCE=server.example.com:/srv/disk-nix-smoke \
  DISK_NIX_NFS_EXPORT_PROPERTY=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-nfs-smoke.sh
```

## VDO smoke test

The repository also includes a root-only VDO harness for existing lab volumes:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VDO_NAME=archive \
  nix run .#integration-vdo-smoke
```

When enabled, it:

- verifies `vdo status --name <name>` can read the selected VDO volume
- verifies `vdostats --human-readable <name>` can read runtime counters
- verifies `disk-nix inspect <name> --json` sees VDO topology
- applies `vdoVolumes.<name>.properties.writePolicy` against the selected disposable
  VDO volume
- verifies the generated JSON report was written, the rendered `vdo changeWritePolicy --name.

verifies the generated JSON report was written, the rendered `vdo changeWritePolicy
--name <name> --writePolicy <policy>` command succeeded, and `vdo status --name <name>`
reports the requested write policy

- executes a `vdoVolumes.<name>.operation = "rescan"` apply plan
- verifies the rendered `vdo status --name <name>`, `vdostats --human-readable <name>`,
  and `disk-nix inspect <name>` commands succeeded
- verifies the generated JSON report was written

This test does not create, grow, start, stop, or remove a VDO volume. It still requires destructive opt-in because it reads real VDO management state and changes the selected volume's write policy.

It is intended for disposable lab hosts where the named volume can be safely probed and mutated. The default write policy is `sync`;

override it with `DISK_NIX_VDO_WRITE_POLICY=auto`, `sync`, or `async`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VDO_NAME=archive \
  DISK_NIX_VDO_WRITE_POLICY=sync \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-vdo-smoke.sh
```

## iSCSI session smoke test

The repository also includes a root-only iSCSI harness for existing lab
sessions:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root \
  DISK_NIX_LUN_PATH=/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage.root-lun-0 \
  nix run .#integration-iscsi-smoke
```

When enabled, it:

- verifies `iscsiadm --mode session` reports the selected target
- verifies `lsscsi -t -s` can read host-visible transport inventory
- verifies `disk-nix inspect <target> --json` sees iSCSI topology
- executes an `iscsiSessions.<target>.operation = "rescan"` apply plan
- verifies the rendered `iscsiadm --mode session --rescan`, `lsscsi -t -s`, and
  `disk-nix inspect <target> --json` commands succeeded
- when `DISK_NIX_LUN_PATH` is set, executes `luns.<target>:0.operation = "rescan"` for
  that host-visible path
- verifies the rendered host-side `disk-nix-scsi-rescan` handoff and `multipath -r`
  commands succeeded for the selected LUN path
- when `DISK_NIX_LUN_DATA_SURVIVAL=1` and `DISK_NIX_LUN_MOUNTPOINT` are set, writes.

when `DISK_NIX_LUN_DATA_SURVIVAL=1` and `DISK_NIX_LUN_MOUNTPOINT` are set, writes
`disk-nix-iscsi-lun-sentinel.txt` to an already-mounted filesystem on that LUN, injects
a failed host-side LUN rescan, verifies the partial-execution recovery report includes
`resume-after-fix`, verifies the sentinel remains readable, reruns a clean LUN rescan
apply, and verifies the sentinel remains readable after the resumed operation

- verifies the generated JSON report was written

This test does not discover, log in to, log out from, grow, attach, detach, or remove an iSCSI target or LUN.

It still requires destructive opt-in because it performs a real session rescan and, when `DISK_NIX_LUN_PATH` is set, a real host-side LUN rescan.

It is intended for disposable lab hosts where the named session and optional LUN path can be safely refreshed.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root \
  DISK_NIX_LUN_PATH=/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage.root-lun-0 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-iscsi-smoke.sh
```

## Multipath map smoke test

The repository also includes a root-only multipath harness for existing lab
maps:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_MULTIPATH_MAP=mpatha \
  DISK_NIX_MULTIPATH_RESIZE=1 \
  DISK_NIX_MULTIPATH_ADD_PATH=/dev/sdb \
  DISK_NIX_MULTIPATH_REMOVE_PATH=/dev/sdc \
  DISK_NIX_MULTIPATH_REPLACE_OLD_PATH=/dev/sde \
  DISK_NIX_MULTIPATH_REPLACE_NEW_PATH=/dev/sdf \
  DISK_NIX_MULTIPATH_FLUSH=1 \
  nix run .#integration-multipath-smoke
```

When enabled, it:

- verifies `multipath -ll <map>` can read the selected map
- verifies `lsscsi -t -s` can read host-visible transport inventory
- verifies `disk-nix inspect <map> --json` sees multipath topology
- executes a `multipathMaps.inventory.operation = "rescan"` apply plan with `target =
  <map>`
- verifies the rendered `multipath -ll <map>`, `lsscsi -t -s`, and `multipath -r`
  commands succeeded
- when `DISK_NIX_MULTIPATH_RESIZE=1` is set, executes `multipathMaps.resize.operation =
  "grow"` with `target = <map>`
- verifies the rendered `multipathd resize map <map>` and follow-up `multipath -r`
  commands succeeded
- when `DISK_NIX_MULTIPATH_ADD_PATH` or `DISK_NIX_MULTIPATH_REMOVE_PATH` is set, executes.

when `DISK_NIX_MULTIPATH_ADD_PATH` or `DISK_NIX_MULTIPATH_REMOVE_PATH` is set, executes
`multipathMaps.paths.addDevices` and/or `multipathMaps.paths.removeDevices` for the
explicitly named paths

- verifies the rendered `multipathd add path <path>` and `multipathd del path <path>`
  commands succeeded for those selected paths
- when `DISK_NIX_MULTIPATH_REPLACE_OLD_PATH` and `DISK_NIX_MULTIPATH_REPLACE_NEW_PATH`
  are set, executes `multipathMaps.paths.replaceDevices` for the explicit path pair
- verifies the rendered `multipathd add path <new-path>` command succeeds before
  `multipathd del path <old-path>` succeeds
- when `DISK_NIX_MULTIPATH_FLUSH=1` is set, executes `multipathMaps.flush.destroy =
  true` with `allowDestructive = true` and `backupVerified = true`
- verifies the rendered `multipath -f <map>` command succeeded
- verifies the generated JSON report was written

This test requires destructive opt-in because `multipath -r` reloads live maps,
`DISK_NIX_MULTIPATH_RESIZE=1` asks multipathd to resize the selected map, and
the add/remove/replace/flush variables mutate explicitly selected paths or
maps. It is intended for disposable lab hosts where the named map and paths can
be safely refreshed, replaced, or removed. Use an `mpath*` name such as
`mpatha` or a `/dev/mapper/*` path.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_MULTIPATH_MAP=mpatha \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-multipath-smoke.sh
```

## NVMe namespace smoke test

The repository also includes a root-only NVMe harness for existing lab
controllers:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NVME_CONTROLLER=/dev/nvme0 \
  DISK_NIX_NVME_RECONNECT=1 \
  DISK_NIX_NVME_RECONNECT_NQN=nqn.2014-08.org.nvmexpress.discovery \
  DISK_NIX_NVME_RECONNECT_TRANSPORT=tcp \
  DISK_NIX_NVME_RECONNECT_TRADDR=192.0.2.10 \
  DISK_NIX_NVME_RECONNECT_TRSVCID=4420 \
  DISK_NIX_NVME_RECONNECT_CONTROLLER=/dev/nvme0 \
  nix run .#integration-nvme-smoke
```

When enabled, it:

- verifies `nvme list-ns <controller> --all --output-format=json` can read namespace
  inventory
- verifies `nvme list-subsys --output-format=json` can read subsystem paths
- verifies `disk-nix inspect <controller> --json` sees NVMe topology
- executes an `nvmeNamespaces.<controller>.operation = "rescan"` apply plan
- verifies the rendered `nvme list-ns`, `nvme list-subsys`, and `nvme ns-rescan
  <controller>` commands succeeded
- when `DISK_NIX_NVME_CREATE_DELETE=1` is set.

when `DISK_NIX_NVME_CREATE_DELETE=1` is set with `DISK_NIX_NVME_NAMESPACE_ID`,
`DISK_NIX_NVME_NAMESPACE_SIZE`, and `DISK_NIX_NVME_CONTROLLERS`, applies an
`nvmeNamespaces.<controller>.operation = "create"` plan, verifies `nvme create-ns
<controller> --nsze-si <size> --ncap-si <size>`, `nvme attach-ns <controller>
--namespace-id <id> --controllers <ids>`, and namespace rescan succeed, then applies a
destructive cleanup plan and verifies `nvme detach-ns <controller> --namespace-id <id>
--controllers <ids>`, `nvme delete-ns <controller> --namespace-id <id>`, and final
namespace inventory succeed

- verifies namespace identity drift for that create/delete path by checking `nvme list-ns.

verifies namespace identity drift for that create/delete path by checking `nvme list-ns
<controller> --all --output-format=json` contains the selected namespace id after
create and no longer contains it after delete

- when `DISK_NIX_NVME_GROW=1` is set, applies an `nvmeNamespaces.<controller>.operation =.

when `DISK_NIX_NVME_GROW=1` is set, applies an `nvmeNamespaces.<controller>.operation =
"grow"` plan and verifies the rendered `nvme list-subsys` and `nvme ns-rescan
<controller>` commands succeeded under the reviewed grow policy

- when `DISK_NIX_NVME_ATTACH_DETACH=1` is set.

when `DISK_NIX_NVME_ATTACH_DETACH=1` is set with `DISK_NIX_NVME_NAMESPACE_ID` and
`DISK_NIX_NVME_CONTROLLERS`, applies an `nvmeNamespaces.<controller>.operation =
"attach"` plan, verifies `nvme attach-ns <controller> --namespace-id <id> --controllers
<ids>` and `nvme ns-rescan <controller>` succeed, then applies a matching detach plan
and verifies `nvme detach-ns <controller> --namespace-id <id> --controllers <ids>` plus
a final namespace rescan succeed

- when `DISK_NIX_NVME_RECONNECT=1` is set.

when `DISK_NIX_NVME_RECONNECT=1` is set with `DISK_NIX_NVME_RECONNECT_NQN`,
`DISK_NIX_NVME_RECONNECT_TRANSPORT`, `DISK_NIX_NVME_RECONNECT_TRADDR`, optional
`DISK_NIX_NVME_RECONNECT_TRSVCID`, and `DISK_NIX_NVME_RECONNECT_CONTROLLER`,
disconnects the reviewed NQN with `nvme disconnect`, reconnects it with `nvme connect`,
waits for the expected controller path, verifies `disk-nix inspect <controller> --json`
sees the reconnected controller, and reruns the namespace rescan apply

- verifies the generated JSON report was written

By default this test does not create, grow, attach, detach, or delete NVMe namespaces. The create/delete and attach/detach modes are deliberately opt-in and require a disposable namespace that can safely end deleted or detached from the selected controller.

The harness still requires destructive opt-in because `nvme ns-rescan` refreshes live controller namespace state and namespace lifecycle changes visibility or allocation.

Use a controller path such as `/dev/nvme0`, not a namespace path such as `/dev/nvme0n1`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NVME_CONTROLLER=/dev/nvme0 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-nvme-smoke.sh
```

## Target-side LUN property smoke test

The repository also includes a root-only LIO target-side LUN property harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-target-lun-smoke
```

When enabled, it:

- creates a temporary 64 MiB backing file and attaches it to a disposable `/dev/loop*`
- creates a temporary LIO block backstore with `targetcli /backstores/block create`
- creates a temporary iSCSI target and maps the first backstore as a LUN
- applies `targetLuns.<iqn>.properties."lio.writeCache" = "off"`
- verifies the rendered `targetcli /backstores/block/<name> set attribute
  emulate_write_cache=0` command succeeded
- creates a second temporary backstore and applies `targetLuns.<iqn>.operation =
  "attach"` to map it as another LUN with a reviewed initiator ACL
- formats the second loop-backed LUN as ext4, writes `disk-nix-target-lun-sentinel.txt`

formats the second loop-backed LUN as ext4, writes `disk-nix-target-lun-sentinel.txt`,
injects a failed target-side detach apply before target state is mutated, verifies the
partial-execution recovery report includes `resume-after-fix` and domain recovery
guidance, and verifies the sentinel remains readable

- applies `targetLuns.<iqn>.operation = "detach"` to remove that initiator ACL and unmap the.

applies `targetLuns.<iqn>.operation = "detach"` to remove that initiator ACL and unmap
the second LUN without deleting the backstore, then verifies the sentinel remains
readable after the resumed detach operation

- verifies `targetLuns.<iqn>.destroy = true` is refused without `allowDestructive = true`

verifies `targetLuns.<iqn>.destroy = true` is refused without `allowDestructive =
true`, leaves the command plan empty, and reports non-destructive review-policy
guidance

- removes the temporary target, backstores, loop devices, and backing files during
  cleanup

This test intentionally mutates only the temporary LIO target state and
loop-backed block device it creates. It is VM-callable with
`DISK_NIX_VM_HARNESSES=target-lun`, but it is not part of the default VM suite
because LIO kernel target support varies by runner.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-target-lun-smoke.sh
```

## Layered VM smoke test

The repository includes a root-only layered harness intended for disposable VMs:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-layered-vm-smoke
```

When enabled, it:

- creates a temporary partitioned loop-backed disk image
- formats and opens a LUKS mapper on the loop partition
- creates an LVM PV, VG, and root LV on the mapper
- creates and mounts an ext4 filesystem on the LV
- verifies `disk-nix inspect <mountpoint> --json` sees the layered topology
- writes a sentinel file, grows the loop backing file, and executes one multi-domain apply.

writes a sentinel file, grows the loop backing file, and executes one multi-domain
apply plan for `partitions.layeredPart`, `luks.devices.layeredMapper`,
`volumes.layeredRoot`, `filesystems.layeredRoot`, and `filesystems.layeredRootRemount`

- verifies the rendered and executed `growpart <loop> 1`, `cryptsetup resize <mapper>`

verifies the rendered and executed `growpart <loop> 1`, `cryptsetup resize <mapper>`,
`lvextend --resizefs --size 192M <lv>`, `resize2fs <lv>`, and `mount -o
remount,rw,noatime <mountpoint>` commands succeeded and the JSON report was written

- verifies the LV grew, the remount option is active, the sentinel survived, and the
  mounted filesystem remains inspectable after the grow
- executes a VM-backed failure-injection apply where real `lvextend --resizefs --size
  256M <lv>` succeeds and real `xfs_growfs <mountpoint>` fails against the ext4 mount
- verifies the failed report records `partialExecutionRecovery`

verifies the failed report records `partialExecutionRecovery` with the completed LV
grow action, failed filesystem action, failed command, remaining remount action,
completed mutating command count, fresh-topology review notes, and domain,
roll-forward, rollback, and recovery-point preservation actions

- verifies rollback review stays non-mutating: rollback precondition commands are read-only.

verifies rollback review stays non-mutating: rollback precondition commands are
read-only, the rollback recipe is `refused`, reversible and destructive mutation
sections are empty, required topology evidence is listed, and operator-only guidance is
emitted instead of an automated unsafe rollback

- resumes with a clean follow-up apply for the remaining remount action, verifies `mount -o.

resumes with a clean follow-up apply for the remaining remount action, verifies `mount
-o remount,rw,relatime <mountpoint>` succeeds, and confirms the sentinel remains
readable after the failed-and-resumed apply sequence

- verifies the failed apply report was written, the LV growth before the failure is
  visible, and sentinel data still survives after the failed apply
- unmounts the filesystem, deactivates the VG, executes a `luks.devices.layeredMapper` close.

unmounts the filesystem, deactivates the VG, executes a `luks.devices.layeredMapper`
close apply plan, and verifies the rendered `cryptsetup close <mapper>` command
succeeded

- reopens the LUKS mapper with the temporary key, reactivates the VG, remounts the LV,
  verifies the sentinel survived, and inspects the reopened layered topology

The harness removes the mount, VG, mapper, loop device, backing file, and key material during cleanup. It is included in the default VM smoke suite alongside the loop, Btrfs, and synthetic failure-recovery harnesses, but it is not run by `nix flake check` because it mutates real kernel block-device state.

The LUKS, LVM, MD RAID, bcachefs, ZFS, NFS, VDO, iSCSI, multipath, and NVMe harnesses remain packaged and VM-callable through `DISK_NIX_VM_HARNESSES`;

bcachefs is not part of the default VM list because some NixOS test kernels do not expose the `bcachefs` filesystem module even when `bcachefs-tools` is available.

