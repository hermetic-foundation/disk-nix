# Integration tests

Unit tests and flake checks cover parsers, planning, command rendering, NixOS
module evaluation, examples, schema generation, completions, and manpage output.
Real storage mutation needs additional host-backed tests because Nix build
sandboxes cannot safely create privileged block devices.

## VM destructive suite

The preferred destructive workflow is to run the smoke harnesses inside a
disposable virtual machine. The flake exposes an opt-in NixOS VM test that
boots a guest and runs the suite inside it:

```sh
nix build .#integration-vm-test
```

This derivation is intentionally not part of default `nix flake check`; it runs
QEMU and performs real storage mutations inside the guest.

If you already have a disposable VM or lab guest, run the in-guest suite
directly:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-vm-smoke
```

The VM suite refuses to run unless:

- `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set
- it is running as root
- `systemd-detect-virt --vm` detects a virtual machine

For controlled lab automation where VM detection is unavailable but isolation
is provided externally, set `DISK_NIX_INTEGRATION_ASSUME_VM=1`.

By default the suite runs the loop, Btrfs, bcachefs, LUKS, LVM, MD RAID, and
failure-recovery smoke harnesses. To run a subset:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VM_HARNESSES="loop btrfs" \
  nix run .#integration-vm-smoke
```

The individual harnesses below remain available for targeted lab debugging,
but they should still be treated as destructive host operations.

The ZFS harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=zfs` in a disposable guest that has working ZFS
kernel support and a configured host ID. It is not part of the default VM suite
until the flake VM test provisions that kernel support reliably.

The NFS client harness is also packaged with the VM suite and can be selected
explicitly with `DISK_NIX_VM_HARNESSES=nfs` when the guest can reach a
disposable export supplied through `DISK_NIX_NFS_SOURCE`. It is not part of the
default VM suite because the flake VM test does not yet provision a server
export.

The VDO harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=vdo` when the guest has an existing disposable VDO
volume named by `DISK_NIX_VDO_NAME`. It is not part of the default VM suite
because the flake VM test does not yet provision a VDO volume.

The iSCSI harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=iscsi` when the guest has an existing disposable
iSCSI session for the target named by `DISK_NIX_ISCSI_TARGET`. It is not part
of the default VM suite because the flake VM test does not yet provision an
iSCSI target.

The multipath harness is packaged with the VM suite and can be selected
explicitly with `DISK_NIX_VM_HARNESSES=multipath` when the guest has an
existing disposable multipath map named by `DISK_NIX_MULTIPATH_MAP`. It is not
part of the default VM suite because the flake VM test does not yet provision
multiple backing paths for a map.

The NVMe harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=nvme` when the guest has an existing disposable
controller path named by `DISK_NIX_NVME_CONTROLLER`. It is not part of the
default VM suite because the flake VM test does not yet provision an NVMe
controller.

## Failure-recovery smoke test

The repository includes a synthetic failed-apply harness:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-failure-recovery-smoke
```

The harness refuses to run unless `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set,
matching the execute-mode integration guard used by the destructive harnesses.
It does not require root and does not mutate real storage. Instead, it uses
fake storage tools ahead of `PATH` for fifteen failed apply paths:

- a layered LVM volume grow followed by an ext4 filesystem grow where fake
  `lvextend` succeeds and fake `resize2fs` fails
- a ZFS snapshot rollback where fake `zfs list` succeeds and fake
  `zfs rollback tank/home@before` fails
- an NVMe namespace create where fake `nvme list-ns` succeeds and fake
  `nvme create-ns /dev/nvme0 --nsze-si 100G --ncap-si 100G` fails
- an NVMe namespace grow where fake namespace and subsystem inventory succeeds
  and fake `nvme ns-rescan /dev/nvme1` fails
- an NVMe namespace attach where fake namespace and subsystem inventory
  succeeds and fake `nvme attach-ns /dev/nvme2 --namespace-id 7 --controllers 0x2` fails
- an NVMe namespace detach where fake namespace and subsystem inventory
  succeeds and fake `nvme detach-ns /dev/nvme3 --namespace-id 8 --controllers 0x3` fails
- an NVMe namespace destroy where fake `nvme detach-ns` succeeds and fake
  `nvme delete-ns /dev/nvme4 --namespace-id 9` fails
- a target-side LUN create through the Linux LIO provider where fake
  `targetcli` inventory, backstore creation, and target creation succeed and
  fake `targetcli /iscsi/iqn.2026-06.example:storage.root/tpg1/luns create /backstores/block/_dev_zvol_tank_root lun=7`
  fails
- a multipath path replacement where fake `multipath -ll /dev/mapper/mpatha`
  and fake `multipathd add path /dev/sdd` succeed and fake
  `multipathd del path /dev/sdc` fails
- an iSCSI session logout where fake `iscsiadm --logout` fails for the reviewed
  target and portal
- an iSCSI session login where fake `iscsiadm --mode discovery` succeeds and
  fake `iscsiadm --login` fails for the reviewed target and portal
- an LVM cache attach where fake `lvconvert --type cache` fails for the
  reviewed origin LV and cache pool
- an LVM cache detach where fake `lvs` succeeds and fake
  `lvconvert --uncache` fails for the reviewed origin LV
- a VDO property mutation where fake `disk-nix inspect archive` succeeds and
  fake `vdo changeWritePolicy --name archive --writePolicy sync` fails
- an LVM cache property mutation where fake `lvchange --cachemode` fails for a
  reviewed origin LV

The test verifies that the failed report and receipt preserve:

- `partialExecutionRecovery.completedActionIds` containing
  `volumes:vg0/root:grow`
- `partialExecutionRecovery.failedActionId` as `filesystem:root:grow`
- the failed `resize2fs vg0/root 50GiB` command and non-zero status
- one completed mutating command before failure
- `partialExecutionRecovery.failedActionId` as
  `snapshot:tank/home@before:rollback`
- the failed `zfs rollback tank/home@before` command and non-zero status
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme0:create`
- the failed `nvme create-ns /dev/nvme0 --nsze-si 100G --ncap-si 100G` command
  and non-zero status after namespace inventory inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme1:grow`
- the failed `nvme ns-rescan /dev/nvme1` command and non-zero status after
  namespace and subsystem inventory inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme2:attach`
