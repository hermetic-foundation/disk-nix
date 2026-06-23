self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.disk-nix;
  json = pkgs.formats.json { };
  applyScriptOutDir = lib.optionalString (cfg.apply.scriptOut != null) (
    builtins.dirOf cfg.apply.scriptOut
  );
  applyReportOutDir = lib.optionalString (cfg.apply.reportOut != null) (
    builtins.dirOf cfg.apply.reportOut
  );
  applyCommand = if cfg.apply.failOnBlocked then "apply" else "validate";
  applyPolicy = builtins.removeAttrs cfg.apply [ "execute" ];
  defaultToolPackages = with pkgs; [
    bcachefs-tools
    bcache-tools
    btrfs-progs
    cryptsetup
    dosfstools
    e2fsprogs
    exfatprogs
    f2fs-tools
    lvm2
    mdadm
    multipath-tools
    nfs-utils
    ntfs3g
    nvme-cli
    openiscsi
    parted
    util-linux
    vdo
    xfsprogs
    zfs
  ];
  applyArgs = [
    applyCommand
    "--spec"
    "/etc/disk-nix/spec.json"
  ]
  ++ lib.optional cfg.apply.probeCurrent "--probe-current"
  ++ lib.optional cfg.apply.execute "--execute"
  ++ lib.optionals (cfg.apply.scriptOut != null) [
    "--script-out"
    cfg.apply.scriptOut
  ]
  ++ lib.optionals (cfg.apply.reportOut != null) [
    "--report-out"
    cfg.apply.reportOut
  ];
  applyValidationScript = pkgs.writeShellScript "disk-nix-apply-validation" ''
    ${lib.optionalString (cfg.apply.scriptOut != null) ''
      mkdir -p ${lib.escapeShellArg applyScriptOutDir}
    ''}
    ${lib.optionalString (cfg.apply.reportOut != null) ''
      mkdir -p ${lib.escapeShellArg applyReportOutDir}
    ''}
    exec ${lib.escapeShellArgs ([ (lib.getExe cfg.package) ] ++ applyArgs)}
  '';
  operationType = lib.types.nullOr (
    lib.types.enum [
      "create"
      "format"
      "grow"
      "shrink"
      "check"
      "repair"
      "scrub"
      "trim"
      "replace-device"
      "add-device"
      "remove-device"
      "set-property"
      "snapshot"
      "promote"
      "import"
      "export"
      "activate"
      "deactivate"
      "assemble"
      "start"
      "stop"
      "open"
      "close"
      "remount"
      "rename"
      "rebalance"
      "rollback"
      "destroy"
    ]
  );
  lifecycleSubmodule =
    { name, ... }:
    {
      options = {
        operation = lib.mkOption {
          type = operationType;
          default = null;
          description = "Requested lifecycle operation for this storage object.";
          example = "grow";
        };

        addDevices = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "Devices to add to this storage object.";
          example = [ "/dev/disk/by-id/nvme-replacement" ];
        };

        devices = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "Explicit member or path devices for storage objects such as MD RAID arrays, ZFS pools, LUNs, and multipath maps.";
          example = [
            "/dev/disk/by-id/nvme-a"
            "/dev/disk/by-id/nvme-b"
          ];
        };

        removeDevices = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "Devices to remove from this storage object.";
          example = [ "/dev/disk/by-id/old-disk" ];
        };

        replaceDevices = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = { };
          description = "Mapping of old device path to replacement device path.";
          example = {
            "/dev/disk/by-id/old-cache" = "/dev/disk/by-id/new-cache";
          };
        };

        renameTo = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "New name or path for rename lifecycle operations.";
          example = "tank/home-staged";
        };

        properties = lib.mkOption {
          type = lib.types.attrsOf json.type;
          default = { };
          description = "Storage-specific properties to set on this object.";
          example = {
            autotrim = "on";
          };
        };

        destroy = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Request destruction of this object.";
        };

        preserveData = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Whether disk-nix must preserve data for this object.";
        };

        readOnly = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Request a read-only lifecycle action when the storage domain supports it, such as ZFS pool import.";
          example = true;
        };

        desiredSize = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Desired object size for grow, shrink, or provisioning plans.";
          example = "100GiB";
        };

        target = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = ''
            Explicit target identity when it differs from the attribute name.
            Some command domains require concrete targets for executable plans:
            LVM logical volumes use vg/lv, LVM thin pools use vg/pool, MD RAID
            arrays use /dev/md*, multipath maps use mpath* or /dev/mapper/*,
            bcache uses /dev/bcache*, and loop devices use /dev/loop*.
          '';
          example = "tank/home";
        };

        path = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Filesystem path for path-addressed lifecycle objects such as Btrfs subvolumes.";
          example = "/mnt/persist/@home";
        };

        mountpoint = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Mountpoint for lifecycle objects addressed by mounted path.";
          example = "/home";
        };

        device = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Backing device or partition path for this lifecycle object.";
          example = "/dev/disk/by-id/nvme-root";
        };

        client = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Client or network selector for NFS export lifecycle declarations.";
          example = "192.0.2.0/24";
        };

        options = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Option string for NFS export lifecycle declarations.";
          example = "rw,sync,no_subtree_check";
        };

        start = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Partition start offset for partition lifecycle declarations.";
          example = "1MiB";
        };

        startOffset = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for start, accepted by the planner for partition lifecycle declarations.";
          example = "1MiB";
        };

        end = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Partition end offset or size for partition lifecycle declarations.";
          example = "100%";
        };

        endOffset = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for end, accepted by the planner for partition lifecycle declarations.";
          example = "100%";
        };

        partitionNumber = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Partition number used by partition resize lifecycle declarations.";
          example = "1";
        };

        number = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for partitionNumber, accepted by the planner for partition lifecycle declarations.";
          example = "1";
        };

        partitionType = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Partition type/name argument used by partition lifecycle declarations.";
          example = "linux";
        };

        level = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "RAID level for array lifecycle declarations.";
          example = "1";
        };

        raidLevel = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for level, accepted by the planner for RAID lifecycle declarations.";
          example = "1";
        };

        portal = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Network storage portal for lifecycle declarations such as iSCSI sessions.";
          example = "192.0.2.10:3260";
        };

        namespaceId = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "NVMe namespace id used by namespace attach, detach, and delete lifecycle declarations.";
          example = "4";
        };

        controllers = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Comma-separated NVMe controller id list used by namespace attach and detach operations.";
          example = "0x1";
        };

        keySlot = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "LUKS keyslot number used by keyslot lifecycle declarations.";
          example = "1";
        };

        keyFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Existing LUKS key file used when changing key material.";
          example = "/run/keys/root-old";
        };

        newKeyFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Replacement or newly enrolled LUKS key file.";
          example = "/run/keys/root-new";
        };

        tokenId = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "LUKS token id used by token lifecycle declarations.";
          example = "0";
        };

        tokenFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "JSON token file imported by LUKS token lifecycle declarations.";
          example = "/run/keys/root-token.json";
        };

        jsonFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for tokenFile accepted by LUKS token lifecycle declarations.";
          example = "/run/keys/root-token.json";
        };

        metadata = lib.mkOption {
          type = lib.types.attrsOf json.type;
          default = { };
          description = "Domain-specific metadata copied into the planner spec.";
          example = {
            target = "iqn.2026-06.example:storage/root";
            lun = 0;
          };
        };
      };
    };
  snapshotSubmodule =
    { name, ... }:
    {
      options = {
        target = lib.mkOption {
          type = lib.types.str;
          default = name;
          defaultText = lib.literalExpression "<attribute name>";
          description = "Dataset, volume, or filesystem target for this snapshot.";
          example = "tank/home";
        };

        destroy = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Request snapshot destruction.";
        };

        rollback = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Request rollback of the target to this snapshot.";
        };

        cloneTo = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "ZFS dataset target for cloning this snapshot.";
          example = "tank/home-review";
        };

        renameTo = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "New ZFS snapshot name or Btrfs snapshot path for rename lifecycle operations.";
          example = "tank/home@before-prune";
        };

        recursiveRollback = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Render recursive ZFS rollback with zfs rollback -r when explicitly true.";
        };

        hold = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "ZFS hold tag to apply to this snapshot.";
          example = "disk-nix-retain";
        };

        holdTag = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for hold, copied into the planner spec.";
          example = "disk-nix-retain";
        };

        releaseHold = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "ZFS hold tag to release from this snapshot.";
          example = "old-retention-tag";
        };

        readOnly = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Create this snapshot read-only when the target domain supports it, such as Btrfs subvolume snapshots.";
          example = true;
        };

        preserveData = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Whether newer target data should be preserved.";
        };

        metadata = lib.mkOption {
          type = lib.types.attrsOf json.type;
          default = { };
          description = "Domain-specific snapshot metadata copied into the planner spec.";
        };
      };
    };
  lifecycleAttrs = lib.types.attrsOf (lib.types.submodule lifecycleSubmodule);
  snapshotAttrs = lib.types.attrsOf (lib.types.submodule snapshotSubmodule);
  cleanSpecAttrs = lib.filterAttrs (_: value: value != null && value != [ ] && value != { });
  normalizeLifecycleSpec = lib.mapAttrs (
    _: object:
    object.metadata
    // cleanSpecAttrs {
      inherit (object)
        operation
        addDevices
        devices
        removeDevices
        replaceDevices
        renameTo
        properties
        destroy
        preserveData
        readOnly
        desiredSize
        target
        path
        mountpoint
        device
        client
        options
        start
        startOffset
        end
        endOffset
        partitionNumber
        number
        partitionType
        level
        raidLevel
        portal
        namespaceId
        controllers
        keySlot
        keyFile
        newKeyFile
        tokenId
        tokenFile
        jsonFile
        ;
    }
  );
  normalizeSnapshotSpec = lib.mapAttrs (
    _: snapshot:
    snapshot.metadata
    // cleanSpecAttrs {
      inherit (snapshot)
        target
        destroy
        rollback
        cloneTo
        renameTo
        recursiveRollback
        hold
        holdTag
        releaseHold
        readOnly
        preserveData
        ;
    }
  );
  typedFilesystemSpec = lib.mapAttrs (_: filesystem: {
    inherit (filesystem)
      device
      fsType
      mountpoint
      options
      neededForBoot
      operation
      addDevices
      removeDevices
      replaceDevices
      properties
      resizePolicy
      preserveData
      desiredSize
      ;
  }) cfg.filesystems;
  typedNfsMountSpec = lib.mapAttrs (_: mount: {
    inherit (mount)
      source
      fsType
      mountpoint
      options
      neededForBoot
      operation
      destroy
      preserveData
      ;
    device = mount.source;
  }) cfg.nfs.mounts;
  typedNfsFilesystemSpec = lib.mapAttrs (_: mount: {
    inherit (mount)
      fsType
      mountpoint
      options
      neededForBoot
      preserveData
      ;
    device = mount.source;
  }) activeNfsMounts;
  typedSwapSpec = lib.mapAttrs (_: swap: {
    inherit (swap)
      device
      operation
      desiredSize
      priority
      randomEncryption
      preserveData
      properties
      ;
  }) cfg.swaps;
  typedLuksSpec = lib.mapAttrs (_: luks: {
    inherit (luks)
      device
      name
      operation
      desiredSize
      allowDiscards
      bypassWorkqueues
      preLVM
      preserveData
      destroy
      properties
      ;
  }) cfg.luks.devices;
  typedIscsiSpec = cleanSpecAttrs {
    inherit (cfg.iscsi)
      initiatorName
      discoverPortal
      enableAutoLoginOut
      extraConfig
      ;
    boot = cleanSpecAttrs {
      inherit (cfg.iscsi.boot)
        enable
        discoverPortal
        target
        loginAll
        logLevel
        extraIscsiCommands
        extraConfig
        ;
    };
    sessions = normalizeLifecycleSpec cfg.iscsi.sessions;
  };
  filesystemToNixos =
    filesystem:
    {
      inherit (filesystem)
        device
        fsType
        neededForBoot
        ;
    }
    // lib.optionalAttrs (filesystem.options != [ ]) {
      inherit (filesystem) options;
    };
  isDestroyLifecycle =
    object:
    (object.destroy or false)
    || builtins.elem (object.operation or null) [
      "destroy"
      "close"
    ];
  activeLifecycleAttrs = attrs: lib.filterAttrs (_: object: !isDestroyLifecycle object) attrs;
  activeSwaps = lib.filterAttrs (_: swap: !isDestroyLifecycle swap) cfg.swaps;
  activeLuksDevices = lib.filterAttrs (_: luks: !isDestroyLifecycle luks) cfg.luks.devices;
  activeNfsMounts = lib.filterAttrs (_: mount: !isDestroyLifecycle mount) cfg.nfs.mounts;
  activePhysicalVolumes = activeLifecycleAttrs cfg.physicalVolumes;
  activeVolumeGroups = activeLifecycleAttrs cfg.volumeGroups;
  activeVolumes = activeLifecycleAttrs cfg.volumes;
  activeThinPools = activeLifecycleAttrs cfg.thinPools;
  activeLvmSnapshots = activeLifecycleAttrs cfg.lvmSnapshots;
  activeLvmCaches = activeLifecycleAttrs cfg.lvmCaches;
  activeMdRaids = activeLifecycleAttrs cfg.mdRaids;
  activeMultipathMaps = activeLifecycleAttrs cfg.multipathMaps;
  activeVdoVolumes = activeLifecycleAttrs cfg.vdoVolumes;
  activePools = activeLifecycleAttrs cfg.pools;
  activeDatasets = activeLifecycleAttrs cfg.datasets;
  activeZvols = activeLifecycleAttrs cfg.zvols;
  activeSnapshots = lib.filterAttrs (_: snapshot: !snapshot.destroy) cfg.snapshots;
  activeCaches = activeLifecycleAttrs cfg.caches;
  activeIscsiSessions = activeLifecycleAttrs cfg.iscsi.sessions;
  activeIscsiSessionPortals = lib.filter (portal: portal != null) (
    map (session: session.portal) (lib.attrValues activeIscsiSessions)
  );
  iscsiDiscoverPortal =
    if cfg.iscsi.discoverPortal != null then
      cfg.iscsi.discoverPortal
    else if activeIscsiSessionPortals != [ ] then
      builtins.head activeIscsiSessionPortals
    else
      null;
  hasActiveAttrs = attrs: attrs != { };
  hasActiveLvm =
    hasActiveAttrs activePhysicalVolumes
    || hasActiveAttrs activeVolumeGroups
    || hasActiveAttrs activeVolumes
    || hasActiveAttrs activeThinPools
    || hasActiveAttrs activeLvmSnapshots
    || hasActiveAttrs activeLvmCaches;
  hasActiveLvmThinSupport = hasActiveAttrs activeThinPools || hasActiveAttrs activeLvmCaches;
  hasActiveVdoVolumes = hasActiveAttrs activeVdoVolumes;
  hasActiveMdRaids = hasActiveAttrs activeMdRaids;
  hasActiveMultipathMaps = hasActiveAttrs activeMultipathMaps;
  hasActiveCaches = hasActiveAttrs activeCaches;
  zfsPoolNameFromIdentity =
    identity: builtins.head (lib.splitString "/" (builtins.head (lib.splitString "@" identity)));
  zfsLifecycleIdentities =
    attrs:
    lib.mapAttrsToList (name: object: if object.target != null then object.target else name) attrs;
  zfsExtraPools = lib.unique (
    lib.filter (pool: pool != "" && !(lib.hasPrefix "/" pool)) (
      map zfsPoolNameFromIdentity (
        zfsLifecycleIdentities activePools
        ++ zfsLifecycleIdentities activeDatasets
        ++ zfsLifecycleIdentities activeZvols
        ++ (map (snapshot: snapshot.target) (lib.attrValues activeSnapshots))
      )
    )
  );
  nfsExportLines =
    lib.mapAttrsToList
      (
        name: export:
        let
          exportPath =
            if export.path != null then
              export.path
            else if export.target != null then
              export.target
            else
              name;
        in
        "${exportPath} ${export.client}(${export.options})"
      )
      (
        lib.filterAttrs (
          _: export:
          export.client != null && export.options != null && !export.destroy && export.operation != "destroy"
        ) cfg.exports
      );
  supportedFilesystemTypes = lib.unique (
    (map (filesystem: filesystem.fsType) (lib.attrValues cfg.filesystems))
    ++ (map (mount: mount.fsType) (lib.attrValues activeNfsMounts))
    ++ lib.optional (zfsExtraPools != [ ]) "zfs"
  );
in
{
  options.services.disk-nix = {
    enable = lib.mkEnableOption "disk-nix storage lifecycle integration";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.disk-nix;
      defaultText = lib.literalExpression "inputs.disk-nix.packages.${pkgs.system}.disk-nix";
      description = "disk-nix CLI package used by the NixOS module.";
    };

    toolPackages = lib.mkOption {
      type = lib.types.listOf lib.types.package;
      default = defaultToolPackages;
      defaultText = lib.literalExpression ''
        with pkgs; [
          bcachefs-tools
          bcache-tools
          btrfs-progs
          cryptsetup
          dosfstools
          e2fsprogs
          exfatprogs
          f2fs-tools
          lvm2
          mdadm
          multipath-tools
          nfs-utils
          ntfs3g
          nvme-cli
          openiscsi
          parted
          util-linux
          vdo
          xfsprogs
          zfs
        ]
      '';
      description = ''
        Storage probe and apply tools installed with disk-nix and added to the
        disk-nix apply service PATH. Override this to pin alternate tool
        packages or to trim domains that are not used on a host.
      '';
    };

    spec = lib.mkOption {
      type = json.type;
      default = { };
      description = ''
        Desired storage declaration emitted as JSON for the disk-nix planner.
        This is intentionally broad while the typed NixOS option hierarchy is
        developed.
      '';
    };

    filesystems = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule (
          { name, ... }:
          {
            options = {
              device = lib.mkOption {
                type = lib.types.str;
                description = "Device, mapper path, dataset, or remote source backing the filesystem.";
                example = "/dev/disk/by-uuid/59b8deb7-5fa0-4eb3-b68c-40ac18d4f648";
              };

              fsType = lib.mkOption {
                type = lib.types.str;
                description = "Filesystem type passed to NixOS fileSystems and disk-nix.";
                example = "xfs";
              };

              mountpoint = lib.mkOption {
                type = lib.types.str;
                default = name;
                defaultText = lib.literalExpression "<attribute name>";
                description = "Mountpoint managed by NixOS.";
                example = "/";
              };

              options = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "Mount options passed to NixOS fileSystems.";
                example = [
                  "noatime"
                  "compress=zstd"
                ];
              };

              neededForBoot = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Whether this filesystem is required in the initrd or early boot.";
              };

              operation = lib.mkOption {
                type = operationType;
                default = null;
                description = "Requested filesystem lifecycle operation for disk-nix planning, such as rebalance.";
                example = "rebalance";
              };

              addDevices = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "Devices to add to this filesystem through disk-nix lifecycle planning.";
                example = [ "/dev/disk/by-id/nvme-btrfs-new" ];
              };

              removeDevices = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ ];
                description = "Devices to remove from this filesystem through disk-nix lifecycle planning.";
                example = [ "/dev/disk/by-id/nvme-btrfs-old" ];
              };

              replaceDevices = lib.mkOption {
                type = lib.types.attrsOf lib.types.str;
                default = { };
                description = "Filesystem device replacements from old device path to new device path.";
                example = {
                  "/dev/disk/by-id/nvme-btrfs-old" = "/dev/disk/by-id/nvme-btrfs-new";
                };
              };

              properties = lib.mkOption {
                type = lib.types.attrsOf json.type;
                default = { };
                description = "Filesystem properties to set through disk-nix lifecycle planning.";
                example = {
                  label = "bulk-data";
                };
              };

              resizePolicy = lib.mkOption {
                type = lib.types.enum [
                  "none"
                  "grow-only"
                  "shrink-allowed"
                ];
                default = "none";
                description = "Lifecycle resize policy used by the disk-nix planner.";
              };

              desiredSize = lib.mkOption {
                type = lib.types.nullOr lib.types.str;
                default = null;
                description = "Desired filesystem size for planner and executor advisory commands.";
                example = "100GiB";
              };

              preserveData = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Whether the planner must preserve existing data for this filesystem.";
              };
            };
          }
        )
      );
      default = { };
      description = "Typed filesystem declarations used to generate both disk-nix spec and NixOS fileSystems.";
    };

    swaps = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule (
          { name, ... }:
          {
            options = {
              device = lib.mkOption {
                type = lib.types.str;
                default = name;
                defaultText = lib.literalExpression "<attribute name>";
                description = "Swap device path, by-id path, by-uuid path, or generated mapper path.";
                example = "/dev/disk/by-label/swap";
              };

              priority = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = null;
                description = "Optional swap priority passed to NixOS swapDevices.";
              };

              operation = lib.mkOption {
                type = operationType;
                default = null;
                description = "Requested swap lifecycle operation for disk-nix planning.";
                example = "grow";
              };

              desiredSize = lib.mkOption {
                type = lib.types.nullOr lib.types.str;
                default = null;
                description = "Desired swap size for disk-nix lifecycle planning.";
                example = "16GiB";
              };

              randomEncryption = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Enable NixOS random encryption for this swap device.";
              };

              preserveData = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Whether the planner should treat existing swap signatures as data to preserve.";
              };

              properties = lib.mkOption {
                type = lib.types.attrsOf json.type;
                default = { };
                description = "Swap properties to set through disk-nix lifecycle planning, such as label or swap.uuid.";
                example = {
                  label = "swap";
                  "swap.uuid" = "01234567-89ab-cdef-0123-456789abcdef";
                };
              };
            };
          }
        )
      );
      default = { };
      description = "Typed swap declarations used to generate both disk-nix spec and NixOS swapDevices.";
    };

    luks.devices = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule (
          { name, ... }:
          {
            options = {
              name = lib.mkOption {
                type = lib.types.str;
                default = name;
                defaultText = lib.literalExpression "<attribute name>";
                description = "Mapper name for the opened LUKS device.";
              };

              device = lib.mkOption {
                type = lib.types.str;
                description = "Encrypted block device path.";
                example = "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7";
              };

              allowDiscards = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Enable discard passthrough for this LUKS device.";
              };

              bypassWorkqueues = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Enable cryptsetup workqueue bypass options where supported.";
              };

              preLVM = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Open this device before LVM activation.";
              };

              operation = lib.mkOption {
                type = operationType;
                default = null;
                description = "Requested LUKS lifecycle operation for disk-nix planning.";
                example = "grow";
              };

              desiredSize = lib.mkOption {
                type = lib.types.nullOr lib.types.str;
                default = null;
                description = "Desired opened mapper size for disk-nix lifecycle planning.";
                example = "100%";
              };

              preserveData = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Whether the planner must preserve the existing LUKS container.";
              };

              destroy = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Request LUKS mapper teardown in disk-nix planning without adding the device to boot.initrd.luks.devices.";
              };

              properties = lib.mkOption {
                type = lib.types.attrsOf json.type;
                default = { };
                description = "LUKS header properties to set through disk-nix lifecycle planning, such as label, subsystem, or luks.uuid.";
                example = {
                  label = "cryptroot";
                  "luks.uuid" = "01234567-89ab-cdef-0123-456789abcdef";
                };
              };
            };
          }
        )
      );
      default = { };
      description = "Typed LUKS declarations used to generate both disk-nix spec and boot.initrd.luks.devices.";
    };

    nfs.mounts = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule (
          { name, ... }:
          {
            options = {
              source = lib.mkOption {
                type = lib.types.str;
                description = "NFS source in host:/export form.";
                example = "nas.example.com:/srv/home";
              };

              fsType = lib.mkOption {
                type = lib.types.enum [
                  "nfs"
                  "nfs4"
                ];
                default = "nfs4";
                description = "NFS filesystem type passed to NixOS fileSystems.";
              };

              mountpoint = lib.mkOption {
                type = lib.types.str;
                default = name;
                defaultText = lib.literalExpression "<attribute name>";
                description = "Mountpoint managed by NixOS.";
                example = "/home";
              };

              options = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [
                  "_netdev"
                  "nofail"
                ];
                description = "Mount options passed to NixOS fileSystems.";
                example = [
                  "_netdev"
                  "x-systemd.automount"
                  "vers=4.2"
                ];
              };

              neededForBoot = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Whether this NFS mount is required in the initrd or early boot.";
              };

              operation = lib.mkOption {
                type = operationType;
                default = null;
                description = "Requested NFS client mount lifecycle operation for disk-nix planning.";
                example = "create";
              };

              destroy = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Request unmount/removal of this NFS client mount in disk-nix planning.";
              };

              preserveData = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Whether the planner must preserve remote data for this NFS mount.";
              };
            };
          }
        )
      );
      default = { };
      description = "Typed NFS client mounts used to generate both disk-nix spec and NixOS fileSystems.";
    };

    iscsi = {
      initiatorName = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "iSCSI initiator name used by services.openiscsi and optional boot login.";
        example = "iqn.2026-06.org.example:host";
      };

      discoverPortal = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Portal used by the regular open-iscsi service for target discovery.";
        example = "192.0.2.10:3260";
      };

      enableAutoLoginOut = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable NixOS open-iscsi automatic login/logout for discovered automatic targets.";
      };

      extraConfig = lib.mkOption {
        type = lib.types.lines;
        default = "";
        description = "Extra lines appended to the regular open-iscsi iscsid.conf.";
      };

      sessions = lib.mkOption {
        type = lifecycleAttrs;
        default = { };
        description = "Typed iSCSI session lifecycle declarations emitted into the disk-nix planner spec.";
      };

      boot = {
        enable = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Configure NixOS boot.iscsi-initiator for early-boot iSCSI login.";
        };

        discoverPortal = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Portal used by boot.iscsi-initiator.";
          example = "192.0.2.10:3260";
        };

        target = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "iSCSI target used by boot.iscsi-initiator when loginAll is false.";
          example = "iqn.2026-06.org.example:storage.root";
        };

        loginAll = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Log into all discovered boot iSCSI targets instead of one target.";
        };

        logLevel = lib.mkOption {
          type = lib.types.int;
          default = 1;
          description = "boot.iscsi-initiator log level.";
        };

        extraIscsiCommands = lib.mkOption {
          type = lib.types.lines;
          default = "";
          description = "Extra iscsiadm commands to run in the initrd after login.";
        };

        extraConfig = lib.mkOption {
          type = lib.types.nullOr lib.types.lines;
          default = null;
          description = "Extra lines appended to the initrd iscsid.conf.";
        };
      };
    };

    volumes = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed volume lifecycle declarations emitted into the disk-nix planner spec.";
    };

    disks = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed disk lifecycle declarations emitted into the disk-nix planner spec.";
    };

    partitions = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed partition lifecycle declarations emitted into the disk-nix planner spec.";
    };

    btrfsSubvolumes = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed Btrfs subvolume lifecycle declarations emitted into the disk-nix planner spec.";
    };

    btrfsQgroups = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed Btrfs qgroup lifecycle declarations emitted into the disk-nix planner spec.";
    };

    vdoVolumes = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed VDO volume lifecycle declarations emitted into the disk-nix planner spec.";
    };

    physicalVolumes = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LVM physical-volume lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a concrete block device path.";
    };

    luksKeyslots = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LUKS keyslot lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a LUKS backing device and keyslot or key-file metadata depending on operation.";
    };

    luksTokens = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LUKS token lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a LUKS backing device and token JSON file for imports or a token id for removal.";
    };

    volumeGroups = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed volume-group lifecycle declarations emitted into the disk-nix planner spec.";
    };

    thinPools = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LVM thin-pool lifecycle declarations emitted into the disk-nix planner spec.";
    };

    lvmSnapshots = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LVM snapshot lifecycle declarations emitted into the disk-nix planner spec.";
    };

    lvmCaches = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LVM cache lifecycle declarations emitted into the disk-nix planner spec. Attach plans require a vg/origin target and cache-pool logical volume.";
    };

    loopDevices = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed loop-device lifecycle declarations emitted into the disk-nix planner spec. Refresh and detach command plans require a /dev/loop* target.";
    };

    mdRaids = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed MD RAID lifecycle declarations emitted into the disk-nix planner spec. Executable create, grow, member-add, member-replacement, and member-removal plans require an explicit /dev/md* array target.";
    };

    multipathMaps = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed multipath map lifecycle declarations emitted into the disk-nix planner spec. Executable grow and replacement preflight plans require a concrete mpath* or /dev/mapper/* map target.";
    };

    pools = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed pool lifecycle declarations emitted into the disk-nix planner spec.";
    };

    datasets = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed dataset lifecycle declarations emitted into the disk-nix planner spec.";
    };

    zvols = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed ZFS zvol lifecycle declarations emitted into the disk-nix planner spec.";
    };

    luns = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LUN lifecycle declarations emitted into the disk-nix planner spec.";
    };

    nvmeNamespaces = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed NVMe namespace lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a /dev/nvme* controller target and namespace metadata for attach or delete operations.";
    };

    exports = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed NFS export lifecycle declarations emitted into the disk-nix planner spec. Executable exportfs plans require a local export path plus explicit client and options fields.";
    };

    caches = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed cache-layer lifecycle declarations emitted into the disk-nix planner spec. bcache command plans require a concrete /dev/bcache* target.";
    };

    snapshots = lib.mkOption {
      type = snapshotAttrs;
      default = { };
      description = "Typed snapshot lifecycle declarations emitted into the disk-nix planner spec.";
    };

    apply = {
      mode = lib.mkOption {
        type = lib.types.enum [
          "manual"
          "activation"
          "boot"
          "install"
        ];
        default = "manual";
        description = "When disk-nix may perform imperative storage actions.";
      };

      allowDestructive = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow destructive storage actions such as wipe, format, or destroy.";
      };

      allowFormat = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow formatting filesystems.";
      };

      allowShrink = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow shrink operations.";
      };

      allowPotentialDataLoss = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow explicitly reviewed potential-data-loss actions such as shrink, rollback, and device removal after any configured backup or confirmation gates pass.";
      };

      allowGrow = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Allow non-destructive grow operations.";
      };

      allowOffline = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Allow storage operations that require offline coordination.";
      };

      allowPropertyChanges = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Allow non-destructive storage property changes.";
      };

      allowDeviceReplacement = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Allow device add, replacement, and removal topology changes.";
      };

      allowRebalance = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Allow pool or filesystem rebalance operations.";
      };

      requireBackup = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Require backupVerified=true for destructive or potential-data-loss actions.";
      };

      backupVerified = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Assert that required backups have been verified before policy validation.";
      };

      requireConfirmation = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Require confirmation=true for high-risk or offline actions.";
      };

      confirmation = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Explicit operator confirmation for policies that require it.";
      };

      requireConfirmationFile = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Path to an operator-controlled confirmation file. disk-nix apply confirms it only when the file contains a standalone 'disk-nix confirm' line.";
      };

      probeCurrent = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Probe current topology during disk-nix apply-policy validation.";
      };

      failOnBlocked = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Fail the activation service when policy blocks planned actions. When false, activation uses disk-nix validate so blocked actions are reported without failing the unit.";
      };

      execute = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Run ready, policy-allowed storage commands during activation with disk-nix apply --execute. The default only validates policy and writes review artifacts.";
      };

      scriptOut = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "/run/disk-nix/apply.sh";
        description = "Write the allowed command and verification plan to this reviewable shell script path during apply-policy validation.";
      };

      reportOut = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "/run/disk-nix/apply-report.json";
        description = "Write the JSON apply-policy report to this path during validation, including blocked policy details before failures are returned.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    warnings =
      lib.optional (cfg.apply.mode == "boot")
        "services.disk-nix.apply.mode = \"boot\" is reserved; disk-nix currently wires imperative apply only for mode = \"activation\"."
      ++
        lib.optional (cfg.apply.mode == "install")
          "services.disk-nix.apply.mode = \"install\" is reserved; disk-nix currently wires imperative apply only for mode = \"activation\".";

    environment.systemPackages = [ cfg.package ] ++ cfg.toolPackages;

    environment.etc."disk-nix/spec.json".source = json.generate "disk-nix-spec.json" {
      spec = cfg.spec // {
        filesystems = (cfg.spec.filesystems or { }) // typedFilesystemSpec // typedNfsFilesystemSpec;
        swaps = (cfg.spec.swaps or { }) // typedSwapSpec;
        luks = (cfg.spec.luks or { }) // {
          devices = ((cfg.spec.luks or { }).devices or { }) // typedLuksSpec;
        };
        iscsi = (cfg.spec.iscsi or { }) // typedIscsiSpec;
        iscsiSessions = (cfg.spec.iscsiSessions or { }) // normalizeLifecycleSpec cfg.iscsi.sessions;
        nfs = (cfg.spec.nfs or { }) // {
          mounts = ((cfg.spec.nfs or { }).mounts or { }) // typedNfsMountSpec;
        };
        disks = (cfg.spec.disks or { }) // normalizeLifecycleSpec cfg.disks;
        partitions = (cfg.spec.partitions or { }) // normalizeLifecycleSpec cfg.partitions;
        btrfsSubvolumes = (cfg.spec.btrfsSubvolumes or { }) // normalizeLifecycleSpec cfg.btrfsSubvolumes;
        btrfsQgroups = (cfg.spec.btrfsQgroups or { }) // normalizeLifecycleSpec cfg.btrfsQgroups;
        vdoVolumes = (cfg.spec.vdoVolumes or { }) // normalizeLifecycleSpec cfg.vdoVolumes;
        physicalVolumes = (cfg.spec.physicalVolumes or { }) // normalizeLifecycleSpec cfg.physicalVolumes;
        luksKeyslots = (cfg.spec.luksKeyslots or { }) // normalizeLifecycleSpec cfg.luksKeyslots;
        luksTokens = (cfg.spec.luksTokens or { }) // normalizeLifecycleSpec cfg.luksTokens;
        volumes = (cfg.spec.volumes or { }) // normalizeLifecycleSpec cfg.volumes;
        volumeGroups = (cfg.spec.volumeGroups or { }) // normalizeLifecycleSpec cfg.volumeGroups;
        thinPools = (cfg.spec.thinPools or { }) // normalizeLifecycleSpec cfg.thinPools;
        lvmSnapshots = (cfg.spec.lvmSnapshots or { }) // normalizeLifecycleSpec cfg.lvmSnapshots;
        lvmCaches = (cfg.spec.lvmCaches or { }) // normalizeLifecycleSpec cfg.lvmCaches;
        loopDevices = (cfg.spec.loopDevices or { }) // normalizeLifecycleSpec cfg.loopDevices;
        mdRaids = (cfg.spec.mdRaids or { }) // normalizeLifecycleSpec cfg.mdRaids;
        multipathMaps = (cfg.spec.multipathMaps or { }) // normalizeLifecycleSpec cfg.multipathMaps;
        pools = (cfg.spec.pools or { }) // normalizeLifecycleSpec cfg.pools;
        datasets = (cfg.spec.datasets or { }) // normalizeLifecycleSpec cfg.datasets;
        zvols = (cfg.spec.zvols or { }) // normalizeLifecycleSpec cfg.zvols;
        luns = (cfg.spec.luns or { }) // normalizeLifecycleSpec cfg.luns;
        nvmeNamespaces = (cfg.spec.nvmeNamespaces or { }) // normalizeLifecycleSpec cfg.nvmeNamespaces;
        exports = (cfg.spec.exports or { }) // normalizeLifecycleSpec cfg.exports;
        caches = (cfg.spec.caches or { }) // normalizeLifecycleSpec cfg.caches;
        snapshots = (cfg.spec.snapshots or { }) // normalizeSnapshotSpec cfg.snapshots;
      };
      apply = applyPolicy;
    };

    fileSystems =
      lib.mapAttrs' (_: filesystem: {
        name = filesystem.mountpoint;
        value = filesystemToNixos filesystem;
      }) cfg.filesystems
      // lib.mapAttrs' (_: mount: {
        name = mount.mountpoint;
        value = filesystemToNixos {
          inherit (mount)
            fsType
            neededForBoot
            options
            ;
          device = mount.source;
        };
      }) activeNfsMounts;

    swapDevices = lib.mapAttrsToList (
      _: swap:
      {
        inherit (swap) device;
      }
      // lib.optionalAttrs (swap.priority != null) {
        inherit (swap) priority;
      }
      // lib.optionalAttrs swap.randomEncryption {
        randomEncryption.enable = true;
      }
    ) activeSwaps;

    boot.initrd.luks.devices = lib.mapAttrs (_: luks: {
      inherit (luks)
        device
        preLVM
        allowDiscards
        bypassWorkqueues
        ;
    }) activeLuksDevices;

    boot.supportedFilesystems = supportedFilesystemTypes;

    services.lvm = lib.mkIf (hasActiveLvm || hasActiveVdoVolumes) {
      enable = lib.mkDefault true;
      boot.thin.enable = lib.mkIf hasActiveLvmThinSupport (lib.mkDefault true);
      boot.vdo.enable = lib.mkIf hasActiveVdoVolumes (lib.mkDefault true);
    };

    boot.initrd.services.lvm.enable = lib.mkIf (hasActiveLvm || hasActiveVdoVolumes) (
      lib.mkDefault true
    );

    boot.swraid = lib.mkIf hasActiveMdRaids {
      enable = lib.mkDefault true;
      mdadmConf = lib.mkDefault "PROGRAM ${pkgs.coreutils}/bin/true";
    };

    services.multipath.enable = lib.mkIf hasActiveMultipathMaps (lib.mkDefault true);

    boot.zfs.extraPools = lib.mkIf (zfsExtraPools != [ ]) (lib.mkAfter zfsExtraPools);

    boot.bcache.enable = lib.mkIf hasActiveCaches (lib.mkDefault true);

    boot.initrd.services.bcache.enable = lib.mkIf hasActiveCaches (lib.mkDefault true);

    services.openiscsi = lib.mkIf (cfg.iscsi.initiatorName != null) {
      enable = true;
      name = cfg.iscsi.initiatorName;
      inherit (cfg.iscsi)
        enableAutoLoginOut
        extraConfig
        ;
      discoverPortal = iscsiDiscoverPortal;
    };

    services.nfs.server = lib.mkIf (nfsExportLines != [ ]) {
      enable = lib.mkDefault true;
      exports = lib.mkAfter ("\n" + lib.concatStringsSep "\n" nfsExportLines + "\n");
    };

    boot.iscsi-initiator = lib.mkIf cfg.iscsi.boot.enable {
      name = cfg.iscsi.initiatorName;
      inherit (cfg.iscsi.boot)
        target
        loginAll
        logLevel
        extraIscsiCommands
        extraConfig
        ;
      discoverPortal =
        if cfg.iscsi.boot.discoverPortal != null then
          cfg.iscsi.boot.discoverPortal
        else
          iscsiDiscoverPortal;
    };

    systemd.services.disk-nix-plan = {
      description = "Validate disk-nix storage apply policy";
      wantedBy = lib.mkIf (cfg.apply.mode == "activation") [ "multi-user.target" ];
      path = cfg.toolPackages;
      serviceConfig = {
        Type = "oneshot";
        ExecStart = applyValidationScript;
      };
    };

    assertions = [
      {
        assertion = !(cfg.apply.allowDestructive && cfg.apply.mode == "activation");
        message = "disk-nix refuses destructive activation-mode storage changes.";
      }
      {
        assertion = cfg.apply.execute -> cfg.apply.failOnBlocked;
        message = "services.disk-nix.apply.execute requires services.disk-nix.apply.failOnBlocked=true because disk-nix validate cannot execute storage commands.";
      }
      {
        assertion = cfg.apply.scriptOut != null -> lib.hasPrefix "/" cfg.apply.scriptOut;
        message = "services.disk-nix.apply.scriptOut must be an absolute path.";
      }
      {
        assertion = cfg.apply.reportOut != null -> lib.hasPrefix "/" cfg.apply.reportOut;
        message = "services.disk-nix.apply.reportOut must be an absolute path.";
      }
      {
        assertion = cfg.iscsi.boot.enable -> cfg.iscsi.initiatorName != null;
        message = "services.disk-nix.iscsi.boot.enable requires services.disk-nix.iscsi.initiatorName.";
      }
      {
        assertion =
          cfg.iscsi.boot.enable -> (cfg.iscsi.boot.discoverPortal != null || iscsiDiscoverPortal != null);
        message = "services.disk-nix.iscsi.boot.enable requires services.disk-nix.iscsi.boot.discoverPortal, services.disk-nix.iscsi.discoverPortal, or an active services.disk-nix.iscsi.sessions entry with portal.";
      }
      {
        assertion = cfg.iscsi.boot.enable -> (cfg.iscsi.boot.loginAll || cfg.iscsi.boot.target != null);
        message = "services.disk-nix.iscsi.boot.enable requires a boot target unless loginAll is true.";
      }
    ];
  };
}
