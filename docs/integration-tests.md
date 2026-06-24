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

## Flake coverage

`nix flake check` does not run destructive integration tests. It does validate
that the loop smoke harness parses, remains opt-in, and still contains the
expected loop and filesystem setup steps. This keeps the harness available and
packaged while preserving safe default checks.

## Remaining integration coverage

The loop smoke test is only the first host-backed integration path. Feature
completion still needs disposable VM or lab-host tests for LUKS, LVM, Btrfs,
bcachefs, ZFS, MD RAID, multipath, iSCSI, NFS, VDO, NVMe namespace operations,
failure recovery, and destructive apply behavior.