- the failed `nvme attach-ns /dev/nvme2 --namespace-id 7 --controllers 0x2`
  command and non-zero status after namespace and subsystem inventory
  inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme3:detach`
- the failed `nvme detach-ns /dev/nvme3 --namespace-id 8 --controllers 0x3`
  command and non-zero status after namespace and subsystem inventory
  inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme4:destroy`
- the failed `nvme delete-ns /dev/nvme4 --namespace-id 9` command and
  non-zero status
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:storage.root:create`
- the failed LIO `targetcli .../tpg1/luns create` command and non-zero status
  after target-side inventory, backstore creation, and target creation
- `partialExecutionRecovery.failedActionId` as
  `multipathMaps:root-map:replace-device:/dev/sdc`
- the failed `multipathd del path /dev/sdc` command and non-zero status after
  successful multipath map inspection and replacement-path add
- `partialExecutionRecovery.failedActionId` as
  `iscsisessions:iqn.2026-06.example:storage.old:logout`
- the failed `iscsiadm --mode node --targetname iqn.2026-06.example:storage.old --portal 192.0.2.11:3260 --logout` command
  and non-zero status
- `partialExecutionRecovery.failedActionId` as
  `iscsisessions:iqn.2026-06.example:storage.root:login`
- the failed `iscsiadm --mode node --targetname iqn.2026-06.example:storage.root --portal 192.0.2.10:3260 --login` command
  and non-zero status after successful discovery
- `partialExecutionRecovery.failedActionId` as
  `lvmCaches:vg0/root:add-device:vg0/root-cache`
- the failed `lvconvert --type cache --cachepool vg0/root-cache vg0/root`
  command and non-zero status
- `partialExecutionRecovery.failedActionId` as
  `lvmCaches:vg0/root:remove-device:vg0/root-cache`
- the failed `lvconvert --uncache vg0/root` command and non-zero status after
  successful cache-state inspection
- `partialExecutionRecovery.failedActionId` as
  `vdoVolumes:archive:set-property:writePolicy`
- the failed `vdo changeWritePolicy --name archive --writePolicy sync` command
  and non-zero status after target inspection
- `partialExecutionRecovery.failedActionId` as
  `lvmCaches:vg0/root:set-property:lvm.cache-mode`
- the failed `lvchange --cachemode writethrough vg0/root` command and non-zero
  status
- retry/review, roll-forward review, rollback review, snapshot-preservation,
  and domain-recovery guidance for the failed applies

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-failure-recovery-smoke.sh
```

## Loop-backed smoke test

The repository includes a root-only loop-backed smoke harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-loop-smoke
```

The harness refuses to run unless `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set.
When enabled, it:

- creates a temporary 64 MiB backing file
- attaches it to the next available `/dev/loop*`
- formats the temporary loop device with ext4
- verifies `disk-nix inspect <loop> --json` can see the real loop node
- executes a safe `loopDevices.<loop>.operation = "rescan"` apply plan
- grows the temporary backing file, refreshes the loop device capacity, and
  executes an ext4 `resizePolicy = "grow-only"` apply plan
