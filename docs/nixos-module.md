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

The module writes `/etc/disk-nix/spec.json`, installs the CLI and default
storage tooling, and derives the matching NixOS `fileSystems`, `swapDevices`,
`boot.initrd.luks.devices`, `boot.supportedFilesystems`, `services.lvm`,
`boot.initrd.services.lvm`, `boot.swraid`, `services.multipath`,
`boot.zfs.extraPools`, `boot.bcache`, `boot.initrd.services.bcache`,
`services.openiscsi`, `boot.iscsi-initiator`, and selected
`services.nfs.server.exports` entries.
Raw `spec` remains available for storage domains whose typed NixOS options have
not been implemented yet.

`toolPackages` defaults to the storage tools used by the probe and executor
adapters, including Btrfs, ext, XFS, exFAT, LVM, cryptsetup, MD RAID,
multipath, NFS, iSCSI, NVMe, VDO, bcache, ZFS, partitioning, and util-linux
tooling. The apply service adds these packages to `PATH`, and the same packages
are installed in `environment.systemPackages`. Override the list to pin
site-specific tool builds or to trim unused storage domains.

Typed NFS export declarations derive regular NixOS NFS server export lines
only when they are non-destroy declarations with explicit `client` and
`options` fields. Destructive or under-specified export declarations remain in
the disk-nix planner spec for review instead of being re-added to `/etc/exports`.
Typed swap and LUKS declarations follow the same split: destroy operations stay
in the generated disk-nix spec, but they are not re-added to NixOS
`swapDevices` or `boot.initrd.luks.devices`.
Typed NFS client mounts also keep destroy operations in the generated disk-nix
spec while filtering them out of the derived NixOS `fileSystems` entries.
Typed active LVM declarations enable NixOS LVM support and initrd LVM support by
default, and typed thin-pool or LVM-cache declarations also enable NixOS thin
support. Typed active MD RAID declarations enable `boot.swraid` and add the same
no-op `PROGRAM` line used by the installer profile unless the host overrides
`boot.swraid.mdadmConf`. Typed active multipath map declarations enable
`services.multipath` so stage-1 and stage-2 include the daemon and kernel
support expected by `/dev/mapper/mpath*` consumers.
Typed active ZFS pool, dataset, zvol, and ZFS snapshot declarations add their
pool names to `boot.zfs.extraPools` and include `zfs` in
`boot.supportedFilesystems`, so NixOS imports pools that disk-nix is asked to
manage even when no legacy-mounted ZFS `fileSystems` entry references them.
NixOS requires `networking.hostId` whenever ZFS support is enabled.
Typed active bcache cache declarations enable NixOS bcache support and initrd
bcache udev support by default, so `/dev/bcache*` mappings are assembled before
early consumers try to mount or inspect them.

Lifecycle declaration attribute names are usable object names only for domains
whose native tools address objects by name, such as ZFS datasets, ZFS pools,
VDO volumes, and iSCSI target IQNs. Domains addressed by kernel paths or
compound LVM names need concrete targets before `apply --execute` can run:
swap and NFS exports need local paths, loop devices need `/dev/loop*`, MD RAID
arrays need `/dev/md*`, multipath maps need `mpath*` or `/dev/mapper/*`, bcache
operations need `/dev/bcache*`, and LVM logical volumes and thin pools need
canonical `vg/lv` or `vg/pool` targets. Declarations that omit these concrete
addresses still produce reviewable plans, but their command plans stay
non-ready instead of guessing from logical keys.
MD RAID member add, replacement, and removal declarations use the same explicit
array target requirement as create and grow plans. Multipath replacement
declarations use the concrete map target for preflight inspection, then render
separate path add and delete commands from the `replaceDevices` mapping.

Typed filesystem declarations include:

- `device`
- `fsType`
- `mountpoint`
- `options`
- `neededForBoot`
- `operation`
- `addDevices`
- `removeDevices`
- `replaceDevices`
- `properties`
- `resizePolicy`
- `desiredSize`
- `preserveData`

