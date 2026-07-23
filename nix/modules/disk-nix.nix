self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.disk-nix;
  packageSystem = pkgs.stdenv.hostPlatform.system;
  json = pkgs.formats.json { };
  applyScriptOutDir = lib.optionalString (cfg.apply.scriptOut != null) (
    builtins.dirOf cfg.apply.scriptOut
  );
  applyReportOutDir = lib.optionalString (cfg.apply.reportOut != null) (
    builtins.dirOf cfg.apply.reportOut
  );
  applyReceiptOutDir = lib.optionalString (cfg.apply.receiptOut != null) (
    builtins.dirOf cfg.apply.receiptOut
  );
  applyCommand = if cfg.apply.failOnBlocked then "apply" else "validate";
  applyPolicy = builtins.removeAttrs cfg.apply [
    "execute"
    "declarativeHandoff"
  ];
  applyRunsAsService =
    cfg.apply.mode == "activation" || cfg.apply.mode == "install" || cfg.apply.mode == "boot";
  applyRunsAtBoot = cfg.apply.mode == "boot";
  defaultToolPackages = with pkgs; [
    bash
    bcachefs-tools
    bcache-tools
    btrfs-progs
    cloud-utils
    coreutils
    cryptsetup
    dosfstools
    e2fsprogs
    exfatprogs
    f2fs-tools
    lvm2
    lsscsi
    mdadm
    multipath-tools
    nfs-utils
    ntfs3g
    nvme-cli
    openiscsi
    parted
    smartmontools
    targetcli-fb
    tgt
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
  ]
  ++ lib.optionals (cfg.apply.receiptOut != null) [
    "--receipt-out"
    cfg.apply.receiptOut
  ];
  applyValidationScript = pkgs.writeShellScript "disk-nix-apply-validation" ''
    set -euo pipefail
    ${lib.optionalString (cfg.apply.scriptOut != null) ''
      mkdir -p ${lib.escapeShellArg applyScriptOutDir}
    ''}
    ${lib.optionalString (cfg.apply.reportOut != null) ''
      mkdir -p ${lib.escapeShellArg applyReportOutDir}
    ''}
    ${lib.optionalString (cfg.apply.receiptOut != null) ''
      mkdir -p ${lib.escapeShellArg applyReceiptOutDir}
    ''}
    ${lib.escapeShellArgs ([ (lib.getExe cfg.package) ] ++ applyArgs)}
    ${lib.optionalString cfg.apply.declarativeHandoff.autoImport.enable ''
      config_path=${lib.escapeShellArg cfg.apply.declarativeHandoff.autoImport.configurationPath}
      backup_dir=${lib.escapeShellArg cfg.apply.declarativeHandoff.autoImport.backupDirectory}
      handoff_module=${lib.escapeShellArg declarativeHandoffNixPath}
      import_patch=${lib.escapeShellArg declarativeHandoffImportPatchPath}
      if [ ! -f "$config_path" ]; then
        printf 'disk-nix declarative handoff auto-import requires existing config: %s\n' "$config_path" >&2
        exit 1
      fi
      if ${pkgs.gnugrep}/bin/grep -F -q "$handoff_module" "$config_path"; then
        printf 'disk-nix declarative handoff module already imported in %s\n' "$config_path" >&2
        exit 0
      fi
      mkdir -p "$backup_dir"
      backup_path="$backup_dir/$(${pkgs.coreutils}/bin/basename "$config_path").$(${pkgs.coreutils}/bin/date -u +%Y%m%dT%H%M%SZ).bak"
      ${pkgs.coreutils}/bin/cp --preserve=mode,ownership,timestamps "$config_path" "$backup_path"
      ${pkgs.patch}/bin/patch --forward --backup --input="$import_patch" "$config_path"
      printf 'disk-nix declarative handoff import patch applied to %s; backup: %s\n' "$config_path" "$backup_path" >&2
    ''}
  '';
  moduleTypes = import ./disk-nix/types.nix { inherit lib json; };
  inherit (moduleTypes)
    operationType
    lifecycleAttrs
    snapshotAttrs
    ;
  cleanSpecAttrs = lib.filterAttrs (_: value: value != null && value != [ ] && value != { });
  normalizeLifecycleSpec = lib.mapAttrs (
    _: object:
    object.metadata
    // cleanSpecAttrs {
      inherit (object)
        operation
        action
        addDevices
        devices
        paths
        devicePaths
        removeDevices
        replaceDevices
        cacheSetUuid
        physicalSize
        renameTo
        renameTarget
        newName
        properties
        destroy
        preserveData
        readOnly
        readonly
        desiredSize
        targetSize
        size
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
        nsid
        controllers
        controllerId
        controller
        keySlot
        slot
        keyFile
        currentKeyFile
        newKeyFile
        tokenId
        token
        tokenFile
        jsonFile
        ;
      "key-slot" = object."key-slot";
      "key-file" = object."key-file";
      "new-key-file" = object."new-key-file";
      "token-id" = object."token-id";
      "token-file" = object."token-file";
    }
  );
  normalizeSnapshotSpec = lib.mapAttrs (
    _: snapshot:
    snapshot.metadata
    // cleanSpecAttrs {
      inherit (snapshot)
        target
        name
        snapshotName
        path
        snapshotPath
        operation
        action
        destroy
        rollback
        cloneTo
        cloneTarget
        clone
        renameTo
        renameTarget
        newName
        recursiveRollback
        recursive
        hold
        holdTag
        releaseHold
        readOnly
        readonly
        preserveData
        ;
      "snapshot-name" = snapshot."snapshot-name";
      "snapshot-path" = snapshot."snapshot-path";
      "zfs.rollbackRecursive" = snapshot."zfs.rollbackRecursive";
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
      action
      destroy
      addDevices
      removeDevices
      replaceDevices
      properties
      metadata
      resizePolicy
      preserveData
      desiredSize
      targetSize
      size
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
      action
      destroy
      preserveData
      metadata
      ;
    device = mount.source;
  }) cfg.nfs.mounts;
  isDestroyLifecycle =
    object:
    let
      requestedOperation =
        if (object.operation or null) != null then object.operation else (object.action or null);
    in
    (object.destroy or false)
    || builtins.elem requestedOperation [
      "destroy"
      "close"
      "deactivate"
      "logout"
      "unmount"
      "unexport"
      "detach"
      "stop"
      "remove-key"
      "remove-token"
    ];
  activeNfsMounts = lib.filterAttrs (_: mount: !isDestroyLifecycle mount) cfg.nfs.mounts;
  activeLuksDevices = lib.filterAttrs (_: luks: !isDestroyLifecycle luks) cfg.luks.devices;
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
      target
      path
      operation
      action
      destroy
      desiredSize
      targetSize
      size
      priority
      randomEncryption
      preserveData
      properties
      ;
  }) cfg.swaps;
  swapDevicePath =
    swap:
    if swap.target != null then
      swap.target
    else if swap.path != null then
      swap.path
    else
      swap.device;
  hasDeclaredZram =
    cfg.zram.enable
    || cfg.zram.operation != null
    || cfg.zram.action != null
    || cfg.zram.swapDevices != 1
    || cfg.zram.memoryPercent != 50
    || cfg.zram.memoryMax != null
    || cfg.zram.priority != 5
    || cfg.zram.algorithm != "zstd"
    || cfg.zram.writebackDevice != null
    || cfg.zram.preserveData != true
    || cfg.zram.properties != { };
  typedZramSpec = lib.optionalAttrs hasDeclaredZram (cleanSpecAttrs {
    inherit (cfg.zram)
      enable
      operation
      action
      swapDevices
      memoryPercent
      memoryMax
      priority
      algorithm
      writebackDevice
      preserveData
      properties
      ;
  });
  typedLuksSpec = lib.mapAttrs (_: luks: {
    inherit (luks)
      device
      name
      target
      mapperName
      mapper
      mapper-name
      operation
      action
      desiredSize
      targetSize
      size
      allowDiscards
      bypassWorkqueues
      preLVM
      preserveData
      destroy
      properties
      ;
  }) cfg.luks.devices;
  luksMapperName =
    _attrName: luks:
    if luks.target != null then
      luks.target
    else if luks.mapperName != null then
      luks.mapperName
    else if luks.mapper-name != null then
      luks.mapper-name
    else if luks.mapper != null then
      luks.mapper
    else
      luks.name;
  activeLuksMapperNames = lib.mapAttrsToList luksMapperName activeLuksDevices;
  luksDeviceConfig = luks: {
    inherit (luks)
      device
      preLVM
      allowDiscards
      bypassWorkqueues
      ;
  };
  activeLuksDeviceConfig = builtins.listToAttrs (
    lib.mapAttrsToList (name: luks: {
      name = luksMapperName name luks;
      value = luksDeviceConfig luks;
    }) activeLuksDevices
  );
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
  runtimeState = import ./disk-nix/runtime-state.nix {
    inherit
      cfg
      lib
      pkgs
      cleanSpecAttrs
      swapDevicePath
      activeLuksDeviceConfig
      ;
  };
  inherit (runtimeState)
    activeFilesystemMountpoints
    activeSwapDevicePaths
    activeDiskPaths
    activePartitionIdentities
    activeNvmeNamespaceIdentities
    activeLuksKeyslotIdentities
    activeLuksTokenIdentities
    activeBackingFilePaths
    activeBtrfsSubvolumePaths
    activeBtrfsQgroupSelectors
    activeDmMapTargets
    activeMdRaidTargets
    activeMultipathMapIdentities
    activePoolIdentities
    activeDatasetIdentities
    activeZvolIdentities
    activeVolumeGroupIdentities
    activeVolumeIdentities
    activeThinPoolIdentities
    activeLvmCacheIdentities
    activeVdoVolumeIdentities
    activePhysicalVolumePaths
    activeLoopDeviceTargets
    activeSnapshotIdentities
    activeIscsiSessionIdentities
    activeLunHostPaths
    activeNfsExportSelectors
    activeCacheIdentities
    nativeFileSystems
    nativeSwapDevices
    nativeZramSwap
    nativeOpenIscsi
    nativeBootIscsi
    supportedFilesystemTypes
    hasActiveLvm
    hasActiveLvmThinSupport
    hasActiveVdoVolumes
    hasActiveMdRaids
    hasActiveMultipathMaps
    hasActiveCaches
    zfsExtraPools
    nfsExportLines
    iscsiDiscoverPortal
    declarativeHandoffNixPath
    declarativeHandoffImportPatchPath
    declarativeHandoffNix
    declarativeHandoffImportPatch
    steadyState
    ;
