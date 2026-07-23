{
  pkgs,
  nixosModuleTest,
  ...
}:

pkgs.runCommand "disk-nix-nixos-module-steady-state-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
      steadyState=${
        pkgs.lib.escapeShellArg (
          builtins.readFile nixosModuleTest.config.environment.etc."disk-nix/steady-state.json".source
        )
      }
      printf '%s\n' "$steadyState" > steady-state
      jq -e '
        .version == 1
        and .fileSystems."/srv/tuned".device == "nas.example.com:/srv/tuned"
        and .fileSystems."/srv/tuned".fsType == "nfs4"
        and .fileSystems."/mnt/local-mount".device == "/dev/disk/by-label/local-mount"
        and .fileSystems."/mnt/local-mount".fsType == "xfs"
        and (.fileSystems | has("/mnt/local-unmount") | not)
        and (.fileSystems | has("/srv/old") | not)
        and (.swapDevices | length == 4)
        and (.swapDevices | any(.device == "/dev/disk/by-label/swap"))
        and (.swapDevices | all(.device != "/dev/disk/by-label/destroyed-swap"))
        and .luksDevices.cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
        and (.luksDevices | has("cryptclosed") | not)
        and .zramSwap.enable == true
        and .zramSwap.swapDevices == 2
        and .zramSwap.memoryMax == 8589934592
        and (.supportedFilesystems | index("xfs") != null)
        and (.supportedFilesystems | index("nfs4") != null)
        and (.supportedFilesystems | index("zfs") != null)
        and (.nfsExports | index("/srv/share 192.0.2.0/24(rw,sync,no_subtree_check)") != null)
        and (.nfsExports | all(. | contains("/srv/old-share") | not))
        and (.storageIdentities.filesystemMountpoints | index("/mnt/local-mount") != null)
        and (.storageIdentities.filesystemMountpoints | index("/mnt/local-unmount") == null)
        and (.storageIdentities.swapDevices | index("/dev/disk/by-label/swap") != null)
        and (.storageIdentities.swapDevices | index("/dev/disk/by-label/destroyed-swap") == null)
        and (.storageIdentities.physicalVolumes | index("/dev/disk/by-id/nvme-pv-grow") != null)
        and (.storageIdentities.volumes | index("vg0/scratch") != null)
        and (.storageIdentities.volumes | index("vg0/archive") == null)
        and (.storageIdentities.thinPools | index("vg0/thinpool") != null)
        and (.storageIdentities.lvmCaches | index("vg0/root") != null)
        and (.storageIdentities.vdoVolumes | index("archive") != null)
        and (.storageIdentities.vdoVolumes | index("cold-archive") == null)
        and (.storageIdentities.luksKeyslots | index("/dev/disk/by-id/root-luks keyslot 1") != null)
        and (.storageIdentities.luksTokens | index("/dev/disk/by-id/root-luks token 0") != null)
        and (.storageIdentities.backingFiles | index("/var/lib/images/root.img") != null)
        and (.storageIdentities.loopDevices | index("/dev/loop7") != null)
        and (.storageIdentities.dmMaps | index("/dev/mapper/cryptroot") != null)
        and (.storageIdentities.mdRaids | index("/dev/md/root") != null)
        and (.storageIdentities.mdRaids | index("/dev/md/oldroot") == null)
        and (.storageIdentities.multipathMaps | index("mpatha") != null)
        and (.storageIdentities.pools | index("vault") != null)
        and (.storageIdentities.pools | index("moveme") == null)
        and (.storageIdentities.datasets | index("tank/home") != null)
        and (.storageIdentities.zvols | index("tank/vm/root") != null)
        and (.storageIdentities.btrfsSubvolumes | index("/mnt/persist/@home") != null)
        and (.storageIdentities.btrfsQgroups | index("0/257 /mnt/persist") != null)
        and (.storageIdentities.snapshots | index("tank/home@before-upgrade") != null)
        and (.storageIdentities.caches | index("tank/l2arc0") != null)
        and (.storageIdentities.nvmeNamespaces | index("/dev/nvme0 nsid 4") != null)
        and (.networkStorage.iscsiSessionTargets | index("iqn.2026-06.example:storage.root") != null)
        and (.networkStorage.iscsiSessionTargets | index("iqn.2026-06.example:storage.logout") == null)
        and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
        and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-0") != null)
        and (.networkStorage.lunHostPaths | index("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") == null)
        and (.networkStorage.nfsExportSelectors | index("/srv/share 192.0.2.0/24") != null)
        and (.networkStorage.nfsExportSelectors | index("/srv/old-share 192.0.2.55") == null)
        and .lifecycleManaged.filesystems."/mnt/local-mount".operation == "mount"
        and .lifecycleManaged.filesystems."/mnt/local-mount".identity == "/mnt/local-mount"
        and (.lifecycleManaged.filesystems | has("/mnt/local-unmount") | not)
        and .lifecycleManaged.swapDevices."/dev/disk/by-label/swap".operation == "format"
        and .lifecycleManaged.swapDevices."/dev/disk/by-label/swap".desiredSize == "8GiB"
        and (.lifecycleManaged.swapDevices | has("/dev/disk/by-label/destroyed-swap") | not)
        and .lifecycleManaged.physicalVolumes."/dev/disk/by-id/nvme-pv-grow".operation == "grow"
        and .lifecycleManaged.volumes."vg0/scratch".operation == "create"
        and .lifecycleManaged.volumes."vg0/scratch".desiredSize == "10GiB"
        and (.lifecycleManaged.volumes | has("vg0/archive") | not)
        and .lifecycleManaged.thinPools."vg0/thinpool".operation == "grow"
        and .lifecycleManaged.lvmCaches."vg0/root".operation == "create"
        and .lifecycleManaged.vdoVolumes.archive.operation == "grow"
        and .lifecycleManaged.vdoVolumes.archive.desiredSize == "4TiB"
        and (.lifecycleManaged.vdoVolumes | has("cold-archive") | not)
        and .lifecycleManaged.luksKeyslots."/dev/disk/by-id/root-luks keyslot 1".operation == "add-key"
        and (.lifecycleManaged.luksKeyslots | has("/dev/disk/by-id/root-luks keyslot 2") | not)
        and .lifecycleManaged.btrfsQgroups."0/257 /mnt/persist".identity == "0/257 /mnt/persist"
        and .lifecycleManaged.snapshots."tank/home@before-upgrade".operation == "create"
        and .lifecycleManaged.luns."/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0".operation == "grow"
        and (.lifecycleManaged.luns | has("/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.2026-06.example:storage-lun-3") | not)
        and .lifecycleManaged.iscsiSessions."iqn.2026-06.example:storage.root".operation == "grow"
        and (.lifecycleManaged.iscsiSessions | has("iqn.2026-06.example:storage.logout") | not)
        and .lifecycleManaged.nfsExports."/srv/share 192.0.2.0/24".operation == "export"
        and (.lifecycleManaged.nfsExports | has("/srv/old-share 192.0.2.55") | not)
        and .iscsi.openiscsi.enable == true
        and .iscsi.openiscsi.discoverPortal == "192.0.2.10:3260"
        and .iscsi.bootInitiator.enable == true
        and .iscsi.bootInitiator.discoverPortal == "192.0.2.10:3260"
        and (.declarativeHandoff.fileSystems | index("/mnt/local-mount") != null)
        and (.declarativeHandoff.fileSystems | index("/mnt/local-unmount") == null)
        and (.declarativeHandoff.swapDevices | index("/dev/disk/by-label/swap") != null)
        and (.declarativeHandoff.swapDevices | index("/dev/disk/by-label/destroyed-swap") == null)
        and (.declarativeHandoff.luksDevices | index("cryptroot") != null)
        and (.declarativeHandoff.luksDevices | index("cryptclosed") == null)
        and (.declarativeHandoff.nfsExports | index("/srv/share 192.0.2.0/24") != null)
        and (.declarativeHandoff.nfsExports | index("/srv/old-share 192.0.2.55") == null)
        and (.declarativeHandoff.iscsiSessions | index("iqn.2026-06.example:storage.root") != null)
        and (.declarativeHandoff.iscsiSessions | index("iqn.2026-06.example:storage.logout") == null)
        and .declarativeHandoff.iscsiBoot == true
        and .declarativeHandoff.nixModule == "/etc/disk-nix/declarative-handoff.nix"
        and .declarativeHandoff.importPatch == "/etc/disk-nix/declarative-handoff-import.patch"
        and .declarativeHandoff.autoImport.enabled == false
        and .declarativeHandoff.autoImport.configurationPath == "/etc/nixos/configuration.nix"
        and .declarativeHandoff.autoImport.backupDirectory == "/var/backups/disk-nix"
        and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/spec.json") != null)
        and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/steady-state.json") != null)
        and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/declarative-handoff.nix") != null)
        and (.declarativeHandoff.generatedFiles | index("/etc/disk-nix/declarative-handoff-import.patch") != null)
        and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply.sh") != null)
        and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply-report.json") != null)
        and (.declarativeHandoff.generatedFiles | index("/run/disk-nix/apply-receipt.json") != null)
        and .nativeServices.lvm == true
        and .nativeServices.lvmThin == true
        and .nativeServices.lvmVdo == true
        and .nativeServices.mdraid == true
        and .nativeServices.multipath == true
        and .nativeServices.bcache == true
        and .nativeServices.nfsServer == true
        and (.nativeServices.zfsExtraPools | index("tank") != null)
        and (.nativeServices.zfsExtraPools | index("moveme") == null)
        and (.nativeServices.zfsExtraPools | index("mnt") == null)
      ' steady-state
  touch "$out"
''
