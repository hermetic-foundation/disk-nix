# disk-nix

`disk-nix` is a NixOS-native storage lifecycle manager.

It has two jobs:

- discover the current Linux storage topology
- plan and execute guarded storage lifecycle changes from declarative specs

The project is intended to cover the full disko-replacement scope: disks,
partitions, filesystems, mounts, swap, zram, LUKS, device-mapper, LVM, VDO,
Btrfs, bcachefs, ZFS, MD RAID, multipath, NVMe, iSCSI, target-side LUNs, NFS,
bcache, loop devices, backing files, snapshots, cache layers, health metadata,
and recovery guidance.

The license is AGPL-3.0-or-later.

## Documentation

Start here:

- [docs/index.md](docs/index.md): documentation map and recommended reading
  order
- [docs/user-guide.md](docs/user-guide.md): task-oriented guide for using the
  project
- [docs/status.md](docs/status.md): implemented behavior and hardening notes
- [docs/feature-checklist.md](docs/feature-checklist.md): completed feature
  scope
- [docs/operator-runbooks.md](docs/operator-runbooks.md): high-risk workflows
  such as replacement, rollback, failed apply recovery, degraded arrays, and
  shared storage

Reference docs:

- [docs/cli.md](docs/cli.md): CLI commands and JSON contracts
- [docs/planning.md](docs/planning.md): planning, risk classification,
  reconciliation, dependency ordering, and apply policy
- [docs/nixos-module.md](docs/nixos-module.md): NixOS module options and
  generated files
- [docs/storage-scope.md](docs/storage-scope.md): storage domains and update
  operations
- [docs/integration-tests.md](docs/integration-tests.md): VM and host-backed
  smoke tests
- [docs/compatibility.md](docs/compatibility.md): versioning and compatibility
  policy
- [docs/architecture.md](docs/architecture.md): implementation structure

## Quick Start

Enter the development shell:

```sh
nix develop
```

Build the CLI:

```sh
nix build
```

Run the full safe check set:

```sh
nix flake check
```

Render the documentation for browser review:

```sh
node scripts/render-docs.mjs
```

Open `build/docs-site/index.html` in a browser.

## Inspect Storage

Use `topology` when you want the full graph:

```sh
disk-nix topology
disk-nix topology --json
```

Use focused views when you want one storage domain:

```sh
disk-nix devices
disk-nix filesystems
disk-nix lvm
disk-nix zfs
disk-nix btrfs
disk-nix nvme
disk-nix iscsi
disk-nix luns
disk-nix nfs
disk-nix network-storage
disk-nix ids --json
```

Use `inspect` when you know the target object:

```sh
disk-nix inspect /dev/nvme0n1 --json
disk-nix inspect tank/home --json
disk-nix inspect iqn.2026-06.example:storage.root --json
```

## Plan Changes

Write a versioned spec:

```json
{
  "version": 1,
  "filesystems": {
    "root": {
      "device": "/dev/disk/by-label/nixos-root",
      "fsType": "xfs",
      "mountpoint": "/",
      "resizePolicy": "grow-only",
      "desiredSize": "100%"
    }
  },
  "apply": {
    "probeCurrent": true
  }
}
```

Plan it:

```sh
disk-nix plan --spec ./storage.json
disk-nix plan --spec ./storage.json --probe-current --json
```

Validate it:

```sh
disk-nix validate --spec ./storage.json
```

Migrate older aliases into the current versioned shape:

```sh
disk-nix migrate --spec ./storage.json --json
```

## Apply Safely

`apply` is policy-gated.

By default, it produces a review report instead of blindly mutating storage:

```sh
disk-nix apply --spec ./storage.json --json
```

Generate a script after policy validation:

```sh
disk-nix apply --spec ./storage.json --script-out ./apply.sh
```

Execute only when the spec policy and operator review allow it:

```sh
sudo disk-nix apply --spec ./storage.json --execute --report-out ./apply-report.json
```

High-risk operations stay blocked until the spec opts in with the relevant
policy flags, such as `allowOffline`, `allowDestructive`,
`allowPotentialDataLoss`, or `requireBackup`.

## NixOS Module

Import the module:

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;

    apply = {
      mode = "manual";
      probeCurrent = true;
      allowGrow = true;
      allowShrink = false;
      allowDestructive = false;
      allowPotentialDataLoss = false;
    };

    filesystems.root = {
      device = "/dev/disk/by-label/nixos-root";
      fsType = "xfs";
      mountpoint = "/";
      neededForBoot = true;
      resizePolicy = "grow-only";
      desiredSize = "100%";
    };
  };
}
```

The module writes:

- `/etc/disk-nix/spec.json`
- `/etc/disk-nix/steady-state.json`
- optional declarative handoff files for reviewed post-mutation NixOS updates

It can also derive native NixOS settings such as `fileSystems`, `swapDevices`,
initrd LUKS devices, supported filesystems, ZFS pools, Btrfs support, zram,
NFS exports, iSCSI settings, MD RAID, multipath, bcache, LVM, and VDO service
flags.

## Safety Model

`disk-nix` treats storage mutation as a reviewable operation.

The planner classifies actions as:

- safe
- reversible
- online
- offline-required
- destructive
- potential-data-loss
- unsupported

Blocked reports include non-destructive advice where possible. Failed execution
reports include `partialExecutionRecovery`, retry guidance, roll-forward
review, domain-specific recovery notes, rollback recipes where they are
provably safe, and operator-only handoff where automation would be unsafe.

## Integration Tests

Default flake checks do not run destructive host mutations.

Run the disposable VM suite explicitly:

```sh
nix build .#integration-vm-test
```

Run a root-only smoke harness only on disposable storage:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 nix run .#integration-loop-smoke
```

Dry-run the translated disko example suite:

```sh
nix run .#integration-disko-examples
```

Its destructive mode is separately guarded and expects disposable
`/dev/sdb` through `/dev/sdf`.

```sh
sudo env DISK_NIX_DISKO_E2E_EXECUTE=1 \
  DISK_NIX_DISKO_E2E_CONFIRM=wipe-/dev/sdb-/dev/sdc-/dev/sdd-/dev/sde-/dev/sdf \
  nix run .#integration-disko-examples
```

On hosts without ZFS or bcachefs kernel support, the destructive suite still
plans and preflights those examples but skips their execution.

Run the synthetic failure-recovery harness without root:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 nix run .#integration-failure-recovery-smoke
```

See [docs/integration-tests.md](docs/integration-tests.md) before running any
host-backed harness.
