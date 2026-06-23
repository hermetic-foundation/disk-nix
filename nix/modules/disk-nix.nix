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
  operationType = lib.types.nullOr (
    lib.types.enum [
      "create"
      "format"
      "grow"
      "shrink"
      "replace-device"
      "add-device"
      "remove-device"
      "set-property"
      "snapshot"
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

        desiredSize = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Desired object size for grow, shrink, or provisioning plans.";
          example = "100GiB";
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
    cleanSpecAttrs (
      object.metadata
      // {
        inherit (object)
          operation
          addDevices
          removeDevices
          replaceDevices
          properties
          destroy
          preserveData
          desiredSize
          ;
      }
    )
  );
  normalizeSnapshotSpec = lib.mapAttrs (
    _: snapshot:
    cleanSpecAttrs (
      snapshot.metadata
      // {
        inherit (snapshot)
          target
          destroy
          rollback
          preserveData
          ;
      }
    )
  );
  typedFilesystemSpec = lib.mapAttrs (_: filesystem: {
    inherit (filesystem)
      device
      fsType
      mountpoint
      options
      neededForBoot
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
      preserveData
      ;
    device = mount.source;
  }) cfg.nfs.mounts;
  typedSwapSpec = lib.mapAttrs (_: swap: {
    inherit (swap)
      device
      priority
      randomEncryption
      preserveData
      ;
  }) cfg.swaps;
  typedLuksSpec = lib.mapAttrs (_: luks: {
    inherit (luks)
      device
      name
      allowDiscards
      bypassWorkqueues
      preLVM
      preserveData
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

              preserveData = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Whether the planner must preserve the existing LUKS container.";
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

    volumeGroups = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed volume-group lifecycle declarations emitted into the disk-nix planner spec.";
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

    luns = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed LUN lifecycle declarations emitted into the disk-nix planner spec.";
    };

    exports = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed NFS export lifecycle declarations emitted into the disk-nix planner spec.";
    };

    caches = lib.mkOption {
      type = lifecycleAttrs;
      default = { };
      description = "Typed cache-layer lifecycle declarations emitted into the disk-nix planner spec.";
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
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    environment.etc."disk-nix/spec.json".source = json.generate "disk-nix-spec.json" {
      spec = cfg.spec // {
        filesystems = (cfg.spec.filesystems or { }) // typedFilesystemSpec // typedNfsMountSpec;
        swaps = (cfg.spec.swaps or { }) // typedSwapSpec;
        luks = (cfg.spec.luks or { }) // {
          devices = ((cfg.spec.luks or { }).devices or { }) // typedLuksSpec;
        };
        iscsi = (cfg.spec.iscsi or { }) // typedIscsiSpec;
        iscsiSessions = (cfg.spec.iscsiSessions or { }) // normalizeLifecycleSpec cfg.iscsi.sessions;
        nfs = (cfg.spec.nfs or { }) // {
          mounts = ((cfg.spec.nfs or { }).mounts or { }) // typedNfsMountSpec;
        };
        volumes = (cfg.spec.volumes or { }) // normalizeLifecycleSpec cfg.volumes;
        volumeGroups = (cfg.spec.volumeGroups or { }) // normalizeLifecycleSpec cfg.volumeGroups;
        pools = (cfg.spec.pools or { }) // normalizeLifecycleSpec cfg.pools;
        datasets = (cfg.spec.datasets or { }) // normalizeLifecycleSpec cfg.datasets;
        luns = (cfg.spec.luns or { }) // normalizeLifecycleSpec cfg.luns;
        exports = (cfg.spec.exports or { }) // normalizeLifecycleSpec cfg.exports;
        caches = (cfg.spec.caches or { }) // normalizeLifecycleSpec cfg.caches;
        snapshots = (cfg.spec.snapshots or { }) // normalizeSnapshotSpec cfg.snapshots;
      };
      apply = cfg.apply;
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
      }) cfg.nfs.mounts;

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
    ) cfg.swaps;

    boot.initrd.luks.devices = lib.mapAttrs (_: luks: {
      inherit (luks)
        device
        preLVM
        allowDiscards
        bypassWorkqueues
        ;
    }) cfg.luks.devices;

    services.openiscsi = lib.mkIf (cfg.iscsi.initiatorName != null) {
      enable = true;
      name = cfg.iscsi.initiatorName;
      inherit (cfg.iscsi)
        enableAutoLoginOut
        extraConfig
        ;
      discoverPortal = cfg.iscsi.discoverPortal;
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
          cfg.iscsi.discoverPortal;
    };

    systemd.services.disk-nix-plan = {
      description = "Validate disk-nix storage apply policy";
      wantedBy = lib.mkIf (cfg.apply.mode == "activation") [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${lib.getExe cfg.package} apply --spec /etc/disk-nix/spec.json";
      };
    };

    assertions = [
      {
        assertion = !(cfg.apply.allowDestructive && cfg.apply.mode == "activation");
        message = "disk-nix refuses destructive activation-mode storage changes.";
      }
      {
        assertion = cfg.iscsi.boot.enable -> cfg.iscsi.initiatorName != null;
        message = "services.disk-nix.iscsi.boot.enable requires services.disk-nix.iscsi.initiatorName.";
      }
      {
        assertion =
          cfg.iscsi.boot.enable
          -> (cfg.iscsi.boot.discoverPortal != null || cfg.iscsi.discoverPortal != null);
        message = "services.disk-nix.iscsi.boot.enable requires services.disk-nix.iscsi.boot.discoverPortal or services.disk-nix.iscsi.discoverPortal.";
      }
      {
        assertion = cfg.iscsi.boot.enable -> (cfg.iscsi.boot.loginAll || cfg.iscsi.boot.target != null);
        message = "services.disk-nix.iscsi.boot.enable requires a boot target unless loginAll is true.";
      }
    ];
  };
}
