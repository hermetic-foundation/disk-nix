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

By default the suite runs the loop, Btrfs, LUKS, LVM, and MD RAID smoke
harnesses. To run a subset:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VM_HARNESSES="loop btrfs" \
  nix run .#integration-vm-smoke
```

The individual harnesses below remain available for targeted lab debugging,
but they should still be treated as destructive host operations.

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

- creates a temporary 128 MiB backing file
- attaches it to the next available `/dev/loop*`
- creates a temporary LVM physical volume and volume group
- verifies `disk-nix inspect <vg> --json` sees the volume group
- executes a `volumeGroups.<name>.operation = "rescan"` apply plan
- verifies the generated JSON report was written and the rendered
  `pvscan --cache`, `vgscan`, and `vgchange --refresh <vg>` commands succeeded
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

## Flake coverage

`nix flake check` does not run destructive integration tests. It does validate
that the loop smoke harnesses parse, remain opt-in, and still contain the
expected loop, filesystem setup, resize, mount, scrub, LUKS format, LUKS open,
LUKS close, LVM create, LVM rescan, MD RAID create, MD RAID rescan, and VM
orchestration guard steps. This keeps the harnesses available and packaged
while preserving safe default checks.

## Remaining integration coverage

The VM smoke suite and targeted loop tests are only the first host-backed
integration paths. Feature completion still needs disposable VM or lab-host
tests for broader LUKS format/grow/keyslot/token behavior, broader LVM
LV/thin/cache/device-topology behavior, bcachefs, ZFS, broader MD RAID
grow/member-topology behavior, multipath, iSCSI, NFS, VDO, NVMe namespace
operations, failure recovery, and broader destructive apply behavior.
