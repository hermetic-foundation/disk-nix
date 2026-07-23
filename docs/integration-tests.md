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

By default the suite runs the loop, Btrfs, swap, layered-VM, and
failure-recovery smoke harnesses. To run a subset:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VM_HARNESSES="loop btrfs" \
  nix run .#integration-vm-smoke
```

The individual harnesses below remain available for targeted lab debugging,
but they should still be treated as destructive host operations.

## Disko example suite

The repository includes generated disk-nix equivalents for every upstream
`nix-community/disko` file under `example/`, including the nested
stand-alone NixOS configuration example.

The generated specs live in `examples/disko/`, with source mapping and known
translation notes in `examples/disko/manifest.json`.

Run the safe dry-run gate with:

```sh
nix run .#integration-disko-examples
```

That command plans and dry-runs every generated spec. It fails if any example
has blocked policy, unresolved command inputs, manual-only commands, or
non-ready command rendering.

Run the non-root destructive-shape preflight with:

```sh
env DISK_NIX_DISKO_E2E_PREFLIGHT=1 nix run .#integration-disko-examples
```

That mode rewrites mountpoints the same way destructive execution does, renders
the command plan, and refuses host path targets outside the disposable E2E
root and expected storage device namespaces.

Destructive execution is guarded separately. It is intended only for disposable
lab disks addressed by stable `/dev/disk/by-id` identities. On the current lab
host, those stable identities correspond to the disks currently enumerated as
`/dev/sda` and `/dev/sdc` through `/dev/sdf`; `/dev/sdb` is excluded because it
is the system disk after the reboot.

```sh
sudo env DISK_NIX_DISKO_E2E_EXECUTE=1 \
  DISK_NIX_DISKO_E2E_CONFIRM=wipe-/dev/disk/by-id/wwn-0x5000c500a5a461dc-/dev/disk/by-id/wwn-0x5000c50087a102ce-/dev/disk/by-id/wwn-0x5000c50087a11cd1-/dev/disk/by-id/wwn-0x5000c500a5a40803-/dev/disk/by-id/wwn-0x5000c500a5a3ab29 \
  nix run .#integration-disko-examples
