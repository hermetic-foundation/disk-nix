{
  pkgs,
  nixosModuleTest,
  ...
}:

pkgs.runCommand "disk-nix-nixos-module-activation-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
      applyScript='${nixosModuleTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
      grep -- 'validate' "$applyScript"
      grep -- '--probe-current' "$applyScript"
      grep -- '--script-out' "$applyScript"
      grep -- '/run/disk-nix/apply.sh' "$applyScript"
      grep -- '--report-out' "$applyScript"
      grep -- '/run/disk-nix/apply-report.json' "$applyScript"
      grep -- '--receipt-out' "$applyScript"
      grep -- '/run/disk-nix/apply-receipt.json' "$applyScript"
      printf '%s\n' ${pkgs.lib.escapeShellArgs (map toString nixosModuleTest.config.systemd.services.disk-nix-plan.path)} > service-paths
      grep -- 'bcachefs-tools-' service-paths
      grep -- 'btrfs-progs-' service-paths
      grep -- 'dosfstools-' service-paths
      grep -- 'exfatprogs-' service-paths
      grep -- 'f2fs-tools-' service-paths
      grep -- 'lvm2-' service-paths
      grep -- 'lsscsi-' service-paths
      grep -- 'ntfs3g-' service-paths
      grep -- 'open-iscsi-' service-paths
      grep -- 'smartmontools-' service-paths
      grep -- 'targetcli-fb-' service-paths
      grep -- 'tgt-' service-paths
      grep -- 'util-linux-' service-paths
      grep -- 'zfs-user-' service-paths
      swapDevices=${
        pkgs.lib.escapeShellArg (
          builtins.toJSON (map (swap: { inherit (swap) device; }) nixosModuleTest.config.swapDevices)
        )
      }
      printf '%s\n' "$swapDevices" > swap-devices
      jq -e '
        length == 4
        and any(.[]; .device == "/dev/disk/by-label/swap")
        and any(.[]; .device == "/dev/disk/by-label/swap-inventory")
        and any(.[]; .device == "/dev/disk/by-label/swap-target-size")
        and any(.[]; .device == "/dev/disk/by-label/swap-size")
        and all(.[]; .device != "/dev/disk/by-label/action-old-swap")
        and all(.[]; .device != "/dev/disk/by-label/destroyed-swap")
      ' swap-devices
      luksDevices=${
        pkgs.lib.escapeShellArg (
          builtins.toJSON (
            pkgs.lib.mapAttrs (_: luks: {
              inherit (luks) device;
            }) nixosModuleTest.config.boot.initrd.luks.devices
          )
        )
      }
      printf '%s\n' "$luksDevices" > luks-devices
      jq -e '
        has("cryptroot")
        and .cryptroot.device == "/dev/disk/by-partuuid/d024c121-4300-4493-a643-055bc4d5caa7"
        and has("cryptTargetSizeMapper")
        and .cryptTargetSizeMapper.device == "/dev/disk/by-id/target-size-luks"
        and (has("cryptTargetSize") | not)
        and has("cryptSize")
        and .cryptSize.device == "/dev/disk/by-id/size-luks"
        and has("cryptarchive")
        and .cryptarchive.device == "/dev/disk/by-id/archive-luks"
        and (has("cryptold") | not)
        and (has("cryptclosed") | not)
      ' luks-devices
      fileSystems=${
        pkgs.lib.escapeShellArg (
          builtins.toJSON (
            pkgs.lib.mapAttrs (_: fs: {
              inherit (fs) device fsType;
            }) nixosModuleTest.config.fileSystems
          )
        )
      }
      printf '%s\n' "$fileSystems" > file-systems
      jq -e '
        has("/srv/shared")
        and ."/srv/shared".device == "nas.example.com:/srv/shared"
        and has("/srv/tuned")
        and ."/srv/tuned".device == "nas.example.com:/srv/tuned"
        and ."/srv/tuned".fsType == "nfs4"
        and has("/mnt/local-mount")
        and ."/mnt/local-mount".device == "/dev/disk/by-label/local-mount"
        and ."/mnt/local-mount".fsType == "xfs"
        and (has("/mnt/local-unmount") | not)
        and has("/mnt/local-rescan")
        and ."/mnt/local-rescan".device == "/dev/disk/by-label/local-rescan"
        and ."/mnt/local-rescan".fsType == "xfs"
        and has("/mnt/action-rescan")
        and ."/mnt/action-rescan".device == "/dev/disk/by-label/action-rescan"
        and ."/mnt/action-rescan".fsType == "xfs"
        and (has("/mnt/action-unmount") | not)
        and (has("/mnt/teardown-only") | not)
        and (has("/mnt/destroyed") | not)
        and has("/srv/action")
        and ."/srv/action".device == "nas.example.com:/srv/action"
        and ."/srv/action".fsType == "nfs4"
        and has("/run/disk-nix-tmp")
        and ."/run/disk-nix-tmp".device == "tmpfs"
        and ."/run/disk-nix-tmp".fsType == "tmpfs"
        and has("/srv/disk-nix-cache")
        and ."/srv/disk-nix-cache".device == "/var/cache/disk-nix"
        and ."/srv/disk-nix-cache".fsType == "none"
        and has("/srv/disk-nix-overlay")
        and ."/srv/disk-nix-overlay".device == "overlay"
        and ."/srv/disk-nix-overlay".fsType == "overlay"
        and (has("/srv/old") | not)
      ' file-systems
      supportedFilesystems=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleTest.config.boot.supportedFilesystems)}
      printf '%s\n' "$supportedFilesystems" > supported-filesystems
      jq -e '
        .xfs == true
        and .btrfs == true
        and .bcachefs == true
        and .f2fs == true
        and .tmpfs == true
        and .overlay == true
        and .nfs4 == true
        and .zfs == true
        and (has("jfs") | not)
      ' supported-filesystems
      nativeStorage=${
        pkgs.lib.escapeShellArg (
          builtins.toJSON {
            lvm = nixosModuleTest.config.services.lvm.enable;
            lvmInitrd = nixosModuleTest.config.boot.initrd.services.lvm.enable;
            lvmThin = nixosModuleTest.config.services.lvm.boot.thin.enable;
            lvmVdo = nixosModuleTest.config.services.lvm.boot.vdo.enable;
            swraid = nixosModuleTest.config.boot.swraid.enable;
            mdadmConf = nixosModuleTest.config.boot.swraid.mdadmConf;
            multipath = nixosModuleTest.config.services.multipath.enable;
            zfsExtraPools = nixosModuleTest.config.boot.zfs.extraPools;
            zfsForceImportRoot = nixosModuleTest.config.boot.zfs.forceImportRoot;
            bcache = nixosModuleTest.config.boot.bcache.enable;
            bcacheInitrd = nixosModuleTest.config.boot.initrd.services.bcache.enable;
            zram = nixosModuleTest.config.zramSwap.enable;
            zramSwapDevices = nixosModuleTest.config.zramSwap.swapDevices;
            zramMemoryPercent = nixosModuleTest.config.zramSwap.memoryPercent;
            zramMemoryMax = nixosModuleTest.config.zramSwap.memoryMax;
            zramPriority = nixosModuleTest.config.zramSwap.priority;
            zramAlgorithm = nixosModuleTest.config.zramSwap.algorithm;
            openIscsiDiscoverPortal = nixosModuleTest.config.services.openiscsi.discoverPortal;
            bootIscsiDiscoverPortal = nixosModuleTest.config.boot.iscsi-initiator.discoverPortal;
          }
        )
      }
      printf '%s\n' "$nativeStorage" > native-storage
      jq -e '
        .lvm == true
        and .lvmInitrd == true
        and .lvmThin == true
        and .lvmVdo == true
        and .swraid == true
        and (.mdadmConf | test("^PROGRAM .*/bin/true$"))
        and .multipath == true
        and (.zfsExtraPools | index("tank") != null)
        and (.zfsExtraPools | index("mnt") == null)
        and .zfsForceImportRoot == false
        and .bcache == true
        and .bcacheInitrd == true
        and .zram == true
        and .zramSwapDevices == 2
        and .zramMemoryPercent == 40
        and .zramMemoryMax == 8589934592
        and .zramPriority == 20
        and .zramAlgorithm == "zstd"
        and .openIscsiDiscoverPortal == "192.0.2.10:3260"
        and .bootIscsiDiscoverPortal == "192.0.2.10:3260"
      ' native-storage
  touch "$out"
''
