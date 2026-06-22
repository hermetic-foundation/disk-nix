# disk-nix

`disk-nix` is planned as a NixOS-native storage lifecycle manager: a
read-only storage topology engine first, and a safe imperative planner/apply
engine second.

The long-term goal is a full disko replacement that understands modern Linux
storage stacks:

- block devices, partitions, filesystems, mounts, swap, loop devices
- LUKS and device-mapper mappings
- LVM PVs, VGs, LVs, thin pools, snapshots, cache, and VDO
- Btrfs filesystems, devices, subvolumes, snapshots, qgroups, and usage
- ZFS pools, vdevs, datasets, zvols, snapshots, properties, cache, log, and
  special vdevs
- MD RAID, multipath, NVMe namespaces, iSCSI sessions/targets/LUNs, and NFS
- safe lifecycle operations such as grow, replace, rebalance, property updates,
  and migration advice

The project is licensed under AGPL-3.0-or-later from the beginning.

## Current status

This repository is an initial scaffold. The CLI currently provides a small
read-only `topology` command and the crate boundaries needed for the storage
graph, probing, planning, execution policy, NixOS module integration, and
documentation.

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

## CLI

```sh
disk-nix topology
disk-nix topology --json
disk-nix devices
disk-nix filesystems
disk-nix volumes
disk-nix mappings
disk-nix mounts
disk-nix ids
disk-nix inspect /dev/nvme0n1
disk-nix inspect /
disk-nix plan --spec ./examples/simple-root.json
disk-nix plan --spec ./examples/simple-root.json --json
```

The canonical interface is intended to be stable JSON. Human tables and tree
views are presentation layers over the same model.

## NixOS module

The flake exposes a NixOS module:

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;
    apply.mode = "manual";
  };
}
```

The module currently installs the CLI and writes a normalized storage spec to
`/etc/disk-nix/spec.json`. Future revisions will derive regular NixOS
`fileSystems`, `swapDevices`, initrd LUKS, iSCSI, NFS, ZFS, Btrfs, and related
options from the same source of truth.

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
