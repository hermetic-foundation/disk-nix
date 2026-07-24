{
  cfg,
  lib,
  pkgs,
  cleanSpecAttrs,
  swapDevicePath,
  activeLuksDeviceConfig,
}:

let
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
  lifecycle = import ./lifecycle.nix { inherit lib cleanSpecAttrs; };
  inherit (lifecycle)
    activeLifecycleAttrs
    isDestroyLifecycle
    isExportLifecycle
    lifecycleIdentity
    lifecycleManagedMap
    lifecyclePathTarget
    ;
  activeFilesystems = activeLifecycleAttrs cfg.filesystems;
  activeSwaps = lib.filterAttrs (_: swap: !isDestroyLifecycle swap) cfg.swaps;
  activeLuksDevices = lib.filterAttrs (_: luks: !isDestroyLifecycle luks) cfg.luks.devices;
  activeNfsMounts = lib.filterAttrs (_: mount: !isDestroyLifecycle mount) cfg.nfs.mounts;
  activeBtrfsSubvolumes = activeLifecycleAttrs cfg.btrfsSubvolumes;
  activeBtrfsQgroups = activeLifecycleAttrs cfg.btrfsQgroups;
  activePhysicalVolumes = activeLifecycleAttrs cfg.physicalVolumes;
  activeVolumeGroups = lib.filterAttrs (
    _: object: !isDestroyLifecycle object && !isExportLifecycle object
  ) cfg.volumeGroups;
  activeVolumes = activeLifecycleAttrs cfg.volumes;
  activeThinPools = activeLifecycleAttrs cfg.thinPools;
  activeLvmSnapshots = activeLifecycleAttrs cfg.lvmSnapshots;
  activeLvmCaches = activeLifecycleAttrs cfg.lvmCaches;
  activeLuksKeyslots = activeLifecycleAttrs cfg.luksKeyslots;
  activeLuksTokens = activeLifecycleAttrs cfg.luksTokens;
  activeLoopDevices = activeLifecycleAttrs cfg.loopDevices;
  activeBackingFiles = activeLifecycleAttrs cfg.backingFiles;
  activeDmMaps = activeLifecycleAttrs cfg.dmMaps;
  activeMdRaids = activeLifecycleAttrs cfg.mdRaids;
  activeMultipathMaps = activeLifecycleAttrs cfg.multipathMaps;
  activeVdoVolumes = activeLifecycleAttrs cfg.vdoVolumes;
  activePools = lib.filterAttrs (
    _: object: !isDestroyLifecycle object && !isExportLifecycle object
  ) cfg.pools;
  activeDatasets = activeLifecycleAttrs cfg.datasets;
  activeZvols = activeLifecycleAttrs cfg.zvols;
  activeSnapshots = lib.filterAttrs (_: snapshot: !snapshot.destroy) cfg.snapshots;
  activeCaches = activeLifecycleAttrs cfg.caches;
  activeLuns = activeLifecycleAttrs cfg.luns;
  activeIscsiSessions = activeLifecycleAttrs cfg.iscsi.sessions;
  activeIscsiSessionPortals = lib.filter (portal: portal != null) (
    map (session: session.portal) (lib.attrValues activeIscsiSessions)
  );
  activeFilesystemMountpoints =
    (map (filesystem: filesystem.mountpoint) (lib.attrValues activeFilesystems))
    ++ (map (mount: mount.mountpoint) (lib.attrValues activeNfsMounts));
  activeSwapDevicePaths = map swapDevicePath (lib.attrValues activeSwaps);
  activeDisks = activeLifecycleAttrs cfg.disks;
  activePartitions = activeLifecycleAttrs cfg.partitions;
  activeNvmeNamespaces = activeLifecycleAttrs cfg.nvmeNamespaces;
  pathLike = path: lib.hasPrefix "/dev/" path;
  partitionNumber =
    partition:
    if partition.partitionNumber != null then partition.partitionNumber else partition.number;
  partitionIdentity =
    name: partition:
    let
      path = lifecyclePathTarget name partition;
      number = partitionNumber partition;
    in
    if path != null && pathLike path && path != partition.device then
      path
    else if partition.device != null && number != null then
      "${partition.device}#${number}"
    else
      null;
  nvmeNamespaceController =
    name: namespace:
    if namespace.target != null then
      namespace.target
    else if namespace.path != null then
      namespace.path
    else if namespace.device != null then
      namespace.device
    else if lib.hasPrefix "/dev/nvme" name then
      name
    else
      null;
  nvmeNamespaceId =
    namespace: if namespace.namespaceId != null then namespace.namespaceId else namespace.nsid;
  nvmeNamespaceIdentity =
    name: namespace:
    let
      controller = nvmeNamespaceController name namespace;
      nsid = nvmeNamespaceId namespace;
    in
    if controller != null && nsid != null then "${controller} nsid ${nsid}" else null;
  activeDiskPaths = lib.filter (path: path != null && pathLike path) (
    lib.mapAttrsToList lifecyclePathTarget activeDisks
  );
  activePartitionIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList partitionIdentity activePartitions
  );
  activeNvmeNamespaceIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList nvmeNamespaceIdentity activeNvmeNamespaces
  );
  activeCacheIdentities = lib.mapAttrsToList lifecycleIdentity activeCaches;
  numericString = value: builtins.match "[0-9]+" value != null;
  numericNameSuffix =
    name:
    let
      suffix = lib.last (lib.splitString ":" name);
    in
    if numericString suffix then suffix else null;
  luksHeaderDevice =
    name: object:
    if object.device != null then
      object.device
    else if object.target != null && lib.hasPrefix "/" object.target then
      object.target
    else if lib.hasPrefix "/dev/" name then
      name
    else
      null;
  luksKeyslotId =
    name: keyslot:
    if keyslot.keySlot != null then
      keyslot.keySlot
    else if keyslot."key-slot" != null then
      keyslot."key-slot"
    else if keyslot.slot != null then
      keyslot.slot
    else
      numericNameSuffix name;
  luksTokenId =
    name: token:
    if token.tokenId != null then
      token.tokenId
    else if token."token-id" != null then
      token."token-id"
    else if token.token != null then
      token.token
    else
      numericNameSuffix name;
  luksKeyslotIdentity =
    name: keyslot:
    let
      device = luksHeaderDevice name keyslot;
      slot = luksKeyslotId name keyslot;
    in
    if device != null && slot != null then "${device} keyslot ${slot}" else null;
  luksTokenIdentity =
    name: token:
    let
      device = luksHeaderDevice name token;
      tokenId = luksTokenId name token;
    in
    if device != null && tokenId != null then "${device} token ${tokenId}" else null;
  activeLuksKeyslotIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList luksKeyslotIdentity activeLuksKeyslots
  );
  activeLuksTokenIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList luksTokenIdentity activeLuksTokens
  );
  activeBackingFilePaths = lib.filter (path: path != null) (
    lib.mapAttrsToList lifecyclePathTarget activeBackingFiles
  );
  activeBtrfsSubvolumePaths = lib.filter (path: path != null) (
    lib.mapAttrsToList lifecyclePathTarget activeBtrfsSubvolumes
  );
  btrfsQgroupSelector =
    name: qgroup:
    let
      qgroupId =
        if qgroup.target != null && !(lib.hasPrefix "/" qgroup.target) then qgroup.target else name;
      filesystemPath =
        if qgroup.path != null then
          qgroup.path
        else if qgroup.mountpoint != null then
          qgroup.mountpoint
        else if qgroup.target != null && lib.hasPrefix "/" qgroup.target then
          qgroup.target
        else
          null;
    in
    if filesystemPath != null then "${qgroupId} ${filesystemPath}" else null;
  activeBtrfsQgroupSelectors = lib.filter (selector: selector != null) (
    lib.mapAttrsToList btrfsQgroupSelector activeBtrfsQgroups
  );
  isDmMapTarget = target: lib.hasPrefix "/dev/mapper/" target || lib.hasPrefix "/dev/dm-" target;
  activeDmMapTargets = lib.filter (target: target != null && isDmMapTarget target) (
    lib.mapAttrsToList lifecyclePathTarget activeDmMaps
  );
  isMdRaidTarget = target: lib.hasPrefix "/dev/md" target;
  activeMdRaidTargets = lib.filter (target: target != null && isMdRaidTarget target) (
    lib.mapAttrsToList lifecyclePathTarget activeMdRaids
  );
  multipathMapIdentity =
    name: map:
    if map.target != null then
      map.target
    else if map.path != null then
      map.path
    else if map.device != null then
      map.device
    else if lib.hasPrefix "mpath" name || lib.hasPrefix "/dev/mapper/" name then
      name
    else
      null;
  activeMultipathMapIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList multipathMapIdentity activeMultipathMaps
  );
  activePoolIdentities = lib.mapAttrsToList lifecycleIdentity activePools;
  activeDatasetIdentities = lib.mapAttrsToList lifecycleIdentity activeDatasets;
  activeZvolIdentities = lib.mapAttrsToList lifecycleIdentity activeZvols;
  activeVolumeGroupIdentities = lib.mapAttrsToList lifecycleIdentity activeVolumeGroups;
  activeVolumeIdentities = lib.mapAttrsToList lifecycleIdentity activeVolumes;
  activeThinPoolIdentities = lib.mapAttrsToList lifecycleIdentity activeThinPools;
  activeLvmCacheIdentities = lib.mapAttrsToList lifecycleIdentity activeLvmCaches;
  activeVdoVolumeIdentities = lib.mapAttrsToList lifecycleIdentity activeVdoVolumes;
  activePhysicalVolumePaths = lib.filter (path: path != null && pathLike path) (
    lib.mapAttrsToList lifecyclePathTarget activePhysicalVolumes
  );
  activeLoopDeviceTargets = lib.filter (path: path != null && lib.hasPrefix "/dev/loop" path) (
    lib.mapAttrsToList lifecyclePathTarget activeLoopDevices
  );
  snapshotIdentity =
    name: snapshot:
    if snapshot.name != null then
      snapshot.name
    else if snapshot.snapshotName != null then
      snapshot.snapshotName
    else if snapshot."snapshot-name" != null then
      snapshot."snapshot-name"
    else if snapshot.path != null then
      snapshot.path
    else if snapshot.snapshotPath != null then
      snapshot.snapshotPath
    else if snapshot."snapshot-path" != null then
      snapshot."snapshot-path"
    else if lib.hasPrefix "/" name || lib.hasInfix "@" name then
      name
    else
      null;
  activeSnapshotIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList snapshotIdentity activeSnapshots
  );
  iscsiSessionIdentity =
    name: session:
    if session.target != null then
      session.target
    else if lib.hasPrefix "iqn." name || lib.hasPrefix "eui." name || lib.hasPrefix "naa." name then
      name
    else
      null;
  activeIscsiSessionIdentities = lib.filter (identity: identity != null) (
    lib.mapAttrsToList iscsiSessionIdentity activeIscsiSessions
  );
  lunHostPaths =
    name: lun:
    lib.filter pathLike (
      lib.optional (pathLike name) name
      ++ lib.optional (lun.target != null && pathLike lun.target) lun.target
      ++ lib.optional (lun.path != null && pathLike lun.path) lun.path
      ++ lib.optional (lun.device != null && pathLike lun.device) lun.device
      ++ lun.devices
      ++ lun.paths
      ++ lun.devicePaths
    );
  activeLunHostPaths = lib.concatLists (lib.mapAttrsToList lunHostPaths activeLuns);
  primaryLunHostPath =
    name: lun:
    let
      paths = lunHostPaths name lun;
    in
    if paths != [ ] then builtins.head paths else lifecycleIdentity name lun;
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
  zfsExtraPools = lib.unique (
    lib.filter (pool: pool != "" && !(lib.hasPrefix "/" pool)) (
      map zfsPoolNameFromIdentity (
        activePoolIdentities
        ++ activeDatasetIdentities
        ++ activeZvolIdentities
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
          _: export: export.client != null && export.options != null && !isDestroyLifecycle export
        ) cfg.exports
      );
  activeNfsExportSelectors = lib.mapAttrsToList (
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
    "${exportPath} ${export.client}"
  ) (lib.filterAttrs (_: export: export.client != null && !isDestroyLifecycle export) cfg.exports);
  nfsExportSelector =
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
    if export.client != null then "${exportPath} ${export.client}" else null;
  supportedFilesystemTypes = lib.unique (
    (map (filesystem: filesystem.fsType) (lib.attrValues activeFilesystems))
    ++ (map (mount: mount.fsType) (lib.attrValues activeNfsMounts))
    ++ lib.optional (zfsExtraPools != [ ]) "zfs"
  );
  nativeFileSystems =
    lib.mapAttrs' (_: filesystem: {
      name = filesystem.mountpoint;
      value = filesystemToNixos filesystem;
    }) activeFilesystems
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
  nativeSwapDevices = lib.mapAttrsToList (
    _: swap:
    {
      device = swapDevicePath swap;
    }
    // lib.optionalAttrs (swap.priority != null) {
      inherit (swap) priority;
    }
    // lib.optionalAttrs swap.randomEncryption {
      randomEncryption.enable = true;
    }
  ) activeSwaps;
  nativeZramSwap = {
    enable = true;
    inherit (cfg.zram)
      swapDevices
      memoryPercent
      priority
      algorithm
      ;
  }
  // lib.optionalAttrs (cfg.zram.memoryMax != null) {
    inherit (cfg.zram) memoryMax;
  }
  // lib.optionalAttrs (cfg.zram.writebackDevice != null) {
    inherit (cfg.zram) writebackDevice;
  };
  nativeOpenIscsi = cleanSpecAttrs {
    enable = cfg.iscsi.initiatorName != null;
    name = cfg.iscsi.initiatorName;
    inherit (cfg.iscsi)
      enableAutoLoginOut
      extraConfig
      ;
    discoverPortal = iscsiDiscoverPortal;
  };
  nativeBootIscsi = cleanSpecAttrs {
    enable = cfg.iscsi.boot.enable;
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
  declarativeHandoffNixPath = "/etc/disk-nix/declarative-handoff.nix";
  declarativeHandoffImportPatchPath = "/etc/disk-nix/declarative-handoff-import.patch";
  generatedArtifactPaths = [
    "/etc/disk-nix/spec.json"
    "/etc/disk-nix/steady-state.json"
    declarativeHandoffNixPath
    declarativeHandoffImportPatchPath
  ]
  ++ lib.optional (cfg.apply.scriptOut != null) cfg.apply.scriptOut
  ++ lib.optional (cfg.apply.reportOut != null) cfg.apply.reportOut
  ++ lib.optional (cfg.apply.receiptOut != null) cfg.apply.receiptOut;
  declarativeHandoffModule = {
    fileSystems = nativeFileSystems;
    swapDevices = nativeSwapDevices;
    zramSwap = lib.optionalAttrs cfg.zram.enable nativeZramSwap;
    boot = {
      initrd = {
        luks.devices = activeLuksDeviceConfig;
        network.openiscsi = nativeBootIscsi;
        services.lvm.enable = hasActiveLvm || hasActiveVdoVolumes;
      };
      supportedFilesystems = supportedFilesystemTypes;
      zfs = {
        extraPools = zfsExtraPools;
        forceImportRoot = false;
      };
      swraid = {
        enable = hasActiveMdRaids;
        mdadmConf = lib.optionalString hasActiveMdRaids "PROGRAM ${pkgs.coreutils}/bin/true";
      };
    };
    services = {
      lvm = {
        enable = hasActiveLvm || hasActiveVdoVolumes;
        boot.thin.enable = hasActiveLvmThinSupport;
        boot.vdo.enable = hasActiveVdoVolumes;
      };
      multipath.enable = hasActiveMultipathMaps;
      nfs.server = {
        enable = nfsExportLines != [ ];
        exports = lib.concatStringsSep "\n" nfsExportLines;
      };
      openiscsi = nativeOpenIscsi;
    };
  };
  declarativeHandoffNix = pkgs.writeText "disk-nix-declarative-handoff.nix" ''
    # Generated by services.disk-nix.
    # Review this file after successful imperative disk-nix mutations, then copy
    # the relevant declarations into your real NixOS configuration.
    # This file is not imported by default.
    { config, lib, pkgs, ... }:
    ${lib.generators.toPretty { } declarativeHandoffModule}
  '';
  declarativeHandoffImportPatch = pkgs.writeText "disk-nix-declarative-handoff-import.patch" ''
    # Generated by services.disk-nix.
    # Review before applying. This patch is intentionally not applied by default
    # because importing the handoff module changes the declarative storage state
    # that NixOS will enforce.
    --- a/configuration.nix
    +++ b/configuration.nix
    @@
     {
    +  imports = [
    +    ${declarativeHandoffNixPath}
    +  ];
    +
     }
  '';
  steadyState = {
    version = 1;
    fileSystems = nativeFileSystems;
    swapDevices = nativeSwapDevices;
    zramSwap = lib.optionalAttrs cfg.zram.enable nativeZramSwap;
    luksDevices = activeLuksDeviceConfig;
    supportedFilesystems = supportedFilesystemTypes;
    nfsExports = nfsExportLines;
    storageIdentities = {
      filesystemMountpoints = activeFilesystemMountpoints;
      swapDevices = activeSwapDevicePaths;
      disks = activeDiskPaths;
      partitions = activePartitionIdentities;
      physicalVolumes = activePhysicalVolumePaths;
      volumeGroups = activeVolumeGroupIdentities;
      volumes = activeVolumeIdentities;
      thinPools = activeThinPoolIdentities;
      lvmCaches = activeLvmCacheIdentities;
      vdoVolumes = activeVdoVolumeIdentities;
      luksKeyslots = activeLuksKeyslotIdentities;
      luksTokens = activeLuksTokenIdentities;
      backingFiles = activeBackingFilePaths;
      loopDevices = activeLoopDeviceTargets;
      dmMaps = activeDmMapTargets;
      mdRaids = activeMdRaidTargets;
      multipathMaps = activeMultipathMapIdentities;
      pools = activePoolIdentities;
      datasets = activeDatasetIdentities;
      zvols = activeZvolIdentities;
      btrfsSubvolumes = activeBtrfsSubvolumePaths;
      btrfsQgroups = activeBtrfsQgroupSelectors;
      snapshots = activeSnapshotIdentities;
      caches = activeCacheIdentities;
      nvmeNamespaces = activeNvmeNamespaceIdentities;
    };
    networkStorage = {
      iscsiSessionTargets = activeIscsiSessionIdentities;
      lunHostPaths = activeLunHostPaths;
      nfsExportSelectors = activeNfsExportSelectors;
    };
    lifecycleManaged = {
      filesystems = lifecycleManagedMap activeFilesystems (_: filesystem: filesystem.mountpoint);
      swapDevices = lifecycleManagedMap activeSwaps (_: swap: swapDevicePath swap);
      luksDevices = lifecycleManagedMap activeLuksDevices (name: _: name);
      nfsMounts = lifecycleManagedMap activeNfsMounts (_: mount: mount.mountpoint);
      disks = lifecycleManagedMap activeDisks lifecyclePathTarget;
      partitions = lifecycleManagedMap activePartitions partitionIdentity;
      nvmeNamespaces = lifecycleManagedMap activeNvmeNamespaces nvmeNamespaceIdentity;
      physicalVolumes = lifecycleManagedMap activePhysicalVolumes lifecyclePathTarget;
      volumeGroups = lifecycleManagedMap activeVolumeGroups lifecycleIdentity;
      volumes = lifecycleManagedMap activeVolumes lifecycleIdentity;
      thinPools = lifecycleManagedMap activeThinPools lifecycleIdentity;
      lvmSnapshots = lifecycleManagedMap activeLvmSnapshots lifecycleIdentity;
      lvmCaches = lifecycleManagedMap activeLvmCaches lifecycleIdentity;
      luksKeyslots = lifecycleManagedMap activeLuksKeyslots luksKeyslotIdentity;
      luksTokens = lifecycleManagedMap activeLuksTokens luksTokenIdentity;
      backingFiles = lifecycleManagedMap activeBackingFiles lifecyclePathTarget;
      loopDevices = lifecycleManagedMap activeLoopDevices lifecyclePathTarget;
      dmMaps = lifecycleManagedMap activeDmMaps lifecyclePathTarget;
      mdRaids = lifecycleManagedMap activeMdRaids lifecyclePathTarget;
      multipathMaps = lifecycleManagedMap activeMultipathMaps multipathMapIdentity;
      vdoVolumes = lifecycleManagedMap activeVdoVolumes lifecycleIdentity;
      pools = lifecycleManagedMap activePools lifecycleIdentity;
      datasets = lifecycleManagedMap activeDatasets lifecycleIdentity;
      zvols = lifecycleManagedMap activeZvols lifecycleIdentity;
      btrfsSubvolumes = lifecycleManagedMap activeBtrfsSubvolumes lifecyclePathTarget;
      btrfsQgroups = lifecycleManagedMap activeBtrfsQgroups btrfsQgroupSelector;
      snapshots = lifecycleManagedMap activeSnapshots snapshotIdentity;
      caches = lifecycleManagedMap activeCaches lifecycleIdentity;
      luns = lifecycleManagedMap activeLuns primaryLunHostPath;
      iscsiSessions = lifecycleManagedMap activeIscsiSessions iscsiSessionIdentity;
      nfsExports = lifecycleManagedMap (lib.filterAttrs (
        _: export: export.client != null && !isDestroyLifecycle export
      ) cfg.exports) nfsExportSelector;
    };
    iscsi = {
      openiscsi = nativeOpenIscsi;
      bootInitiator = nativeBootIscsi;
    };
    declarativeHandoff = {
      fileSystems = lib.attrNames nativeFileSystems;
      swapDevices = activeSwapDevicePaths;
      luksDevices = lib.attrNames activeLuksDeviceConfig;
      nfsExports = activeNfsExportSelectors;
      iscsiSessions = activeIscsiSessionIdentities;
      iscsiBoot = cfg.iscsi.boot.enable;
      nixModule = declarativeHandoffNixPath;
      importPatch = declarativeHandoffImportPatchPath;
      generatedFiles = generatedArtifactPaths;
      autoImport = {
        enabled = cfg.apply.declarativeHandoff.autoImport.enable;
        configurationPath = cfg.apply.declarativeHandoff.autoImport.configurationPath;
        backupDirectory = cfg.apply.declarativeHandoff.autoImport.backupDirectory;
      };
    };
    nativeServices = {
      lvm = hasActiveLvm || hasActiveVdoVolumes;
      lvmThin = hasActiveLvmThinSupport;
      lvmVdo = hasActiveVdoVolumes;
      mdraid = hasActiveMdRaids;
      multipath = hasActiveMultipathMaps;
      zfsExtraPools = zfsExtraPools;
      bcache = hasActiveCaches;
      nfsServer = nfsExportLines != [ ];
    };
  };
in
{
  inherit
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
}