For ext filesystems, `device` is also used by disk-nix grow, shrink, check, and
repair command plans for `resize2fs` and `e2fsck`. If only `mountpoint` is
declared, source-device maintenance commands remain non-ready until the backing
block device is selected explicitly. XFS label changes also use `device` for
`xfs_admin -L`; FAT/vfat label changes use `device` for `fatlabel`; NTFS label
changes use `device` for `ntfslabel`; exFAT label changes use `device` for
`exfatlabel`. Btrfs, ext, FAT/vfat, NTFS, exFAT, and XFS UUID, volume-ID, or
volume-serial changes use `device` for `btrfstune -U`, `tune2fs -U`,
`fatlabel -i`, `ntfslabel --new-serial`, `exfatlabel -i`, and `xfs_admin -U`
and are offline-required. Check and repair declarations require a stable source
device for tools such as `e2fsck`, `xfs_repair`, `btrfs check`, `fsck.fat`,
`fsck.exfat`, or `ntfsfix`; NTFS repair remains limited Linux-side remediation,
not a replacement for Windows `chkdsk`.
For Btrfs filesystems, typed declarations can also request `operation = "rebalance"`, `operation = "check"`, `operation = "repair"`, `operation = "scrub"`, `operation = "trim"`, device add/remove/replace operations, and filesystem property
updates such as labels or balance filters while still deriving the regular
NixOS `fileSystems` entry from the same declaration.
For ZFS pools, typed declarations can request `operation = "scrub"` to render
reviewed `zpool scrub` plans.

Typed swap declarations include:

- `device`
- `operation`
- `desiredSize`
- `priority`
- `randomEncryption`
- `preserveData`
- `properties`

Typed LUKS declarations include:

- `name`
- `device`
- `operation`
- `desiredSize`
- `allowDiscards`
- `bypassWorkqueues`
- `preLVM`
- `preserveData`
- `destroy`
- `properties`

Typed NFS client mount declarations include:

- `source`
- `fsType`
- `mountpoint`
- `options`
- `neededForBoot`
- `operation`
- `destroy`
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
- `btrfsQgroups`
- `vdoVolumes`
- `physicalVolumes`
- `luksKeyslots`
- `luksTokens`
- `volumes`
- `volumeGroups`
- `thinPools`
- `lvmSnapshots`
- `lvmCaches`
- `loopDevices`
- `mdRaids`
- `multipathMaps`
- `pools`
- `datasets`
- `zvols`
- `luns`
- `nvmeNamespaces`
- `iscsi.sessions`
- `exports`
- `caches`

Each lifecycle declaration includes:

- `operation`
- `addDevices`
- `devices`
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
- `startOffset`
- `end`
- `endOffset`
- `partitionNumber`
- `number`
- `partitionType`
- `level`
- `raidLevel`
- `portal`
- `namespaceId`
- `controllers`
- `metadata`

Typed snapshot declarations include:

- `target`
- `destroy`
- `rollback`
- `hold`
- `holdTag`
- `releaseHold`
- `preserveData`
- `metadata`

Address fields have domain-specific meaning:

- `target`: native object name or required concrete command target; use
  `vg/lv` for logical volumes, `vg/pool` for thin pools, `/dev/md*` for MD
  arrays, `/dev/bcache*` for bcache, and `mpath*` or `/dev/mapper/*` for
  multipath maps
- `path`: local filesystem path for Btrfs subvolumes, Btrfs qgroups, and NFS
  exports
- `device`: backing block device or image path used by formats, LUKS, swap,
  filesystems, partitions, and loop-device setup
