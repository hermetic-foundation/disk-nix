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
      probeCurrent = true;
      requireBackup = false;
      requireConfirmation = false;
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
      desiredSize = "100%";
    };
    swaps.primary = {
      device = "/dev/disk/by-label/swap";
      priority = 5;
    };
  };
}
```

The module writes `/etc/disk-nix/spec.json`, installs the CLI, and derives the
matching NixOS `fileSystems`, `swapDevices`, `boot.initrd.luks.devices`,
`services.openiscsi`, and `boot.iscsi-initiator` entries. Raw `spec` remains
available for storage domains whose typed NixOS options have not been
implemented yet.

Typed filesystem declarations include:

- `device`
- `fsType`
- `mountpoint`
- `options`
- `neededForBoot`
- `resizePolicy`
- `desiredSize`
- `preserveData`

Typed swap declarations include:

- `device`
- `operation`
- `desiredSize`
- `priority`
- `randomEncryption`
- `preserveData`

Typed LUKS declarations include:

- `name`
- `device`
- `operation`
- `desiredSize`
- `allowDiscards`
- `bypassWorkqueues`
- `preLVM`
- `preserveData`

Typed NFS client mount declarations include:

- `source`
- `fsType`
- `mountpoint`
- `options`
- `neededForBoot`
- `preserveData`

Typed iSCSI declarations include:

- `initiatorName`
- `discoverPortal`
- `enableAutoLoginOut`
- `extraConfig`
- `sessions`
- `boot.enable`
- `boot.discoverPortal`
- `boot.target`
- `boot.loginAll`
- `boot.logLevel`
- `boot.extraIscsiCommands`
- `boot.extraConfig`

NixOS boot iSCSI currently requires scripted stage 1. Configurations using
`iscsi.boot.enable = true` must keep `boot.initrd.systemd.enable = false` until
the upstream `boot.iscsi-initiator` module supports systemd initrd.

Typed lifecycle declarations are available for:

- `disks`
- `partitions`
- `btrfsSubvolumes`
- `vdoVolumes`
- `volumes`
- `volumeGroups`
- `thinPools`
- `mdRaids`
- `multipathMaps`
- `pools`
- `datasets`
- `zvols`
- `luns`
- `iscsi.sessions`
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
- `desiredSize`
- `target`
- `path`
- `mountpoint`
- `device`
- `start`
- `end`
- `partitionType`
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
    apply = {
      mode = "activation";
      probeCurrent = true;
      scriptOut = "/run/disk-nix/apply.sh";
      reportOut = "/run/disk-nix/apply-report.json";
    };
    partitions.root = {
      operation = "grow";
      device = "/dev/disk/by-id/nvme-root-part2";
      desiredSize = "100%";
    };
    swaps.primary = {
      device = "/dev/disk/by-label/swap";
      operation = "format";
      desiredSize = "8GiB";
    };
    luks.devices.cryptroot = {
      device = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
      operation = "grow";
      desiredSize = "100%";
    };
    vdoVolumes.archive = {
      operation = "grow";
      desiredSize = "4TiB";
    };
    btrfsSubvolumes."/mnt/persist/@home" = {
      operation = "create";
      path = "/mnt/persist/@home";
    };
    pools.tank = {
      operation = "rebalance";
      addDevices = [ "/dev/disk/by-id/nvme-replacement" ];
      removeDevices = [ "/dev/disk/by-id/old-disk" ];
      properties.autotrim = "on";
    };
    datasets."tank/archive".destroy = true;
    zvols."tank/vm/root" = {
      operation = "grow";
      desiredSize = "80GiB";
    };
    thinPools."vg0/thinpool" = {
      operation = "grow";
      desiredSize = "500GiB";
    };
    mdRaids.root = {
      target = "/dev/md/root";
      addDevices = [ "/dev/disk/by-id/nvme-md-spare" ];
    };
    multipathMaps.mpatha = {
      target = "mpatha";
      addDevices = [ "/dev/sdb" ];
    };
    nfs.mounts."/srv/shared" = {
      source = "nas.example.com:/srv/shared";
      fsType = "nfs4";
      options = [ "_netdev" "x-systemd.automount" "vers=4.2" ];
    };
    iscsi = {
      initiatorName = "iqn.2026-06.example:host";
      discoverPortal = "192.0.2.10:3260";
      enableAutoLoginOut = true;
      boot = {
        enable = true;
        target = "iqn.2026-06.example:storage.root";
      };
      sessions."iqn.2026-06.example:storage.root" = {
        operation = "grow";
        desiredSize = "2TiB";
        metadata.portal = "192.0.2.10:3260";
      };
    };
    snapshots."tank/home@before-upgrade".target = "tank/home";
  };
}
```

## Apply modes

- `manual`: only install the spec and CLI
- `activation`: run apply-policy validation during activation; destructive and
  potential-data-loss actions are refused by default. Set `probeCurrent = true`
  to include current topology comparison in that validation report. Set
  `scriptOut` to emit the allowed command and verification plan as a reviewable
  shell script during validation. Set `reportOut` to persist the JSON report
  before blocked-policy failures are returned. Set `failOnBlocked = false` to
  run `disk-nix validate` during activation so blocked policy is reported
  without failing the unit.
- `boot`: reserved for boot-time lifecycle work
- `install`: reserved for installer workflows

## Policy

Mutation policy should remain explicit:

- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowGrow`
- `allowOffline`
- `allowPropertyChanges`
- `allowDeviceReplacement`
- `allowRebalance`
- `requireBackup`
- `backupVerified`
- `requireConfirmation`
- `confirmation`
- `requireConfirmationFile`
- `probeCurrent`
- `failOnBlocked`
- `scriptOut`
- `reportOut`

`requireBackup` and `requireConfirmation` are additional safety gates for
high-risk actions. `requireConfirmationFile` stores the expected file path in
the generated policy; `disk-nix apply` only treats it as confirmed when the file
contains a standalone line equal to `disk-nix confirm`.
`failOnBlocked` defaults to true. When false, activation keeps writing the same
report data but uses `disk-nix validate`, which exits successfully even when
policy blocks planned actions.
`scriptOut` must be an absolute path. The activation service creates its parent
directory before asking the CLI to write the review script.
`reportOut` must also be an absolute path. The activation service creates its
parent directory before asking the CLI to write the JSON apply report.