```

The destructive mode refuses to run unless all five requested disks exist, the
confirmation string matches exactly, and no selected disk or child reports a
mountpoint.

On hosts without ZFS or bcachefs kernel support, destructive execution skips the
affected generated specs after their normal dry-run and preflight coverage have
proved that every command is ready. On a host with those kernel filesystems
available, those specs execute through the same guarded path.

Set `DISK_NIX_DISKO_E2E_REQUIRE_ALL_KERNELS=1` when using the destructive suite
as a completion gate. In that mode, any ZFS or bcachefs capability skip makes
the run fail after printing the skipped examples, so a green result proves that
every generated spec executed destructively on the current host.

In destructive mode, filesystem mountpoints and Btrfs subvolume targets are
rewritten under `/mnt/disk-nix-disko-e2e/<example>/` before execution. The
harness also performs best-effort teardown of that mount tree, swaps, ZFS
pools, LVM volume groups, MD arrays, and LUKS mappings between examples.

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

## Detailed harness references

The long harness catalogs are split by purpose:

- [Integration failure recovery](integration-failure-recovery.md)
- [Integration smoke harnesses](integration-smoke-harnesses.md)

The failure-recovery reference covers the synthetic failed-command catalog,
recovery reports, rollback-review behavior, and failed-and-resumed apply proof.

The smoke-harness reference covers loop-backed, VM-backed, and lab-backed smoke
harness details.

This page remains the entrypoint for destructive-suite policy, disk requirements, and flake coverage.

## Flake coverage

`nix flake check` does not run destructive integration tests.

It does validate that smoke harnesses parse, remain opt-in, and still contain
expected coverage markers.

Those markers cover loop setup, filesystem resize/mount, Btrfs scrub, bcachefs
format/scrub, LUKS format/open/close, LVM create/rescan, MD RAID create/rescan,
ZFS pool create/scrub, NFS mount/rescan/remount/export/unexport, VDO
status/stats/rescan, iSCSI session rescan, multipath map rescan, NVMe namespace
rescan, VM orchestration guards, layered VM grow assertions, and synthetic
failed-apply `partialExecutionRecovery` assertions.

This keeps the harnesses available and packaged while preserving safe default checks.

## Further integration hardening

The VM smoke suite and targeted loop tests are the first host-backed integration
paths. Additional disposable VM or lab-host hardening should cover broader:

- LUKS format, grow, keyslot, token, open, close, and property behavior
- LVM LV, thin, cache, volume-group, PV, replacement, and device-topology behavior
- complex storage update behavior

This includes bcache, bcachefs, ZFS, MD RAID, multipath, iSCSI, NFS, VDO, NVMe
namespace, cache, filesystem, swap, zram, loop, backing-file, partition, and
device-mapper update behavior.

- target-side LUN LIO, tgt, and SCST create, attach, detach, destroy, grow, property,
  and rescan behavior
- host-side LUN rescan and multipath resize, add, remove, flush, and replace behavior
- property mutation across more supported domains
- recovery behavior beyond the synthetic LVM-plus-filesystem cases
- broader failed-command and destructive-apply behavior

## Coverage anchors

These exact phrases are kept for the flake documentation coverage check after prose restructuring.

```text
bcachefs device add
targetLuns.<iqn>.operation = "attach"
nvme create-ns <controller>
multipathMaps.resize.operation = "grow"
multipathMaps.flush.destroy = true
mdadm <array> --replace <old-loop> --with <new-loop>
mdadm --examine <removed-loop>
failed-reattach recovery
multi-domain apply plan for
loopSmokeLabel.properties.label
luksSmokeLabel.properties.label
btrfsSmokeLabel.properties.label
filesystems.<name>.replaceDevices
swaps.swapSmokeLabel.properties.label
pools.<name>.properties.autotrim
lvmCaches.<vg/lv>.properties.lvm.cache-mode
lvmCaches.<vg/lv>.removeDevices
lvmCaches.<vg/lv>.addDevices
lvmCaches.<vg/lv>.replaceDevices
disk-nix-lvm-cache-replace
cache sentinel survives
caches.bcacheSmoke.properties."bcache.cache-mode"
caches.bcacheReplacement.replaceDevices
disk-nix-bcache-replace
caches.bcacheSmoke.removeDevices
caches.bcacheFailedAttach.addDevices
failed-attach recovery
caches.bcacheSmoke.addDevices
caches.bcacheSmoke.operation = "rescan"
backingFiles.<path>.properties.mode
loopDevices.<loop>.properties."loop.read-only"
zram.properties.algorithm
services.disk-nix.zram
targetLuns.<iqn>.properties."lio.writeCache"
targetLuns.<iqn>.operation = "detach"
targetLuns.<iqn>.destroy = true
DISK_NIX_LUN_PATH
DISK_NIX_LUN_DATA_SURVIVAL=1
disk-nix-iscsi-lun-sentinel.txt
luns.<target>:0.operation = "rescan"
DISK_NIX_MULTIPATH_RESIZE=1
DISK_NIX_MULTIPATH_ADD_PATH
DISK_NIX_MULTIPATH_REMOVE_PATH
DISK_NIX_MULTIPATH_REPLACE_OLD_PATH
multipathMaps.paths.replaceDevices
DISK_NIX_MULTIPATH_FLUSH=1
DISK_NIX_NVME_CREATE_DELETE=1
DISK_NIX_NVME_GROW=1
DISK_NIX_NVME_ATTACH_DETACH=1
namespace identity drift
nvme delete-ns <controller>
nvme attach-ns <controller>
nvme detach-ns <controller>
multipathMaps.paths.addDevices
DISK_NIX_VM_HARNESSES=target-lun
vdoVolumes.<name>.properties.writePolicy
exports.<path>.properties.options
DISK_NIX_NFS_DATA_SURVIVAL=1
disk-nix-nfs-sentinel.txt
mdRaids.<name>.replaceDevices
mdRaids.<name>.removeDevices
failed-detach recovery
mdRaids.<name>.addDevices
fails and removes one RAID1 member
VM-backed failure-injection apply
rollback review stays non-mutating
clean follow-up apply

```
