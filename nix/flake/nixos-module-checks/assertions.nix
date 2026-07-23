{
  pkgs,
  nixosModuleCollisionTest,
  nixosModuleDiskCollisionTest,
  nixosModulePartitionCollisionTest,
  nixosModuleLuksKeyslotCollisionTest,
  nixosModuleLuksTokenCollisionTest,
  nixosModuleBackingFileCollisionTest,
  nixosModuleBtrfsSubvolumeCollisionTest,
  nixosModuleBtrfsQgroupCollisionTest,
  nixosModuleDmMapCollisionTest,
  nixosModuleVdoVolumeCollisionTest,
  nixosModulePhysicalVolumeCollisionTest,
  nixosModuleLoopDeviceCollisionTest,
  nixosModuleMdRaidCollisionTest,
  nixosModuleMultipathMapCollisionTest,
  nixosModuleNvmeNamespaceCollisionTest,
  nixosModuleCacheCollisionTest,
  nixosModulePoolCollisionTest,
  nixosModuleDatasetCollisionTest,
  nixosModuleZvolCollisionTest,
  nixosModuleVolumeGroupCollisionTest,
  nixosModuleVolumeCollisionTest,
  nixosModuleThinPoolCollisionTest,
  nixosModuleLvmCacheCollisionTest,
  nixosModuleSnapshotCollisionTest,
  nixosModuleIscsiSessionCollisionTest,
  nixosModuleLunPathCollisionTest,
  ...
}:

{
  nixosModuleAssertions = pkgs.runCommand "disk-nix-nixos-module-assertions-check" { } ''
    collisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleCollisionTest.config.system.build.toplevel).success))}
    diskCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDiskCollisionTest.config.system.build.toplevel).success))}
    partitionCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePartitionCollisionTest.config.system.build.toplevel).success))}
    luksKeyslotCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLuksKeyslotCollisionTest.config.system.build.toplevel).success))}
    luksTokenCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLuksTokenCollisionTest.config.system.build.toplevel).success))}
    backingFileCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBackingFileCollisionTest.config.system.build.toplevel).success))}
    btrfsSubvolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBtrfsSubvolumeCollisionTest.config.system.build.toplevel).success))}
    btrfsQgroupCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleBtrfsQgroupCollisionTest.config.system.build.toplevel).success))}
    dmMapCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDmMapCollisionTest.config.system.build.toplevel).success))}
    vdoVolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVdoVolumeCollisionTest.config.system.build.toplevel).success))}
    physicalVolumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePhysicalVolumeCollisionTest.config.system.build.toplevel).success))}
    loopDeviceCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLoopDeviceCollisionTest.config.system.build.toplevel).success))}
    mdRaidCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleMdRaidCollisionTest.config.system.build.toplevel).success))}
    multipathMapCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleMultipathMapCollisionTest.config.system.build.toplevel).success))}
    nvmeNamespaceCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleNvmeNamespaceCollisionTest.config.system.build.toplevel).success))}
    cacheCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleCacheCollisionTest.config.system.build.toplevel).success))}
    poolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModulePoolCollisionTest.config.system.build.toplevel).success))}
    datasetCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleDatasetCollisionTest.config.system.build.toplevel).success))}
    zvolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleZvolCollisionTest.config.system.build.toplevel).success))}
    volumeGroupCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVolumeGroupCollisionTest.config.system.build.toplevel).success))}
    volumeCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleVolumeCollisionTest.config.system.build.toplevel).success))}
    thinPoolCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleThinPoolCollisionTest.config.system.build.toplevel).success))}
    lvmCacheCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLvmCacheCollisionTest.config.system.build.toplevel).success))}
    snapshotCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleSnapshotCollisionTest.config.system.build.toplevel).success))}
    iscsiSessionCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleIscsiSessionCollisionTest.config.system.build.toplevel).success))}
    lunPathCollisionSuccess=${pkgs.lib.escapeShellArg (builtins.toJSON ((builtins.tryEval nixosModuleLunPathCollisionTest.config.system.build.toplevel).success))}
    test "$collisionSuccess" = false
    test "$diskCollisionSuccess" = false
    test "$partitionCollisionSuccess" = false
    test "$luksKeyslotCollisionSuccess" = false
    test "$luksTokenCollisionSuccess" = false
    test "$backingFileCollisionSuccess" = false
    test "$btrfsSubvolumeCollisionSuccess" = false
    test "$btrfsQgroupCollisionSuccess" = false
    test "$dmMapCollisionSuccess" = false
    test "$vdoVolumeCollisionSuccess" = false
    test "$physicalVolumeCollisionSuccess" = false
    test "$loopDeviceCollisionSuccess" = false
    test "$mdRaidCollisionSuccess" = false
    test "$multipathMapCollisionSuccess" = false
    test "$nvmeNamespaceCollisionSuccess" = false
    test "$cacheCollisionSuccess" = false
    test "$poolCollisionSuccess" = false
    test "$datasetCollisionSuccess" = false
    test "$zvolCollisionSuccess" = false
    test "$volumeGroupCollisionSuccess" = false
    test "$volumeCollisionSuccess" = false
    test "$thinPoolCollisionSuccess" = false
    test "$lvmCacheCollisionSuccess" = false
    test "$snapshotCollisionSuccess" = false
    test "$iscsiSessionCollisionSuccess" = false
    test "$lunPathCollisionSuccess" = false
    touch "$out"
  '';
}
