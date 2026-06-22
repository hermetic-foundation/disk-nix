# NixOS module

The NixOS module is the primary declarative interface.

## Goals

- define storage once
- emit a normalized JSON planner spec
- derive regular NixOS options such as `fileSystems`, `swapDevices`, initrd
  LUKS devices, ZFS support, Btrfs support, iSCSI/NFS dependencies, and systemd
  units
- keep imperative mutation behind explicit policy

## Initial usage

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;
    apply = {
      mode = "manual";
      allowGrow = true;
      allowShrink = false;
      allowDestructive = false;
    };
    luks.devices.cryptroot = {
      device = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
      allowDiscards = true;
    };
    filesystems.root = {
      device = "/dev/disk/by-label/nixos-root";
      fsType = "xfs";
      mountpoint = "/";
      neededForBoot = true;
      resizePolicy = "grow-only";
    };
    swaps.primary = {
      device = "/dev/disk/by-label/swap";
      priority = 5;
    };
  };
}
```

The module writes `/etc/disk-nix/spec.json`, installs the CLI, and derives the
matching NixOS `fileSystems`, `swapDevices`, and `boot.initrd.luks.devices`
entries. Raw `spec` remains available for storage domains whose typed NixOS
options have not been implemented yet.

Typed filesystem declarations include:

- `device`
- `fsType`
- `mountpoint`
- `options`
- `neededForBoot`
- `resizePolicy`
- `preserveData`

Typed swap declarations include:

- `device`
- `priority`
- `randomEncryption`
- `preserveData`

Typed LUKS declarations include:

- `name`
- `device`
- `allowDiscards`
- `bypassWorkqueues`
- `preLVM`
- `preserveData`

Typed lifecycle declarations are available for:

- `volumes`
- `volumeGroups`
- `pools`
- `datasets`
- `luns`
- `exports`
- `caches`

Each lifecycle declaration includes:

- `operation`
- `addDevices`
- `removeDevices`
- `replaceDevices`
- `properties`
- `destroy`
- `preserveData`
- `metadata`

Typed snapshot declarations include:

- `target`
- `destroy`
- `rollback`
- `preserveData`
- `metadata`

Example lifecycle planning through NixOS options:

```nix
{
  services.disk-nix = {
    pools.tank = {
      operation = "rebalance";
      addDevices = [ "/dev/disk/by-id/nvme-replacement" ];
      removeDevices = [ "/dev/disk/by-id/old-disk" ];
      properties.autotrim = "on";
    };
    datasets."tank/archive".destroy = true;
    snapshots."tank/home@before-upgrade".target = "tank/home";
  };
}
```

## Apply modes

- `manual`: only install the spec and CLI
- `activation`: run apply-policy validation during activation; destructive and
  potential-data-loss actions are refused by default
- `boot`: reserved for boot-time lifecycle work
- `install`: reserved for installer workflows

## Policy

Mutation policy should remain explicit:

- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowGrow`
- `allowPropertyChanges`

Future policies should include:

- `allowDeviceReplacement`
- `allowRebalance`
- `allowOfflineOperations`
- `requireBackup`
- `requireConfirmationFile`
