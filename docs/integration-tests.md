# Integration tests

Unit tests and flake checks cover parsers, planning, command rendering, NixOS
module evaluation, examples, schema generation, completions, and manpage output.
Real storage mutation needs additional host-backed tests because Nix build
sandboxes cannot safely create privileged block devices.

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

## Flake coverage

`nix flake check` does not run destructive integration tests. It does validate
that the loop smoke harnesses parse, remain opt-in, and still contain the
expected loop, filesystem setup, resize, mount, and scrub steps. This keeps the
harnesses available and packaged while preserving safe default checks.

## Remaining integration coverage

The loop smoke tests are only the first host-backed integration paths. Feature
completion still needs disposable VM or lab-host tests for LUKS, LVM,
bcachefs, ZFS, MD RAID, multipath, iSCSI, NFS, VDO, NVMe namespace operations,
failure recovery, and broader destructive apply behavior.