- `portal`: iSCSI target portal; `metadata.portal` is accepted for
  module-derived session declarations

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
      properties.label = "swap";
      properties."swap.uuid" = "01234567-89ab-cdef-0123-456789abcdef";
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
    filesystems.data = {
      device = "/dev/disk/by-label/data";
      fsType = "btrfs";
      mountpoint = "/data";
      operation = "rebalance";
      addDevices = [ "/dev/disk/by-id/nvme-btrfs-new" ];
      removeDevices = [ "/dev/disk/by-id/nvme-btrfs-old" ];
      properties = {
        label = "bulk-data";
        "btrfs.balance.data" = "usage=50";
      };
    };
    btrfsSubvolumes."/mnt/persist/@home" = {
      operation = "create";
      path = "/mnt/persist/@home";
    };
    btrfsQgroups."0/257" = {
      target = "/mnt/persist";
      properties.limit = "25GiB";
    };
    volumes."vg0/scratch" = {
      operation = "create";
      desiredSize = "10GiB";
    };
    pools.tank = {
      operation = "rebalance";
      addDevices = [ "/dev/disk/by-id/nvme-replacement" ];
      removeDevices = [ "/dev/disk/by-id/old-disk" ];
      properties.autotrim = "on";
    };
    datasets."tank/home".operation = "create";
    datasets."tank/archive".destroy = true;
    zvols."tank/vm/root" = {
      operation = "grow";
      desiredSize = "80GiB";
    };
    thinPools."vg0/thinpool" = {
      operation = "grow";
      desiredSize = "500GiB";
    };
    thinPools."vg0/newthin" = {
      operation = "create";
      desiredSize = "100GiB";
    };
    lvmSnapshots."vg0/root-snap" = {
      operation = "snapshot";
      target = "vg0/root";
      desiredSize = "20GiB";
    };
    lvmCaches."vg0/root" = {
      operation = "create";
      device = "vg0/root-cache";
      properties."lvm.cache-mode" = "writethrough";
    };
    luksKeyslots."cryptroot:1" = {
      operation = "create";
      device = "/dev/disk/by-id/root-luks";
      keySlot = "1";
      newKeyFile = "/run/keys/root-new";
    };
    luksTokens."cryptroot:0" = {
      operation = "create";
      device = "/dev/disk/by-id/root-luks";
      tokenId = "0";
      tokenFile = "/run/keys/root-token.json";
    };
    loopDevices."/dev/loop7" = {
      operation = "create";
      device = "/var/lib/images/root.img";
    };
    mdRaids.root = {
      target = "/dev/md/root";
      addDevices = [ "/dev/disk/by-id/nvme-md-spare" ];
    };
    multipathMaps.mpatha = {
      target = "mpatha";
      addDevices = [ "/dev/sdb" ];
    };
    exports."/srv/share" = {
      operation = "create";
      client = "192.0.2.0/24";
      options = "rw,sync,no_subtree_check";
    };
    caches."tank/l2arc0" = {
      replaceDevices."/dev/disk/by-id/old-cache" = "/dev/disk/by-id/new-cache";
    };
    caches."/dev/bcache0" = {
      addDevices = [ "cache-set-uuid" ];
      properties."bcache.cache-mode" = "writethrough";
    };
    nfs.mounts."/srv/shared" = {
      source = "nas.example.com:/srv/shared";
      fsType = "nfs4";
      options = [ "_netdev" "x-systemd.automount" "vers=4.2" ];
    };
    nfs.mounts."/srv/old" = {
      source = "nas.example.com:/srv/old";
      operation = "destroy";
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
    snapshots."/mnt/persist/@home-before-upgrade" = {
      target = "/mnt/persist/@home";
      readOnly = true;
    };
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
  without failing the unit. Set `execute = true` to run ready, policy-allowed
  commands with `disk-nix apply --execute` during activation; this requires
  `failOnBlocked = true`.
- `boot`: reserved for boot-time lifecycle work; the module emits a warning and
  does not wire imperative apply for this mode yet
- `install`: reserved for installer workflows; the module emits a warning and
  does not wire imperative apply for this mode yet

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
- `execute`
- `scriptOut`
- `reportOut`

`requireBackup` and `requireConfirmation` are additional safety gates for
high-risk actions. `requireConfirmationFile` stores the expected file path in
the generated policy; `disk-nix apply` only treats it as confirmed when the file
contains a standalone line equal to `disk-nix confirm`.
`failOnBlocked` defaults to true. When false, activation keeps writing the same
report data but uses `disk-nix validate`, which exits successfully even when
policy blocks planned actions.
`execute` defaults to false. When true, activation runs `disk-nix apply --execute` after policy validation and command-readiness checks pass. The module
requires `failOnBlocked = true` for this mode because `disk-nix validate` is
report-only.
`scriptOut` must be an absolute path. The activation service creates its parent
directory before asking the CLI to write the review script.
`reportOut` must also be an absolute path. The activation service creates its
parent directory before asking the CLI to write the JSON apply report.