- verifies the generated JSON report was written and all executed commands
  succeeded
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
- executes a `filesystems.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered
  `btrfs scrub start -B <mountpoint>` command succeeded
- unmounts, detaches the loop device, and removes the backing file during
  cleanup

This test intentionally formats and mounts only the temporary backing file it
creates. It still requires destructive opt-in because it uses real loop, mount,
and Btrfs tooling.

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
- verifies the generated JSON report was written and the rendered
  `bcachefs scrub <mountpoint>` command succeeded
- unmounts, detaches the loop device, and removes the backing file during
  cleanup

This test intentionally formats and mounts only the temporary backing file it
creates. It still requires destructive opt-in because it uses real loop, mount,
and bcachefs tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-bcachefs-smoke.sh
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
- executes a `luks.devices.<name>.operation = "close"` apply plan with
  `allowOffline = true`
- verifies the generated JSON report was written and the rendered
  `cryptsetup close <mapper>` command succeeded
- detaches the loop device and removes the backing file and key material during
  cleanup

This test intentionally formats only the temporary backing file it creates. It
still requires destructive opt-in because it uses real loop, device-mapper, and
LUKS tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-luks-smoke.sh
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
- creates a temporary LVM physical volume, volume group, logical volume, thin
  pool, thin volume, and snapshot
- verifies `disk-nix inspect <vg> --json` sees the volume group
- executes `volumeGroups.<name>.operation = "rescan"`,
  `volumes.<vg/lv>.operation = "rescan"`,
  `thinPools.<vg/pool>.operation = "rescan"`, and
  `lvmSnapshots.<vg/snapshot>.operation = "rescan"` apply plans
- verifies the generated JSON report was written and the rendered
  `pvscan --cache`, `vgscan`, `vgchange --refresh <vg>`, and LVM `lvs`
  inventory commands succeeded
- removes the temporary volume group, wipes the physical volume metadata,
  detaches the loop device, and removes the backing file during cleanup

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

- creates two temporary 64 MiB backing files
- attaches them to the next available `/dev/loop*` devices
- creates a temporary RAID1 MD array with `mdadm`
- verifies `disk-nix inspect <array> --json` sees the array
- executes an `mdRaids.<name>.operation = "rescan"` apply plan
- verifies the generated JSON report was written and the rendered
  `mdadm --detail`, `mdadm --detail --scan`, `mdadm --examine --scan`, and
  `/proc/mdstat` inventory commands succeeded
- stops the array, wipes member superblocks, detaches the loop devices, and
  removes backing files during cleanup

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
- executes a `pools.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered
  `zpool scrub <pool>` command succeeded
- destroys the temporary pool, detaches the loop device, and removes the
  backing file during cleanup

This test intentionally writes ZFS pool labels only to the temporary backing
file it creates. It still requires destructive opt-in because it uses real
loop and ZFS tooling. The host or guest must already have working ZFS kernel
support; on NixOS this usually also means a configured `networking.hostId`.

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
- verifies the rendered `findmnt --json <mountpoint>` and
  `nfsstat -m <mountpoint>` commands succeeded
- executes an `nfs.mounts.<mountpoint>.operation = "remount"` apply plan
- verifies the rendered `mount -o remount,<options> <mountpoint>` command
  succeeded
- unmounts the temporary client mount during cleanup

This test does not provision an NFS server or export. It requires a disposable
export provided by the operator because server behavior, export policy, network
reachability, NFS version, and authentication vary by lab. The default
filesystem type is `nfs4`, the default mount options are `vers=4.2`, and the
default remount options reuse the mount options. Override them with
`DISK_NIX_NFS_FSTYPE`, `DISK_NIX_NFS_MOUNT_OPTIONS`, and
`DISK_NIX_NFS_REMOUNT_OPTIONS`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NFS_SOURCE=server.example.com:/srv/disk-nix-smoke \
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
- executes a `vdoVolumes.<name>.operation = "rescan"` apply plan
- verifies the rendered `vdo status --name <name>`,
  `vdostats --human-readable <name>`, and `disk-nix inspect <name>` commands
  succeeded
- verifies the generated JSON report was written

This test does not create, grow, start, stop, or remove a VDO volume. It still
requires destructive opt-in because it reads real VDO management state and is
intended for disposable lab hosts where the named volume can be safely probed.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VDO_NAME=archive \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-vdo-smoke.sh
```

## iSCSI session smoke test

The repository also includes a root-only iSCSI harness for existing lab
sessions:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root \
  nix run .#integration-iscsi-smoke
```

