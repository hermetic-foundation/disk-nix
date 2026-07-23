{
  self,
  lib,
  pkgs,
  packageSystem,
  json,
  operationType,
  lifecycleAttrs,
  snapshotAttrs,
  defaultToolPackages,
}:

{
  enable = lib.mkEnableOption "disk-nix storage lifecycle integration";

  package = lib.mkOption {
    type = lib.types.package;
    default = self.packages.${packageSystem}.disk-nix;
    defaultText = lib.literalExpression "inputs.disk-nix.packages.${pkgs.stdenv.hostPlatform.system}.disk-nix";
    description = "disk-nix CLI package used by the NixOS module.";
  };

  toolPackages = lib.mkOption {
    type = lib.types.listOf lib.types.package;
    default = defaultToolPackages;
    defaultText = lib.literalExpression ''
      with pkgs; [
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

            action = lib.mkOption {
              type = operationType;
              default = null;
              description = "Alias for operation accepted by the planner.";
              example = "rebalance";
            };

            destroy = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Request filesystem teardown or destruction in disk-nix planning without adding the mount to NixOS fileSystems.";
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

            metadata = lib.mkOption {
              type = lib.types.attrsOf json.type;
              default = { };
              description = "Domain-specific filesystem metadata copied into the planner spec.";
              example = {
                pool = "tank";
                role = "bulk-data";
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

            targetSize = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Alias accepted by disk-nix for the desired filesystem size.";
              example = "100GiB";
            };

            size = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Short alias accepted by disk-nix for the desired filesystem size.";
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

            target = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Concrete swap path when the attribute name is only a friendly declaration key.";
              example = "/dev/disk/by-label/swap";
            };

            path = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Alias for target accepted by disk-nix for logical swap declaration keys.";
              example = "/swapfile";
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

            action = lib.mkOption {
              type = operationType;
              default = null;
              description = "Alias for operation accepted by the planner.";
              example = "grow";
            };

            destroy = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Request swap teardown or signature removal in disk-nix planning without adding the device to NixOS swapDevices.";
            };

            desiredSize = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Desired swap size for disk-nix lifecycle planning.";
              example = "16GiB";
            };

            targetSize = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Alias accepted by disk-nix for the desired swap size.";
              example = "16GiB";
            };

            size = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Short alias accepted by disk-nix for the desired swap size.";
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
    description = "Typed swap declarations used to generate both disk-nix spec and NixOS swapDevices. A logical attribute name can set target, path, or device to the concrete swap path.";
  };

  zram = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable NixOS zramSwap from the disk-nix storage declaration.";
    };

    operation = lib.mkOption {
      type = operationType;
      default = null;
      description = "Requested zram lifecycle operation for disk-nix planning, such as rescan.";
      example = "rescan";
    };

    action = lib.mkOption {
      type = operationType;
      default = null;
      description = "Alias for operation accepted by the planner.";
      example = "rescan";
    };

    swapDevices = lib.mkOption {
      type = lib.types.ints.positive;
      default = 1;
      description = "Number of zram devices to use as swap.";
    };

    memoryPercent = lib.mkOption {
      type = lib.types.ints.positive;
      default = 50;
      description = "Maximum zram swap size as a percentage of system memory.";
    };

    memoryMax = lib.mkOption {
      type = lib.types.nullOr lib.types.int;
      default = null;
      description = "Maximum total zram swap size in bytes.";
      example = 8589934592;
    };

    priority = lib.mkOption {
      type = lib.types.int;
      default = 5;
      description = "Swap priority for generated zram swap devices.";
    };

    algorithm = lib.mkOption {
      type = lib.types.str;
      default = "zstd";
      description = "Compression algorithm for generated zram swap devices.";
      example = "lz4";
    };

    writebackDevice = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Optional writeback device for incompressible zram pages. NixOS allows only one zram swap device when this is set.";
      example = "/dev/zvol/tank/swap-writeback";
    };

    preserveData = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether disk-nix should treat zram changes as requiring preservation of active swap state.";
    };

    properties = lib.mkOption {
      type = lib.types.attrsOf json.type;
      default = { };
      description = "Zram properties copied into the disk-nix planner spec for planning, review, and reconciliation with generated zramSwap settings.";
      example = {
        "zram.algorithm" = "zstd";
      };
    };
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
              description = "Mapper name for the opened LUKS device. Set this explicitly when the attribute name is only a friendly declaration key.";
            };

            target = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Concrete mapper name when the attribute name is only a friendly declaration key.";
              example = "cryptroot";
            };

            mapperName = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Alias for target accepted by disk-nix for logical LUKS declaration keys.";
              example = "cryptroot";
            };

            mapper-name = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Hyphenated alias for mapperName accepted by disk-nix.";
              example = "cryptroot";
            };

            mapper = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Short alias for target accepted by disk-nix for logical LUKS declaration keys.";
              example = "cryptroot";
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

            action = lib.mkOption {
              type = operationType;
              default = null;
              description = "Alias for operation accepted by the planner.";
              example = "grow";
            };

            desiredSize = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Desired opened mapper size for disk-nix lifecycle planning.";
              example = "100%";
            };

            targetSize = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Alias accepted by disk-nix for the desired opened mapper size.";
              example = "100%";
            };

            size = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Short alias accepted by disk-nix for the desired opened mapper size.";
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
    description = "Typed LUKS declarations used to generate both disk-nix spec and boot.initrd.luks.devices. The name, target, mapperName, or mapper option supplies the concrete mapper name when the attribute name is logical.";
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

            action = lib.mkOption {
              type = operationType;
              default = null;
              description = "Alias for operation accepted by the planner.";
              example = "mount";
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

            metadata = lib.mkOption {
              type = lib.types.attrsOf json.type;
              default = { };
              description = "Domain-specific NFS mount metadata copied into the planner spec.";
              example = {
                server = "nas.example.com";
                export = "/srv/shared";
              };
            };
          };
        }
      )
    );
    default = { };
    description = "Typed NFS client mounts used to generate both disk-nix spec and NixOS fileSystems. The mountpoint option supplies the concrete local path when the attribute name is logical.";
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
    description = "Typed LVM logical-volume lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a canonical vg/lv target through the declaration key, target, or path.";
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
    description = "Typed VDO volume lifecycle declarations emitted into the disk-nix planner spec. The target option supplies the concrete VDO volume name when the attribute name is logical.";
  };

  physicalVolumes = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed LVM physical-volume lifecycle declarations emitted into the disk-nix planner spec. Executable create, grow, and remove plans require a concrete block device path through the declaration key, target, path, or device.";
  };

  luksKeyslots = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed LUKS keyslot lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a LUKS backing device and keyslot or key-file metadata depending on operation; keySlot, key-slot, or slot supplies the concrete slot id when the attribute name is logical.";
  };

  luksTokens = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed LUKS token lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a LUKS backing device and token JSON file for imports or a token id for removal; tokenId, token-id, or token supplies the concrete token id when the attribute name is logical.";
  };

  volumeGroups = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed volume-group lifecycle declarations emitted into the disk-nix planner spec.";
  };

  thinPools = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed LVM thin-pool lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a canonical vg/pool target through the declaration key, target, or path.";
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
    description = "Typed loop-device lifecycle declarations emitted into the disk-nix planner spec. Refresh, grow, and detach command plans require a /dev/loop* target through the declaration key, target, or path.";
  };

  backingFiles = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed file-backed storage origin lifecycle declarations emitted into the disk-nix planner spec. Grow command plans require a path-shaped declaration key, target, or path plus desiredSize, targetSize, or size.";
  };

  dmMaps = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed device-mapper lifecycle declarations emitted into the disk-nix planner spec. Rescan, rename, and destroy command plans require a concrete /dev/mapper/* or /dev/dm-* target through the declaration key, target, or path.";
  };

  mdRaids = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed MD RAID lifecycle declarations emitted into the disk-nix planner spec. Executable create, grow, member-add, member-replacement, and member-removal plans require an explicit /dev/md* array target.";
  };

  multipathMaps = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed multipath map lifecycle declarations emitted into the disk-nix planner spec. Executable grow, replacement preflight, rescan, and destroy plans require a concrete mpath* or /dev/mapper/* map target.";
  };

  pools = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed pool lifecycle declarations emitted into the disk-nix planner spec.";
  };

  datasets = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed dataset lifecycle declarations emitted into the disk-nix planner spec. A logical attribute name can set target or path to the concrete ZFS pool/name dataset.";
  };

  zvols = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed ZFS zvol lifecycle declarations emitted into the disk-nix planner spec. A logical attribute name can set target or path to the concrete ZFS pool/name zvol.";
  };

  luns = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed LUN lifecycle declarations emitted into the disk-nix planner spec.";
  };

  nvmeNamespaces = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed NVMe namespace lifecycle declarations emitted into the disk-nix planner spec. Executable plans require a /dev/nvme* controller path through the declaration key, target, path, or device, plus namespace metadata for attach or delete operations.";
  };

  exports = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed NFS export lifecycle declarations emitted into the disk-nix planner spec. Executable exportfs plans require a local export path through the declaration key, target, or path, plus explicit client and options fields.";
  };

  caches = lib.mkOption {
    type = lifecycleAttrs;
    default = { };
    description = "Typed cache-layer lifecycle declarations emitted into the disk-nix planner spec. bcache device sysfs command plans require a concrete /dev/bcache* target supplied by the declaration key, target, path, or device; cache-set-scoped bcache.set-* property updates require cacheSetUuid.";
  };

  snapshots = lib.mkOption {
    type = snapshotAttrs;
    default = { };
    description = "Typed snapshot lifecycle declarations emitted into the disk-nix planner spec. A logical attribute name can set name, snapshotName, or snapshot-name to the concrete snapshot identity.";
  };

  apply = import ./apply-options.nix {
    inherit lib operationType;
  };
}
