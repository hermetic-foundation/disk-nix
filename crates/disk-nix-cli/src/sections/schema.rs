fn spec_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/midischwarz12/disk-nix/schema/disk-nix-spec.schema.json",
        "title": "disk-nix desired storage spec",
        "description": "Desired storage declaration accepted by disk-nix plan, apply, and validate. The CLI accepts either this direct shape or a wrapper with { spec, apply } as produced by the NixOS module.",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "version": {
                "type": "integer",
                "const": SUPPORTED_SPEC_VERSION,
                "description": "Optional disk-nix spec contract version. Version 1 is the current supported contract."
            },
            "spec": {
                "$ref": "#/$defs/specBody",
                "description": "NixOS module wrapper body. When present, planner lifecycle inputs are read from this object."
            },
            "apply": {
                "$ref": "#/$defs/applyPolicy"
            },
            "filesystems": {
                "$ref": "#/$defs/filesystemMap"
            },
            "swaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "zram": {
                "$ref": "#/$defs/zramSpec"
            },
            "luks": {
                "$ref": "#/$defs/luksSpec"
            },
            "nfs": {
                "$ref": "#/$defs/nfsSpec"
            },
            "iscsi": {
                "$ref": "#/$defs/iscsiSpec"
            },
            "disks": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "partitions": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "btrfsSubvolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "btrfsQgroups": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "vdoVolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "physicalVolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luksKeyslots": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luksTokens": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "volumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "volumeGroups": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "thinPools": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "lvmSnapshots": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "lvmCaches": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "loopDevices": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "backingFiles": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "dmMaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "mdRaids": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "multipathMaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "pools": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "datasets": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "zvols": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luns": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "targetLuns": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "nvmeNamespaces": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "iscsiSessions": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "exports": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "caches": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "snapshots": {
                "$ref": "#/$defs/snapshotMap"
            }
        },
        "$defs": {
            "specBody": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "version": {
                        "type": "integer",
                        "const": SUPPORTED_SPEC_VERSION,
                        "description": "Optional disk-nix spec contract version. Version 1 is the current supported contract."
                    },
                    "filesystems": { "$ref": "#/$defs/filesystemMap" },
                    "swaps": { "$ref": "#/$defs/lifecycleMap" },
                    "zram": { "$ref": "#/$defs/zramSpec" },
                    "luks": { "$ref": "#/$defs/luksSpec" },
                    "nfs": { "$ref": "#/$defs/nfsSpec" },
                    "iscsi": { "$ref": "#/$defs/iscsiSpec" },
                    "disks": { "$ref": "#/$defs/lifecycleMap" },
                    "partitions": { "$ref": "#/$defs/lifecycleMap" },
                    "btrfsSubvolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "btrfsQgroups": { "$ref": "#/$defs/lifecycleMap" },
                    "vdoVolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "physicalVolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "luksKeyslots": { "$ref": "#/$defs/lifecycleMap" },
                    "luksTokens": { "$ref": "#/$defs/lifecycleMap" },
                    "volumes": { "$ref": "#/$defs/lifecycleMap" },
                    "volumeGroups": { "$ref": "#/$defs/lifecycleMap" },
                    "thinPools": { "$ref": "#/$defs/lifecycleMap" },
                    "lvmSnapshots": { "$ref": "#/$defs/lifecycleMap" },
                    "lvmCaches": { "$ref": "#/$defs/lifecycleMap" },
                    "loopDevices": { "$ref": "#/$defs/lifecycleMap" },
                    "backingFiles": { "$ref": "#/$defs/lifecycleMap" },
                    "dmMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "mdRaids": { "$ref": "#/$defs/lifecycleMap" },
                    "multipathMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "pools": { "$ref": "#/$defs/lifecycleMap" },
                    "datasets": { "$ref": "#/$defs/lifecycleMap" },
                    "zvols": { "$ref": "#/$defs/lifecycleMap" },
                    "luns": { "$ref": "#/$defs/lifecycleMap" },
                    "targetLuns": { "$ref": "#/$defs/lifecycleMap" },
                    "nvmeNamespaces": { "$ref": "#/$defs/lifecycleMap" },
                    "iscsiSessions": { "$ref": "#/$defs/lifecycleMap" },
                    "exports": { "$ref": "#/$defs/lifecycleMap" },
                    "caches": { "$ref": "#/$defs/lifecycleMap" },
                    "snapshots": { "$ref": "#/$defs/snapshotMap" }
                }
            },
            "filesystemMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/filesystem" }
            },
            "filesystem": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mountpoint": { "type": "string" },
                    "device": { "type": "string" },
                    "fsType": { "type": "string" },
                    "type": { "type": "string" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "neededForBoot": { "type": "boolean" },
                    "destroy": { "type": "boolean" },
                    "resizePolicy": {
                        "type": "string",
                        "enum": ["none", "grow-only", "shrink-allowed"]
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "addDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "removeDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "replaceDevices": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "preserveData": { "type": "boolean", "default": true },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" }
                }
            },
            "lifecycleMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/lifecycleObject" }
            },
            "zramSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "enable": { "type": "boolean" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "swapDevices": { "type": "integer", "minimum": 1 },
                    "memoryPercent": { "type": "integer", "minimum": 1 },
                    "memoryMax": { "type": ["integer", "null"] },
                    "priority": { "type": "integer" },
                    "algorithm": { "type": "string" },
                    "writebackDevice": { "type": ["string", "null"] },
                    "preserveData": { "type": "boolean", "default": true },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "luksSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "devices": { "$ref": "#/$defs/lifecycleMap" }
                }
            },
            "nfsSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mounts": { "$ref": "#/$defs/nfsMountMap" }
                }
            },
            "nfsMountMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/nfsMount" }
            },
            "nfsMount": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "source": { "type": "string" },
                    "device": { "type": "string" },
                    "fsType": {
                        "type": "string",
                        "enum": ["nfs", "nfs4"]
                    },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "mountpoint": { "type": "string" },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "neededForBoot": { "type": "boolean" },
                    "destroy": { "type": "boolean" },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "preserveData": { "type": "boolean", "default": true }
                }
            },
            "iscsiSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "initiatorName": { "type": ["string", "null"] },
                    "discoverPortal": { "type": ["string", "null"] },
                    "enableAutoLoginOut": { "type": "boolean" },
                    "extraConfig": { "type": "string" },
                    "sessions": { "$ref": "#/$defs/lifecycleMap" },
                    "boot": { "$ref": "#/$defs/iscsiBoot" }
                }
            },
            "iscsiBoot": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "enable": { "type": "boolean" },
                    "discoverPortal": { "type": ["string", "null"] },
                    "target": { "type": ["string", "null"] },
                    "loginAll": { "type": "boolean" },
                    "logLevel": { "type": "integer" },
                    "extraIscsiCommands": { "type": "string" },
                    "extraConfig": { "type": ["string", "null"] }
                }
            },
            "lifecycleObject": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "addDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "devices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "devicePaths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "removeDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "replaceDevices": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "cacheSetUuid": { "type": "string" },
                    "cacheSetUUID": { "type": "string" },
                    "cache-set-uuid": { "type": "string" },
                    "cache_set_uuid": { "type": "string" },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "physicalSize": { "type": ["string", "number"] },
                    "vdoPhysicalSize": { "type": ["string", "number"] },
                    "physical-size": { "type": ["string", "number"] },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "name": { "type": "string" },
                    "target": { "type": "string" },
                    "path": { "type": "string" },
                    "mountpoint": { "type": "string" },
                    "device": { "type": "string" },
                    "disk": { "type": "string" },
                    "client": { "type": "string" },
                    "initiators": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "initiatorIqns": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "clients": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "portal": { "type": "string" },
                    "provider": { "type": "string" },
                    "storageProvider": { "type": "string" },
                    "storage-provider": { "type": "string" },
                    "arrayProvider": { "type": "string" },
                    "array-provider": { "type": "string" },
                    "vendor": { "type": "string" },
                    "arrayVendor": { "type": "string" },
                    "array-vendor": { "type": "string" },
                    "arrayId": { "type": "string" },
                    "arrayID": { "type": "string" },
                    "array-id": { "type": "string" },
                    "array_id": { "type": "string" },
                    "systemId": { "type": "string" },
                    "system-id": { "type": "string" },
                    "storagePool": { "type": "string" },
                    "storage-pool": { "type": "string" },
                    "poolName": { "type": "string" },
                    "pool-name": { "type": "string" },
                    "aggregate": { "type": "string" },
                    "volumeId": { "type": "string" },
                    "volumeID": { "type": "string" },
                    "volume-id": { "type": "string" },
                    "volume_id": { "type": "string" },
                    "volumeName": { "type": "string" },
                    "snapshotId": { "type": "string" },
                    "snapshotID": { "type": "string" },
                    "snapshot-id": { "type": "string" },
                    "snapshot_id": { "type": "string" },
                    "snapshotName": { "type": "string" },
                    "cloneSource": { "type": "string" },
                    "clone-source": { "type": "string" },
                    "sourceSnapshot": { "type": "string" },
                    "source-snapshot": { "type": "string" },
                    "sourceVolume": { "type": "string" },
                    "source-volume": { "type": "string" },
                    "maskingGroup": { "type": "string" },
                    "masking-group": { "type": "string" },
                    "hostGroup": { "type": "string" },
                    "host-group": { "type": "string" },
                    "igroup": { "type": "string" },
                    "lun": { "type": ["string", "number"] },
                    "lunId": { "type": ["string", "number"] },
                    "lun-id": { "type": ["string", "number"] },
                    "lunNumber": { "type": ["string", "number"] },
                    "lun-number": { "type": ["string", "number"] },
                    "namespaceId": { "type": ["string", "number"] },
                    "nsid": { "type": ["string", "number"] },
                    "controllers": { "type": "string" },
                    "controllerId": { "type": ["string", "number"] },
                    "controller": { "type": ["string", "number"] },
                    "keySlot": { "type": ["string", "number"] },
                    "key-slot": { "type": ["string", "number"] },
                    "slot": { "type": ["string", "number"] },
                    "keyFile": { "type": "string" },
                    "key-file": { "type": "string" },
                    "currentKeyFile": { "type": "string" },
                    "newKeyFile": { "type": "string" },
                    "new-key-file": { "type": "string" },
                    "tokenId": { "type": ["string", "number"] },
                    "token-id": { "type": ["string", "number"] },
                    "token": { "type": ["string", "number"] },
                    "tokenFile": { "type": "string" },
                    "token-file": { "type": "string" },
                    "jsonFile": { "type": "string" },
                    "options": { "type": "string" },
                    "priority": { "type": "integer" },
                    "randomEncryption": { "type": "boolean" },
                    "allowDiscards": { "type": "boolean" },
                    "bypassWorkqueues": { "type": "boolean" },
                    "preLVM": { "type": "boolean" },
                    "start": { "type": ["string", "number"] },
                    "startOffset": { "type": ["string", "number"] },
                    "end": { "type": ["string", "number"] },
                    "endOffset": { "type": ["string", "number"] },
                    "partitionNumber": { "type": ["string", "number"] },
                    "number": { "type": ["string", "number"] },
                    "partitionType": { "type": "string" },
                    "level": { "type": "string" },
                    "raidLevel": { "type": "string" },
                    "type": { "type": "string" },
                    "destroy": { "type": "boolean" },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "snapshotMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/snapshot" }
            },
            "snapshot": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "target": { "type": "string" },
                    "path": { "type": "string" },
                    "snapshotPath": { "type": "string" },
                    "snapshot-path": { "type": "string" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "destroy": { "type": "boolean" },
                    "rollback": { "type": "boolean" },
                    "cloneTo": { "type": "string" },
                    "cloneTarget": { "type": "string" },
                    "clone": { "type": "string" },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "recursiveRollback": { "type": "boolean" },
                    "recursive": { "type": "boolean" },
                    "zfs.rollbackRecursive": { "type": "boolean" },
                    "hold": { "type": "string" },
                    "holdTag": { "type": "string" },
                    "releaseHold": { "type": "string" },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "operation": {
                "type": "string",
                "enum": [
                    "create",
                    "format",
                    "grow",
                    "shrink",
                    "check",
                    "repair",
                    "scrub",
                    "trim",
                    "rescan",
                    "replace-device",
                    "add-device",
                    "remove-device",
                    "add-key",
                    "remove-key",
                    "import-token",
                    "remove-token",
                    "set-property",
                    "snapshot",
                    "clone",
                    "promote",
                    "import",
                    "export",
                    "unexport",
                    "attach",
                    "detach",
                    "activate",
                    "deactivate",
                    "assemble",
                    "start",
                    "stop",
                    "login",
                    "logout",
                    "open",
                    "close",
                    "mount",
                    "unmount",
                    "remount",
                    "rename",
                    "rebalance",
                    "rollback",
                    "destroy"
                ]
            },
            "applyPolicy": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["manual", "activation", "boot", "install"],
                        "default": "manual"
                    },
                    "allowDestructive": { "type": "boolean", "default": false },
                    "allowFormat": { "type": "boolean", "default": false },
                    "allowShrink": { "type": "boolean", "default": false },
                    "allowPotentialDataLoss": { "type": "boolean", "default": false },
                    "allowGrow": { "type": "boolean", "default": true },
                    "allowOffline": { "type": "boolean", "default": false },
                    "allowPropertyChanges": { "type": "boolean", "default": true },
                    "allowDeviceReplacement": { "type": "boolean", "default": true },
                    "allowRebalance": { "type": "boolean", "default": true },
                    "requireBackup": { "type": "boolean", "default": false },
                    "backupVerified": { "type": "boolean", "default": false },
                    "requireConfirmation": { "type": "boolean", "default": false },
                    "confirmation": { "type": "boolean", "default": false },
                    "requireConfirmationFile": { "type": ["string", "null"] },
                    "probeCurrent": {
                        "type": "boolean",
                        "description": "NixOS module helper that controls whether activation validation passes --probe-current."
                    },
                    "failOnBlocked": {
                        "type": "boolean",
                        "default": true,
                        "description": "NixOS module helper that controls whether activation uses apply and fails on blocked policy, or validate and reports blocked policy without failing the unit."
                    },
                    "scriptOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --script-out."
                    },
                    "reportOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --report-out."
                    },
                    "receiptOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --receipt-out."
                    }
                }
            }
        }
    })
}