in
{
  options.services.disk-nix = import ./disk-nix/options.nix {
    inherit
      self
      lib
      pkgs
      packageSystem
      json
      operationType
      lifecycleAttrs
      snapshotAttrs
      defaultToolPackages
      ;
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ] ++ cfg.toolPackages;

    environment.etc."disk-nix/spec.json".source = json.generate "disk-nix-spec.json" {
      version = 1;
      spec = cfg.spec // {
        filesystems = (cfg.spec.filesystems or { }) // typedFilesystemSpec // typedNfsFilesystemSpec;
        swaps = (cfg.spec.swaps or { }) // typedSwapSpec;
        zram = (cfg.spec.zram or { }) // typedZramSpec;
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
        backingFiles = (cfg.spec.backingFiles or { }) // normalizeLifecycleSpec cfg.backingFiles;
        dmMaps = (cfg.spec.dmMaps or { }) // normalizeLifecycleSpec cfg.dmMaps;
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

    environment.etc."disk-nix/steady-state.json".source =
      json.generate "disk-nix-steady-state.json" steadyState;

    environment.etc."disk-nix/declarative-handoff.nix".source = declarativeHandoffNix;

    environment.etc."disk-nix/declarative-handoff-import.patch".source = declarativeHandoffImportPatch;

    fileSystems = nativeFileSystems;

    swapDevices = nativeSwapDevices;

    zramSwap = lib.mkIf cfg.zram.enable nativeZramSwap;

    boot.initrd.luks.devices = activeLuksDeviceConfig;

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

    boot.zfs = lib.mkIf (zfsExtraPools != [ ]) {
      extraPools = lib.mkAfter zfsExtraPools;
      forceImportRoot = lib.mkDefault false;
    };

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
      wantedBy = lib.mkIf applyRunsAsService [ "multi-user.target" ];
      wants = lib.mkIf applyRunsAtBoot [ "systemd-udev-settle.service" ];
      after = lib.mkIf applyRunsAtBoot [
        "local-fs.target"
        "systemd-udev-settle.service"
      ];
      before = lib.mkIf applyRunsAtBoot [ "multi-user.target" ];
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
        assertion = cfg.apply.declarativeHandoff.autoImport.enable -> cfg.apply.execute;
        message = "services.disk-nix.apply.declarativeHandoff.autoImport.enable requires services.disk-nix.apply.execute=true so the declarative handoff import is only attempted after successful imperative mutation.";
      }
      {
        assertion = cfg.apply.declarativeHandoff.autoImport.enable -> cfg.apply.mode != "manual";
        message = "services.disk-nix.apply.declarativeHandoff.autoImport.enable requires services.disk-nix.apply.mode to run as a service.";
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
        assertion = cfg.apply.receiptOut != null -> lib.hasPrefix "/" cfg.apply.receiptOut;
        message = "services.disk-nix.apply.receiptOut must be an absolute path.";
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
      {
        assertion = cfg.zram.writebackDevice == null || cfg.zram.swapDevices <= 1;
        message = "services.disk-nix.zram.writebackDevice cannot be shared by multiple zram swap devices.";
      }
      {
        assertion = lib.length activeLuksMapperNames == lib.length (lib.unique activeLuksMapperNames);
        message = "services.disk-nix.luks.devices entries must resolve to unique mapper names.";
      }
      {
        assertion =
          lib.length activeLuksKeyslotIdentities == lib.length (lib.unique activeLuksKeyslotIdentities);
        message = "services.disk-nix.luksKeyslots entries must resolve to unique active concrete device/keyslot selectors.";
      }
      {
        assertion =
          lib.length activeLuksTokenIdentities == lib.length (lib.unique activeLuksTokenIdentities);
        message = "services.disk-nix.luksTokens entries must resolve to unique active concrete device/token selectors.";
      }
      {
        assertion =
          lib.length activeFilesystemMountpoints == lib.length (lib.unique activeFilesystemMountpoints);
        message = "services.disk-nix.filesystems and services.disk-nix.nfs.mounts entries must resolve to unique active mountpoints.";
      }
      {
        assertion = lib.length activeSwapDevicePaths == lib.length (lib.unique activeSwapDevicePaths);
        message = "services.disk-nix.swaps entries must resolve to unique active swap device paths.";
      }
      {
        assertion = lib.length activeDiskPaths == lib.length (lib.unique activeDiskPaths);
        message = "services.disk-nix.disks entries must resolve to unique active concrete device paths.";
      }
      {
        assertion =
          lib.length activePartitionIdentities == lib.length (lib.unique activePartitionIdentities);
        message = "services.disk-nix.partitions entries must resolve to unique active concrete partition selectors.";
      }
      {
        assertion =
          lib.length activeBtrfsSubvolumePaths == lib.length (lib.unique activeBtrfsSubvolumePaths);
        message = "services.disk-nix.btrfsSubvolumes entries must resolve to unique active concrete subvolume paths.";
      }
      {
        assertion =
          lib.length activeBtrfsQgroupSelectors == lib.length (lib.unique activeBtrfsQgroupSelectors);
        message = "services.disk-nix.btrfsQgroups entries must resolve to unique active qgroup/filesystem selectors.";
      }
      {
        assertion = lib.length activeBackingFilePaths == lib.length (lib.unique activeBackingFilePaths);
        message = "services.disk-nix.backingFiles entries must resolve to unique active backing file paths.";
      }
      {
        assertion = lib.length activeDmMapTargets == lib.length (lib.unique activeDmMapTargets);
        message = "services.disk-nix.dmMaps entries must resolve to unique active /dev/mapper/* or /dev/dm-* targets.";
      }
      {
        assertion = lib.length activeMdRaidTargets == lib.length (lib.unique activeMdRaidTargets);
        message = "services.disk-nix.mdRaids entries must resolve to unique active concrete /dev/md* array targets.";
      }
      {
        assertion =
          lib.length activeMultipathMapIdentities == lib.length (lib.unique activeMultipathMapIdentities);
        message = "services.disk-nix.multipathMaps entries must resolve to unique active concrete map identities.";
      }
      {
        assertion = lib.length activePoolIdentities == lib.length (lib.unique activePoolIdentities);
        message = "services.disk-nix.pools entries must resolve to unique active concrete pool identities.";
      }
      {
        assertion = lib.length activeDatasetIdentities == lib.length (lib.unique activeDatasetIdentities);
        message = "services.disk-nix.datasets entries must resolve to unique active concrete dataset identities.";
      }
      {
        assertion = lib.length activeZvolIdentities == lib.length (lib.unique activeZvolIdentities);
        message = "services.disk-nix.zvols entries must resolve to unique active concrete zvol identities.";
      }
      {
        assertion =
          lib.length activeVolumeGroupIdentities == lib.length (lib.unique activeVolumeGroupIdentities);
        message = "services.disk-nix.volumeGroups entries must resolve to unique active concrete volume-group identities.";
      }
      {
        assertion = lib.length activeVolumeIdentities == lib.length (lib.unique activeVolumeIdentities);
        message = "services.disk-nix.volumes entries must resolve to unique active concrete logical-volume identities.";
      }
      {
        assertion = lib.length activeThinPoolIdentities == lib.length (lib.unique activeThinPoolIdentities);
        message = "services.disk-nix.thinPools entries must resolve to unique active concrete thin-pool identities.";
      }
      {
        assertion = lib.length activeLvmCacheIdentities == lib.length (lib.unique activeLvmCacheIdentities);
        message = "services.disk-nix.lvmCaches entries must resolve to unique active concrete cache identities.";
      }
      {
        assertion = lib.length activeCacheIdentities == lib.length (lib.unique activeCacheIdentities);
        message = "services.disk-nix.caches entries must resolve to unique active concrete cache identities.";
      }
      {
        assertion =
          lib.length activeVdoVolumeIdentities == lib.length (lib.unique activeVdoVolumeIdentities);
        message = "services.disk-nix.vdoVolumes entries must resolve to unique active concrete VDO identities.";
      }
      {
        assertion =
          lib.length activePhysicalVolumePaths == lib.length (lib.unique activePhysicalVolumePaths);
        message = "services.disk-nix.physicalVolumes entries must resolve to unique active concrete device paths.";
      }
      {
        assertion = lib.length activeLoopDeviceTargets == lib.length (lib.unique activeLoopDeviceTargets);
        message = "services.disk-nix.loopDevices entries must resolve to unique active concrete /dev/loop* targets.";
      }
      {
        assertion = lib.length activeSnapshotIdentities == lib.length (lib.unique activeSnapshotIdentities);
        message = "services.disk-nix.snapshots entries must resolve to unique active concrete snapshot identities.";
      }
      {
        assertion =
          lib.length activeIscsiSessionIdentities == lib.length (lib.unique activeIscsiSessionIdentities);
        message = "services.disk-nix.iscsi.sessions entries must resolve to unique active concrete target identities.";
      }
      {
        assertion =
          lib.length activeNvmeNamespaceIdentities == lib.length (lib.unique activeNvmeNamespaceIdentities);
        message = "services.disk-nix.nvmeNamespaces entries must resolve to unique active concrete controller/namespace selectors.";
      }
      {
        assertion = lib.length activeLunHostPaths == lib.length (lib.unique activeLunHostPaths);
        message = "services.disk-nix.luns entries must resolve to unique active concrete host paths.";
      }
      {
        assertion = lib.length activeNfsExportSelectors == lib.length (lib.unique activeNfsExportSelectors);
        message = "services.disk-nix.exports entries must resolve to unique active export path/client pairs.";
      }
    ];
  };
}
