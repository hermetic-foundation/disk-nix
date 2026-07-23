{ lib, json }:

let
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
      "rescan"
      "replace-device"
      "add-device"
      "remove-device"
      "add-key"
      "remove-key"
      "import-token"
      "remove-token"
      "set-property"
      "snapshot"
      "clone"
      "promote"
      "import"
      "export"
      "unexport"
      "attach"
      "detach"
      "activate"
      "deactivate"
      "assemble"
      "start"
      "stop"
      "login"
      "logout"
      "open"
      "close"
      "mount"
      "unmount"
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

        action = lib.mkOption {
          type = operationType;
          default = null;
          description = "Alias for operation accepted by the planner.";
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

        paths = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "Stable host paths for path-addressed lifecycle objects such as LUNs.";
          example = [ "/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.example-lun-0" ];
        };

        devicePaths = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = "Alias for paths accepted by disk-nix for stable host path declarations.";
          example = [ "/dev/disk/by-path/ip-192.0.2.11:3260-iscsi-iqn.example-lun-0" ];
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

        cacheSetUuid = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Explicit bcache cache-set UUID for replacement cache media and cache-set-scoped sysfs property updates.";
          example = "11111111-2222-3333-4444-555555555555";
        };

        physicalSize = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Explicit physical backing-size intent for VDO growPhysical planning.";
          example = "6TiB";
        };

        renameTo = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "New name or path for rename lifecycle operations.";
          example = "tank/home-staged";
        };

        renameTarget = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias accepted by disk-nix for rename lifecycle operations.";
          example = "tank/home-staged";
        };

        newName = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias accepted by disk-nix for rename lifecycle operations.";
          example = "tank/home-staged";
        };

        properties = lib.mkOption {
          type = lib.types.attrsOf json.type;
          default = { };
          description = "Storage-specific properties to set on this object, such as bcache.cache-mode or bcache.set-journal-delay-ms for cache-set tuning.";
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

        readonly = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Short alias accepted by disk-nix for read-only lifecycle actions.";
          example = true;
        };

        desiredSize = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Desired object size for grow, shrink, or provisioning plans.";
          example = "100GiB";
        };

        targetSize = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias accepted by disk-nix for the desired object size.";
          example = "100GiB";
        };

        size = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias accepted by disk-nix for the desired object size.";
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

        nsid = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias accepted by disk-nix for NVMe namespace id.";
          example = "4";
        };

        controllers = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Comma-separated NVMe controller id list used by namespace attach and detach operations.";
          example = "0x1";
        };

        controllerId = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias accepted by disk-nix for NVMe controller id lists.";
          example = "0x1";
        };

        controller = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias accepted by disk-nix for NVMe controller id lists.";
          example = "0x1";
        };

        keySlot = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "LUKS keyslot number used by keyslot lifecycle declarations.";
          example = "1";
        };

        "key-slot" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Hyphenated alias accepted by disk-nix for LUKS keyslot number.";
          example = "1";
        };

        slot = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias accepted by disk-nix for LUKS keyslot number.";
          example = "1";
        };

        keyFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Existing LUKS key file used when changing key material.";
          example = "/run/keys/root-old";
        };

        "key-file" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Hyphenated alias accepted by disk-nix for the existing LUKS key file.";
          example = "/run/keys/root-old";
        };

        currentKeyFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias accepted by disk-nix for the existing LUKS key file.";
          example = "/run/keys/root-old";
        };

        newKeyFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Replacement or newly enrolled LUKS key file.";
          example = "/run/keys/root-new";
        };

        "new-key-file" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Hyphenated alias accepted by disk-nix for the replacement or newly enrolled LUKS key file.";
          example = "/run/keys/root-new";
        };

        tokenId = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "LUKS token id used by token lifecycle declarations.";
          example = "0";
        };

        "token-id" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Hyphenated alias accepted by disk-nix for LUKS token id.";
          example = "0";
        };

        token = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias accepted by disk-nix for LUKS token id.";
          example = "0";
        };

        tokenFile = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "JSON token file imported by LUKS token lifecycle declarations.";
          example = "/run/keys/root-token.json";
        };

        "token-file" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Hyphenated alias accepted by disk-nix for the imported LUKS token JSON file.";
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

        name = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Concrete snapshot identity when the attribute name is only a friendly declaration key.";
          example = "tank/home@before-upgrade";
        };

        snapshotName = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for name, copied into the planner spec for explicit snapshot identity.";
          example = "tank/home@before-upgrade";
        };

        "snapshot-name" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Kebab-case alias for snapshotName, copied into the planner spec.";
          example = "tank/home@before-upgrade";
        };

        path = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Concrete snapshot path when the attribute name is a friendly key, especially for Btrfs snapshot rescans.";
          example = "/mnt/persist/@home-before-upgrade";
        };

        snapshotPath = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for path, copied into the planner spec for explicit snapshot identity.";
          example = "/mnt/persist/@home-before-upgrade";
        };

        "snapshot-path" = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Kebab-case alias for snapshotPath, copied into the planner spec.";
          example = "/mnt/persist/@home-before-upgrade";
        };

        operation = lib.mkOption {
          type = operationType;
          default = null;
          description = "Requested snapshot lifecycle operation, such as rescan.";
          example = "rescan";
        };

        action = lib.mkOption {
          type = operationType;
          default = null;
          description = "Alias for operation accepted by the planner.";
          example = "rescan";
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

        cloneTarget = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for cloneTo accepted by the planner.";
          example = "tank/home-review";
        };

        clone = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Short alias for cloneTo accepted by the planner.";
          example = "tank/home-review";
        };

        renameTo = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "New ZFS snapshot name or Btrfs snapshot path for rename lifecycle operations.";
          example = "tank/home@before-prune";
        };

        renameTarget = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for renameTo accepted by the planner.";
          example = "tank/home@before-prune";
        };

        newName = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Alias for renameTo accepted by the planner.";
          example = "tank/home@before-prune";
        };

        recursiveRollback = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Render recursive ZFS rollback with zfs rollback -r when explicitly true.";
        };

        recursive = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Alias for recursiveRollback accepted by the planner.";
        };

        "zfs.rollbackRecursive" = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "ZFS-specific alias for recursiveRollback accepted by the planner.";
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

        readonly = lib.mkOption {
          type = lib.types.nullOr lib.types.bool;
          default = null;
          description = "Alias for readOnly accepted by the planner.";
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
in
{
  inherit
    operationType
    lifecycleAttrs
    snapshotAttrs
    ;
}