When enabled, it:

- verifies `iscsiadm --mode session` reports the selected target
- verifies `lsscsi -t -s` can read host-visible transport inventory
- verifies `disk-nix inspect <target> --json` sees iSCSI topology
- executes an `iscsiSessions.<target>.operation = "rescan"` apply plan
- verifies the rendered `iscsiadm --mode session --rescan`,
  `lsscsi -t -s`, and `disk-nix inspect <target> --json` commands succeeded
- verifies the generated JSON report was written

This test does not discover, log in to, log out from, grow, attach, detach, or
remove an iSCSI target or LUN. It still requires destructive opt-in because it
performs a real session rescan and is intended for disposable lab hosts where
the named session can be safely refreshed.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-iscsi-smoke.sh
```

## Multipath map smoke test

The repository also includes a root-only multipath harness for existing lab
maps:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_MULTIPATH_MAP=mpatha \
  nix run .#integration-multipath-smoke
```

When enabled, it:

- verifies `multipath -ll <map>` can read the selected map
- verifies `lsscsi -t -s` can read host-visible transport inventory
- verifies `disk-nix inspect <map> --json` sees multipath topology
- executes a `multipathMaps.inventory.operation = "rescan"` apply plan with
  `target = <map>`
- verifies the rendered `multipath -ll <map>`, `lsscsi -t -s`, and
  `multipath -r` commands succeeded
- verifies the generated JSON report was written

This test does not add, remove, replace, flush, or resize multipath paths. It
still requires destructive opt-in because `multipath -r` reloads live maps and
is intended for disposable lab hosts where the named map can be safely
refreshed. Use an `mpath*` name such as `mpatha` or a `/dev/mapper/*` path.

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
  nix run .#integration-nvme-smoke
```

When enabled, it:

- verifies `nvme list-ns <controller> --all --output-format=json` can read
  namespace inventory
- verifies `nvme list-subsys --output-format=json` can read subsystem paths
- verifies `disk-nix inspect <controller> --json` sees NVMe topology
- executes an `nvmeNamespaces.<controller>.operation = "rescan"` apply plan
- verifies the rendered `nvme list-ns`, `nvme list-subsys`, and
  `nvme ns-rescan <controller>` commands succeeded
- verifies the generated JSON report was written

This test does not create, grow, attach, detach, or delete NVMe namespaces. It
still requires destructive opt-in because `nvme ns-rescan` refreshes live
controller namespace state and is intended for disposable lab hosts where the
selected controller can be safely rescanned. Use a controller path such as
`/dev/nvme0`, not a namespace path such as `/dev/nvme0n1`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NVME_CONTROLLER=/dev/nvme0 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-nvme-smoke.sh
```

## Flake coverage

`nix flake check` does not run destructive integration tests. It does validate
that the loop smoke harnesses parse, remain opt-in, and still contain the
expected loop, filesystem setup, resize, mount, Btrfs scrub, bcachefs format,
bcachefs scrub, LUKS format, LUKS open, LUKS close, LVM create, LVM rescan, MD
RAID create, MD RAID rescan, ZFS pool create, ZFS scrub, NFS mount, NFS rescan,
NFS remount, VDO status, VDO stats, VDO rescan, VM orchestration guard steps,
iSCSI session rescan, multipath map rescan, NVMe namespace rescan, and the
synthetic failed-apply `partialExecutionRecovery` assertions. This keeps the
harnesses available and packaged while preserving safe default checks.

## Remaining integration coverage

The VM smoke suite and targeted loop tests are only the first host-backed
integration paths. Feature completion still needs disposable VM or lab-host
tests for broader LUKS format/grow/keyslot/token behavior, broader LVM
LV/thin/cache/device-topology behavior, additional device replacement domains,
broader bcachefs multi-device and member-topology behavior, broader ZFS
vdev/dataset/zvol/snapshot behavior, broader MD RAID grow/member-topology
behavior, broader multipath path add/remove/flush/grow/failure behavior,
broader iSCSI LUN failure behavior, broader NFS server/export/unmount/failure
behavior, broader VDO create/grow/start/stop/property/remove behavior,
additional NVMe namespace variant failure behavior, additional cache variant
failure behavior, property mutation across more supported domains, recovery
behavior beyond the synthetic LVM-plus-filesystem, ZFS rollback, NVMe namespace
create, NVMe namespace grow, NVMe namespace attach, NVMe namespace detach, NVMe
namespace delete, target-side LUN LIO create, multipath replace, iSCSI logout,
iSCSI login, LVM cache attach, LVM cache detach, VDO property, and LVM cache
property failed-command paths, and broader destructive apply behavior.
