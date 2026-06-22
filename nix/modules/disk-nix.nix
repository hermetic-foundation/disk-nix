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
  typedFilesystemSpec = lib.mapAttrs (_: filesystem: {
    inherit (filesystem)
      device
      fsType
      mountpoint
      options
      neededForBoot
      resizePolicy
      preserveData
      ;
  }) cfg.filesystems;
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
        filesystems = (cfg.spec.filesystems or { }) // typedFilesystemSpec;
        swaps = (cfg.spec.swaps or { }) // typedSwapSpec;
        luks = (cfg.spec.luks or { }) // {
          devices = ((cfg.spec.luks or { }).devices or { }) // typedLuksSpec;
        };
      };
      apply = cfg.apply;
    };

    fileSystems = lib.mapAttrs' (_: filesystem: {
      name = filesystem.mountpoint;
      value = {
        inherit (filesystem)
          device
          fsType
          neededForBoot
          ;
      }
      // lib.optionalAttrs (filesystem.options != [ ]) {
        inherit (filesystem) options;
      };
    }) cfg.filesystems;

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

    systemd.services.disk-nix-plan = {
      description = "Plan disk-nix storage changes";
      wantedBy = lib.mkIf (cfg.apply.mode == "activation") [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${lib.getExe cfg.package} plan --spec /etc/disk-nix/spec.json";
      };
    };

    assertions = [
      {
        assertion = !(cfg.apply.allowDestructive && cfg.apply.mode == "activation");
        message = "disk-nix refuses destructive activation-mode storage changes.";
      }
    ];
  };
}
