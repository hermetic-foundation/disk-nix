fn parse_size_bytes(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.ends_with('%') {
        return None;
    }

    let number_end = trimmed
        .find(|character: char| !(character.is_ascii_digit() || character == '.'))
        .unwrap_or(trimmed.len());
    let number = trimmed[..number_end].parse::<f64>().ok()?;
    let unit = trimmed[number_end..].trim().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "" | "b" => 1_f64,
        "k" | "kb" => 1_000_f64,
        "m" | "mb" => 1_000_000_f64,
        "g" | "gb" => 1_000_000_000_f64,
        "t" | "tb" => 1_000_000_000_000_f64,
        "p" | "pb" => 1_000_000_000_000_000_f64,
        "ki" | "kib" => 1024_f64,
        "mi" | "mib" => 1024_f64.powi(2),
        "gi" | "gib" => 1024_f64.powi(3),
        "ti" | "tib" => 1024_f64.powi(4),
        "pi" | "pib" => 1024_f64.powi(5),
        _ => return None,
    };

    let bytes = number * multiplier;
    bytes.is_finite().then_some(bytes.round() as u64)
}

fn blocked_action(action: &PlannedAction, policy: &ApplyPolicy) -> Option<BlockedAction> {
    let reason = if action.risk == RiskClass::Unsupported {
        Some("unsupported actions cannot be applied")
    } else if requires_backup(action) && policy.require_backup && !policy.backup_verified {
        Some("backup-required actions require backupVerified=true")
    } else if requires_confirmation(action) && policy.require_confirmation && !policy.confirmation {
        Some("confirmation-required actions require confirmation=true")
    } else if requires_confirmation(action)
        && policy.require_confirmation_file.is_some()
        && !policy.confirmation
    {
        Some(
            "confirmation-file policy requires confirmation=true after checking the configured file",
        )
    } else if action.risk == RiskClass::OfflineRequired && !policy.allow_offline {
        Some("offline-required actions require allowOffline=true")
    } else if action.operation == Operation::Format && !policy.allow_format {
        Some("format actions require allowFormat=true")
    } else if action.operation == Operation::Shrink && !policy.allow_shrink {
        Some("shrink actions require allowShrink=true")
    } else if action.risk == RiskClass::PotentialDataLoss && !policy.allow_potential_data_loss {
        Some("potential-data-loss actions require allowPotentialDataLoss=true")
    } else if action.operation == Operation::Grow && !policy.allow_grow {
        Some("grow actions require allowGrow=true")
    } else if matches!(
        action.operation,
        Operation::AddDevice | Operation::ReplaceDevice | Operation::RemoveDevice
    ) && !policy.allow_device_replacement
    {
        Some("device topology changes require allowDeviceReplacement=true")
    } else if action.operation == Operation::Rebalance && !policy.allow_rebalance {
        Some("rebalance actions require allowRebalance=true")
    } else if action.operation == Operation::SetProperty && !policy.allow_property_changes {
        Some("property changes require allowPropertyChanges=true")
    } else if action.operation == Operation::Format && !policy.allow_destructive {
        Some("format actions also require allowDestructive=true")
    } else if action.destructive
        || action.risk == RiskClass::Destructive
        || action.risk == RiskClass::Irreversible
    {
        (!policy.allow_destructive)
            .then_some("destructive or irreversible actions require allowDestructive=true")
    } else {
        None
    }?;

    Some(BlockedAction {
        id: action.id.clone(),
        operation: action.operation,
        risk: action.risk,
        reason: reason.to_string(),
    })
}

fn requires_backup(action: &PlannedAction) -> bool {
    action.destructive
        || matches!(
            action.risk,
            RiskClass::PotentialDataLoss | RiskClass::Destructive | RiskClass::Irreversible
        )
}

fn requires_confirmation(action: &PlannedAction) -> bool {
    requires_backup(action)
        || matches!(
            action.risk,
            RiskClass::OfflineRequired | RiskClass::Unsupported
        )
}

fn filesystem_context(
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    device: Option<String>,
    desired_size: Option<String>,
) -> ActionContext {
    ActionContext {
        collection: Some("filesystems".to_string()),
        name: Some(name.to_string()),
        target: Some(mountpoint.to_string()),
        device,
        fs_type: Some(fs_type.to_string()),
        mountpoint: Some(mountpoint.to_string()),
        desired_size,
        ..ActionContext::default()
    }
}

fn lifecycle_context(collection: &str, name: &str, object: &Value) -> ActionContext {
    ActionContext {
        collection: Some(collection.to_string()),
        name: Some(name.to_string()),
        target: lifecycle_target(collection, name, object),
        device: lifecycle_device(collection, object),
        devices: lifecycle_devices(collection, object),
        cache_set_uuid: metadata_string_field(
            object,
            &[
                "cacheSetUuid",
                "cacheSetUUID",
                "cache-set-uuid",
                "cache_set_uuid",
                "newCacheSetUuid",
                "newCacheSetUUID",
                "new-cache-set-uuid",
            ],
        ),
        rename_to: string_field(object, &["renameTo", "renameTarget", "newName"]),
        fs_type: string_field(object, &["fsType", "type"]),
        mountpoint: string_field(object, &["mountpoint", "path"])
            .or_else(|| name.starts_with('/').then(|| name.to_string())),
        desired_size: desired_size(object),
        physical_size: metadata_string_field(
            object,
            &[
                "physicalSize",
                "physical-size",
                "physical_size",
                "vdoPhysicalSize",
                "vdo-physical-size",
                "vdo_physical_size",
            ],
        ),
        start: string_field(object, &["start", "startOffset"]),
        end: string_field(object, &["end", "endOffset"]),
        partition_number: string_field(object, &["partitionNumber", "number"]),
        partition_type: string_field(object, &["partitionType", "type"]),
        level: string_field(object, &["level", "raidLevel"]),
        client: string_field(object, &["client"]),
        portal: lifecycle_portal(object),
        provider: metadata_string_field(
            object,
            &[
                "provider",
                "storageProvider",
                "storage-provider",
                "arrayProvider",
                "array-provider",
            ],
        ),
        backstore_type: metadata_string_field(
            object,
            &[
                "backstoreType",
                "backstore-type",
                "backstore_type",
                "lioBackstoreType",
                "lio-backstore-type",
                "lio_backstore_type",
            ],
        ),
        vendor: metadata_string_field(object, &["vendor", "arrayVendor", "array-vendor"]),
        array_id: metadata_string_field(
            object,
            &[
                "arrayId",
                "arrayID",
                "array-id",
                "array_id",
                "systemId",
                "system-id",
            ],
        ),
        storage_pool: metadata_string_field(
            object,
            &[
                "storagePool",
                "storage-pool",
                "poolName",
                "pool-name",
                "aggregate",
            ],
        ),
        volume_id: metadata_string_field(
            object,
            &[
                "volumeId",
                "volumeID",
                "volume-id",
                "volume_id",
                "volumeName",
            ],
        ),
        snapshot_id: metadata_string_field(
            object,
            &[
                "snapshotId",
                "snapshotID",
                "snapshot-id",
                "snapshot_id",
                "snapshotName",
            ],
        ),
        clone_source: metadata_string_field(
            object,
            &[
                "cloneSource",
                "clone-source",
                "sourceSnapshot",
                "source-snapshot",
                "sourceVolume",
                "source-volume",
            ],
        ),
        masking_group: metadata_string_field(
            object,
            &[
                "maskingGroup",
                "masking-group",
                "hostGroup",
                "host-group",
                "initiatorGroup",
                "initiator-group",
                "igroup",
            ],
        ),
        target_id: metadata_string_field(
            object,
            &["targetId", "targetID", "target-id", "target_id", "tid"],
        ),
        group: metadata_string_field(
            object,
            &[
                "group",
                "groupName",
                "group-name",
                "initiatorGroup",
                "initiator-group",
                "initiator_group",
            ],
        ),
        lun: metadata_string_field(
            object,
            &["lun", "lunId", "lun-id", "lunNumber", "lun-number"],
        ),
        options: lifecycle_options(object).or_else(|| {
            (collection == "mdRaids")
                .then(|| metadata_string_field(object, &["metadata"]))
                .flatten()
        }),
        rollback_options: metadata_string_field(
            object,
            &[
                "rollbackOptions",
                "rollback-options",
                "rollback_options",
                "previousOptions",
                "previous-options",
                "previous_options",
                "preApplyOptions",
                "pre-apply-options",
                "pre_apply_options",
            ],
        ),
        namespace_id: metadata_string_field(object, &["namespaceId", "nsid"]),
        controllers: metadata_string_field(object, &["controllers", "controllerId", "controller"]),
        key_slot: metadata_string_field(object, &["keySlot", "key-slot", "slot"]),
        key_file: metadata_string_field(object, &["keyFile", "key-file", "currentKeyFile"]),
        new_key_file: metadata_string_field(object, &["newKeyFile", "new-key-file"]),
        token_id: metadata_string_field(object, &["tokenId", "token-id", "token"]),
        token_file: metadata_string_field(object, &["tokenFile", "token-file", "jsonFile"]),
        read_only: object
            .get("readOnly")
            .or_else(|| object.get("readonly"))
            .and_then(Value::as_bool),
        property_assignments: property_assignments(object),
        rollback_value: metadata_string_field(
            object,
            &[
                "rollbackValue",
                "rollback-value",
                "rollback_value",
                "previousValue",
                "previous-value",
                "previous_value",
                "preApplyValue",
                "pre-apply-value",
                "pre_apply_value",
            ],
        ),
        ..ActionContext::default()
    }
}

fn lifecycle_device(collection: &str, object: &Value) -> Option<String> {
    let keys: &[&str] = if collection == "luns" || collection == "targetLuns" {
        &["device", "disk", "source", "path"]
    } else {
        &["device", "disk", "source"]
    };
    string_field(object, keys)
}

fn lifecycle_devices(collection: &str, object: &Value) -> Vec<String> {
    let keys: &[&str] = if collection == "luns" {
        &["devices", "devicePaths", "paths", "addDevices"]
    } else if collection == "targetLuns" {
        &[
            "initiators",
            "initiatorIqns",
            "clients",
            "devices",
            "addDevices",
        ]
    } else {
        &["devices", "addDevices"]
    };
    string_array_field(object, keys)
}

fn lifecycle_target(collection: &str, name: &str, object: &Value) -> Option<String> {
    if collection == "pools" || collection == "datasets" || collection == "zvols" {
        return string_field(object, &["target"]).or_else(|| Some(name.to_string()));
    }
    if let Some(target) = string_field(object, &["target", "path", "mountpoint"]) {
        return Some(target);
    }
    if collection == "caches" || collection == "mdRaids" || collection == "multipathMaps" {
        if let Some(device) = string_field(object, &["device", "disk", "source"])
            .filter(|target| lifecycle_device_can_be_target(collection, target))
        {
            return Some(device);
        }
    }
    Some(name.to_string())
}

fn lifecycle_device_can_be_target(collection: &str, target: &str) -> bool {
    matches!(
        (collection, target),
        ("caches", target) if target.starts_with("/dev/bcache")
    ) || matches!(
        (collection, target),
        ("mdRaids", target) if target.starts_with("/dev/md")
    ) || matches!(
        (collection, target),
        ("multipathMaps", target)
            if target.starts_with("mpath") || target.starts_with("/dev/mapper/")
    )
}

fn string_field(object: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        object.get(*key).and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
    })
}

fn string_array_field(object: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| {
            object.get(*key).and_then(|value| {
                value.as_array().map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToString::to_string))
                        .collect::<Vec<_>>()
                })
            })
        })
        .unwrap_or_default()
}

fn desired_size(object: &Value) -> Option<String> {
    object
        .get("desiredSize")
        .or_else(|| object.get("targetSize"))
        .or_else(|| object.get("size"))
        .and_then(|value| match value {
            Value::String(size) => Some(size.clone()),
            Value::Number(size) => Some(size.to_string()),
            _ => None,
        })
}

fn lifecycle_options(object: &Value) -> Option<String> {
    string_field(object, &["options"])
        .or_else(|| {
            let options = string_array_field(object, &["options"]);
            if options.is_empty() {
                None
            } else {
                Some(options.join(","))
            }
        })
        .or_else(|| {
            object
                .get("properties")
                .and_then(|properties| string_field(properties, &["options"]))
        })
}

fn property_assignments(object: &Value) -> Vec<String> {
    object
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .map(|(property, value)| format!("{property}={}", property_value(value)))
                .collect()
        })
        .unwrap_or_default()
}

fn lifecycle_portal(object: &Value) -> Option<String> {
    string_field(object, &["portal"]).or_else(|| {
        object
            .get("metadata")
            .and_then(|metadata| string_field(metadata, &["portal"]))
    })
}

fn metadata_string_field(object: &Value, keys: &[&str]) -> Option<String> {
    string_field(object, keys).or_else(|| {
        object
            .get("metadata")
            .and_then(|metadata| string_field(metadata, keys))
    })
}

fn add_filesystem_actions(actions: &mut Vec<PlannedAction>, name: &str, filesystem: &Value) {
    let mountpoint = filesystem
        .get("mountpoint")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let fs_type = filesystem
        .get("fsType")
        .or_else(|| filesystem.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let resize_policy = filesystem
        .get("resizePolicy")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let preserve_data = filesystem
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let desired_size = desired_size(filesystem);
    let device = string_field(filesystem, &["device", "disk"]);

    match resize_policy {
        "grow-only" => actions.push(PlannedAction {
            id: format!("filesystem:{name}:grow"),
            description: format!(
                "allow non-destructive growth for {fs_type} filesystem at {mountpoint}"
            ),
            operation: Operation::Grow,
            risk: RiskClass::Online,
            destructive: false,
            context: filesystem_context(
                name,
                mountpoint,
                fs_type,
                device.clone(),
                desired_size.clone(),
            ),
            advice: None,
        }),
        "shrink-allowed" => actions.push(filesystem_shrink_action(
            name,
            mountpoint,
            fs_type,
            device.clone(),
            desired_size.clone(),
        )),
        _ => actions.push(PlannedAction {
            id: format!("filesystem:{name}:inspect"),
            description: format!("inspect {fs_type} filesystem declaration at {mountpoint}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: filesystem_context(
                name,
                mountpoint,
                fs_type,
                device.clone(),
                desired_size.clone(),
            ),
            advice: None,
        }),
    }

    if !preserve_data {
        actions.push(PlannedAction {
            id: format!("filesystem:{name}:preserve-data-disabled"),
            description: format!(
                "preserveData=false permits destructive replacement for filesystem at {mountpoint}"
            ),
            operation: Operation::Format,
            risk: RiskClass::Destructive,
            destructive: true,
            context: filesystem_context(
                name,
                mountpoint,
                fs_type,
                device.clone(),
                desired_size.clone(),
            ),
            advice: Some(Advice {
                summary: "formatting or replacing a filesystem destroys existing data".to_string(),
                alternatives: vec![
                    "leave preserveData=true and request a grow or property-only update"
                        .to_string(),
                    "migrate data to a new filesystem before replacing this one".to_string(),
                    "require an explicit backup and confirmation policy before applying"
                        .to_string(),
                ],
            }),
        });
    }

    if let Some(
        operation @ (Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::Rebalance
        | Operation::Mount
        | Operation::Unmount
        | Operation::Rescan
        | Operation::Remount),
    ) = filesystem
        .get("operation")
        .or_else(|| filesystem.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation)
    {
        let (risk, destructive, advice) = classify_operation("filesystems", operation, filesystem);
        actions.push(PlannedAction {
            id: format!("filesystems:{name}:{}", operation_id(operation)),
            description: format!(
                "plan {} operation for filesystem {name}",
                operation_id(operation)
            ),
            operation,
            risk,
            destructive,
            context: ActionContext {
                collection: Some("filesystems".to_string()),
                options: lifecycle_options(filesystem),
                rollback_options: metadata_string_field(
                    filesystem,
                    &[
                        "rollbackOptions",
                        "rollback-options",
                        "rollback_options",
                        "previousOptions",
                        "previous-options",
                        "previous_options",
                        "preApplyOptions",
                        "pre-apply-options",
                        "pre_apply_options",
                    ],
                ),
                property_assignments: property_assignments(filesystem),
                ..filesystem_context(
                    name,
                    mountpoint,
                    fs_type,
                    device.clone(),
                    desired_size.clone(),
                )
            },
            advice,
        });
    }

    add_device_membership_actions(actions, "filesystems", name, filesystem);
    add_filesystem_property_actions(actions, name, mountpoint, fs_type, filesystem);
}

fn add_filesystem_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    filesystem: &Value,
) {
    let Some(properties) = filesystem.get("properties").and_then(Value::as_object) else {
        return;
    };
    let desired_size = desired_size(filesystem);

    for (property, value) in properties {
        if fs_type == "btrfs" && is_btrfs_balance_filter_property(property) {
            continue;
        }
        let (risk, advice) = classify_filesystem_property_change(fs_type, property, value);
        actions.push(PlannedAction {
            id: format!("filesystems:{name}:set-property:{property}"),
            description: format!("set property {property} on filesystem {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                property_assignments: property_assignments(filesystem),
                rollback_value: metadata_string_field(
                    filesystem,
                    &[
                        "rollbackValue",
                        "rollback-value",
                        "rollback_value",
                        "previousValue",
                        "previous-value",
                        "previous_value",
                        "preApplyValue",
                        "pre-apply-value",
                        "pre_apply_value",
                    ],
                ),
                ..filesystem_context(
                    name,
                    mountpoint,
                    fs_type,
                    string_field(filesystem, &["device", "disk"]),
                    desired_size.clone(),
                )
            },
            advice,
        });
    }
}

fn classify_filesystem_property_change(
    fs_type: &str,
    property: &str,
    value: &Value,
) -> (RiskClass, Option<Advice>) {
    if is_fat_filesystem_uuid_property(fs_type, property)
        && !is_valid_fat_volume_id(&property_value(value))
    {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem volume ID {} is not a valid FAT volume ID",
                    property_value(value)
                ),
                alternatives: vec![
                    "use an 8-hex-digit FAT volume ID such as A1B2-C3D4 or A1B2C3D4".to_string(),
                    "update NixOS fileSystems and boot references instead of changing the FAT volume ID when possible"
                        .to_string(),
                    "recreate the FAT filesystem only when data preservation is not required"
                        .to_string(),
                ],
            }),
        );
    }

    if is_ntfs_filesystem_uuid_property(fs_type, property)
        && !is_valid_ntfs_volume_serial(&property_value(value))
    {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem volume serial {} is not a valid NTFS serial",
                    property_value(value)
                ),
                alternatives: vec![
                    "use a 16-hex-digit NTFS volume serial such as 01234567-89ABCDEF or 0123456789ABCDEF"
                        .to_string(),
                    "update NixOS fileSystems and dependent mount references instead of changing the NTFS serial when possible"
                        .to_string(),
                    "leave the NTFS serial unchanged unless consumers explicitly depend on it"
                        .to_string(),
                ],
            }),
        );
    }

    if is_exfat_filesystem_uuid_property(fs_type, property)
        && !is_valid_exfat_volume_serial(&property_value(value))
    {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem volume serial {} is not a valid exFAT serial",
                    property_value(value)
                ),
                alternatives: vec![
                    "use an 8-hex-digit exFAT volume serial such as A1B2-C3D4 or A1B2C3D4"
                        .to_string(),
                    "update NixOS fileSystems and dependent mount references instead of changing the exFAT serial when possible"
                        .to_string(),
                    "leave the exFAT serial unchanged unless consumers explicitly depend on it"
                        .to_string(),
                ],
            }),
        );
    }

    if is_filesystem_uuid_property_supported(fs_type, property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "{fs_type} filesystem UUID updates mutate filesystem identity metadata"
                ),
                alternatives: vec![
                    "prefer updating references to the current UUID when possible".to_string(),
                    "update NixOS fileSystems, initrd, bootloader, and dependent mount references before changing the UUID"
                        .to_string(),
                    "perform UUID changes with the filesystem unmounted and a recovery path available"
                        .to_string(),
                ],
            }),
        );
    }
    if is_filesystem_property_supported(fs_type, property) {
        return (RiskClass::Safe, None);
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!(
                "{fs_type} filesystem property {property} is not mapped to a safe command"
            ),
            alternatives: vec![
                "use label, filesystem.label, btrfs.label, exfat.label, ext.label, f2fs.label, fat.label, ntfs.label, vfat.label, or xfs.label when changing filesystem labels"
                    .to_string(),
                "use uuid, filesystem.uuid, btrfs.uuid, exfat.uuid, ext.uuid, fat.uuid, ntfs.uuid, vfat.uuid, or xfs.uuid when changing supported filesystem UUIDs"
                    .to_string(),
                "use ZFS dataset declarations for arbitrary zfs set property updates".to_string(),
                "apply unsupported filesystem property changes manually after reviewing filesystem-specific tooling"
                    .to_string(),
            ],
        }),
    )
}

fn is_filesystem_property_supported(fs_type: &str, property: &str) -> bool {
    match fs_type {
        "btrfs" => matches!(
            property,
            "label"
                | "btrfs.label"
                | "filesystem.label"
                | "uuid"
                | "btrfs.uuid"
                | "filesystem.uuid"
        ),
        "ext2" | "ext3" | "ext4" => {
            matches!(
                property,
                "label"
                    | "ext.label"
                    | "filesystem.label"
                    | "uuid"
                    | "ext.uuid"
                    | "filesystem.uuid"
            )
        }
        "fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat" => matches!(
            property,
            "label"
                | "fat.label"
                | "vfat.label"
                | "filesystem.label"
                | "uuid"
                | "fat.uuid"
                | "vfat.uuid"
                | "filesystem.uuid"
                | "volumeId"
                | "volume-id"
                | "fat.volume-id"
                | "vfat.volume-id"
        ),
        "ntfs" | "ntfs3" => matches!(
            property,
            "label"
                | "ntfs.label"
                | "filesystem.label"
                | "uuid"
                | "ntfs.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "ntfs.serial"
                | "ntfs.volume-serial"
        ),
        "exfat" => matches!(
            property,
            "label"
                | "exfat.label"
                | "filesystem.label"
                | "uuid"
                | "exfat.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "exfat.serial"
                | "exfat.volume-serial"
        ),
        "f2fs" => matches!(property, "label" | "f2fs.label" | "filesystem.label"),
        "xfs" => matches!(
            property,
            "label" | "xfs.label" | "filesystem.label" | "uuid" | "xfs.uuid" | "filesystem.uuid"
        ),
        "zfs" => true,
        _ => false,
    }
}

fn is_filesystem_uuid_property_supported(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        ("btrfs", "uuid" | "btrfs.uuid" | "filesystem.uuid")
            | (
                "ext2" | "ext3" | "ext4",
                "uuid" | "ext.uuid" | "filesystem.uuid"
            )
            | (
                "fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat",
                "uuid"
                    | "fat.uuid"
                    | "vfat.uuid"
                    | "filesystem.uuid"
                    | "volumeId"
                    | "volume-id"
                    | "fat.volume-id"
                    | "vfat.volume-id"
            )
            | (
                "ntfs" | "ntfs3",
                "uuid"
                    | "ntfs.uuid"
                    | "filesystem.uuid"
                    | "serial"
                    | "volumeSerial"
                    | "volume-serial"
                    | "ntfs.serial"
                    | "ntfs.volume-serial"
            )
            | (
                "exfat",
                "uuid"
                    | "exfat.uuid"
                    | "filesystem.uuid"
                    | "serial"
                    | "volumeSerial"
                    | "volume-serial"
                    | "exfat.serial"
                    | "exfat.volume-serial"
            )
            | ("xfs", "uuid" | "xfs.uuid" | "filesystem.uuid")
    )
}

fn is_fat_filesystem_uuid_property(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        (
            "fat" | "fat12" | "fat16" | "fat32" | "msdos" | "vfat",
            "uuid"
                | "fat.uuid"
                | "vfat.uuid"
                | "filesystem.uuid"
                | "volumeId"
                | "volume-id"
                | "fat.volume-id"
                | "vfat.volume-id"
        )
    )
}

fn is_valid_fat_volume_id(value: &str) -> bool {
    fat_volume_id(value).is_some()
}

fn fat_volume_id(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn is_ntfs_filesystem_uuid_property(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        (
            "ntfs" | "ntfs3",
            "uuid"
                | "ntfs.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "ntfs.serial"
                | "ntfs.volume-serial"
        )
    )
}

fn is_valid_ntfs_volume_serial(value: &str) -> bool {
    ntfs_volume_serial(value).is_some()
}

fn ntfs_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 16
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn is_exfat_filesystem_uuid_property(fs_type: &str, property: &str) -> bool {
    matches!(
        (fs_type, property),
        (
            "exfat",
            "uuid"
                | "exfat.uuid"
                | "filesystem.uuid"
                | "serial"
                | "volumeSerial"
                | "volume-serial"
                | "exfat.serial"
                | "exfat.volume-serial"
        )
    )
}

fn is_valid_exfat_volume_serial(value: &str) -> bool {
    exfat_volume_serial(value).is_some()
}

fn exfat_volume_serial(value: &str) -> Option<String> {
    let normalized: String = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if normalized.len() == 8
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(normalized.to_ascii_uppercase())
    } else {
        None
    }
}

fn is_btrfs_balance_filter_property(property: &str) -> bool {
    let property = property
        .strip_prefix("btrfs.balance.")
        .or_else(|| property.strip_prefix("balance."))
        .unwrap_or(property);
    matches!(
        property,
        "data" | "d" | "metadata" | "meta" | "m" | "system" | "s"
    )
}

fn add_swap_actions(actions: &mut Vec<PlannedAction>, name: &str, swap: &Value) {
    let device =
        string_field(swap, &["target", "path", "device"]).unwrap_or_else(|| name.to_string());
    let operation = swap
        .get("operation")
        .or_else(|| swap.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let preserve_data = swap
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let desired_size = desired_size(swap);
    let context = ActionContext {
        collection: Some("swaps".to_string()),
        name: Some(name.to_string()),
        target: Some(device.clone()),
        device: Some(device.clone()),
        desired_size: desired_size.clone(),
        ..ActionContext::default()
    };

    match operation {
        Some(Operation::Grow) => actions.push(PlannedAction {
            id: format!("swaps:{name}:grow"),
            description: format!("grow swap backing storage for {device}"),
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary:
                    "swap growth requires disabling active swap before resizing backing storage"
                        .to_string(),
                alternatives: vec![
                    "add a second swap device before resizing this one".to_string(),
                    "disable swap, resize backing storage, recreate the signature, and re-enable"
                        .to_string(),
                    "verify memory pressure and hibernation dependencies before disabling swap"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Rescan) => actions.push(PlannedAction {
            id: format!("swaps:{name}:rescan"),
            description: format!("refresh swap inventory for {device}"),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "swap rescan refreshes signature, activation, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use grow when backing swap capacity must change".to_string(),
                    "use format only when replacing the swap signature is intended".to_string(),
                    "verify resume and hibernation references before changing swap identity"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Deactivate | Operation::Stop) => actions.push(PlannedAction {
            id: format!("swaps:{name}:deactivate"),
            description: format!("disable active swap on {device}"),
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "swap deactivation runs swapoff without removing the swap signature"
                    .to_string(),
                alternatives: vec![
                    "add replacement swap capacity before disabling active swap".to_string(),
                    "use destroy only when the swap signature should be removed".to_string(),
                    "verify resume and hibernation references before disabling swap".to_string(),
                ],
            }),
        }),
        Some(Operation::Destroy) => actions.push(PlannedAction {
            id: format!("swaps:{name}:destroy"),
            description: format!("disable swap and remove swap signature from {device}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: context.clone(),
            advice: Some(Advice {
                summary:
                    "swap destruction disables active swap and removes swap signature metadata"
                        .to_string(),
                alternatives: vec![
                    "use operation = \"deactivate\" to run swapoff without removing the signature"
                        .to_string(),
                    "remove or update NixOS swapDevices before deleting the swap signature"
                        .to_string(),
                    "verify resume and hibernation references before wiping swap metadata"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Create | Operation::Format) => actions.push(swap_format_action(
            name,
            &device,
            desired_size,
            "create or refresh swap signature",
        )),
        _ if !preserve_data => actions.push(swap_format_action(
            name,
            &device,
            desired_size,
            "preserveData=false permits recreating the swap signature",
        )),
        _ => actions.push(PlannedAction {
            id: format!("swaps:{name}:inspect"),
            description: format!("inspect swap declaration for {device}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: context.clone(),
            advice: None,
        }),
    }

    add_swap_property_actions(actions, name, swap, &context);
}

fn add_swap_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    swap: &Value,
    context: &ActionContext,
) {
    let Some(properties) = swap.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_swap_property_change(property);
        actions.push(PlannedAction {
            id: format!("swaps:{name}:set-property:{property}"),
            description: format!("set swap property {property} on {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                rollback_value: metadata_string_field(
                    swap,
                    &[
                        "rollbackValue",
                        "rollback-value",
                        "rollback_value",
                        "previousValue",
                        "previous-value",
                        "previous_value",
                        "preApplyValue",
                        "pre-apply-value",
                        "pre_apply_value",
                    ],
                ),
                ..context.clone()
            },
            advice,
        });
    }
}

fn classify_swap_property_change(property: &str) -> (RiskClass, Option<Advice>) {
    if is_swap_identity_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "swap label and UUID updates mutate swap signature identity".to_string(),
                alternatives: vec![
                    "prefer updating NixOS swapDevices references to the current identity when possible"
                        .to_string(),
                    "disable active swap and verify hibernation/resume references before changing identity"
                        .to_string(),
                    "use a stable device path instead of changing swap UUID when consumers allow it"
                        .to_string(),
                ],
            }),
        );
    }
    if is_swap_priority_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "swap priority updates reactivate the reviewed swap target".to_string(),
                alternatives: vec![
                    "prefer changing NixOS swapDevices priority for steady-state configuration"
                        .to_string(),
                    "review memory pressure and hibernation/resume state before swapoff".to_string(),
                    "use a temporary additional swap device before changing priority on busy systems"
                        .to_string(),
                ],
            }),
        );
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!("swap property {property} is not mapped to a safe command"),
            alternatives: vec![
                "use label, swap.label, uuid, swap.uuid, priority, or swap.priority for supported swap changes"
                    .to_string(),
                "recreate the swap signature with preserveData=false only when overwriting metadata is intended"
                    .to_string(),
                "apply unsupported swap changes manually after reviewing util-linux swap tools"
                    .to_string(),
            ],
        }),
    )
}

fn is_swap_identity_property(property: &str) -> bool {
    matches!(property, "label" | "swap.label" | "uuid" | "swap.uuid")
}

fn is_swap_priority_property(property: &str) -> bool {
    matches!(property, "priority" | "swap.priority")
}

fn add_zram_actions(actions: &mut Vec<PlannedAction>, zram: &Map<String, Value>) {
    let operation = zram
        .get("operation")
        .or_else(|| zram.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let context = ActionContext {
        collection: Some("zram".to_string()),
        name: Some("zram".to_string()),
        target: Some("zram".to_string()),
        ..ActionContext::default()
    };

    match operation {
        Some(Operation::Rescan) => actions.push(PlannedAction {
            id: "zram:rescan".to_string(),
            description: "refresh zram compressed swap inventory".to_string(),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "zram rescan refreshes generated compressed swap state".to_string(),
                alternatives: vec![
                    "review zramctl output before changing generated zramSwap settings".to_string(),
                    "coordinate swapoff and setup when active zram devices must be recreated"
                        .to_string(),
                ],
            }),
        }),
        _ => actions.push(PlannedAction {
            id: "zram:inspect".to_string(),
            description: "inspect zram compressed swap declaration".to_string(),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: context.clone(),
            advice: None,
        }),
    }

    add_zram_property_actions(actions, zram, &context);
}

fn add_zram_property_actions(
    actions: &mut Vec<PlannedAction>,
    zram: &Map<String, Value>,
    context: &ActionContext,
) {
    let Some(properties) = zram.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        actions.push(PlannedAction {
            id: format!("zram:set-property:{property}"),
            description: format!("set zram property {property}"),
            operation: Operation::SetProperty,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..context.clone()
            },
            advice: Some(Advice {
                summary: format!("zram property {property} requires generator reconciliation"),
                alternatives: vec![
                    "use services.disk-nix.zram options to derive NixOS zramSwap".to_string(),
                    "run a zram rescan before recreating active compressed swap devices".to_string(),
                    "coordinate swapoff before changing live zram algorithm, priority, size, or writeback device"
                        .to_string(),
                ],
            }),
        });
    }
}

fn swap_format_action(
    name: &str,
    device: &str,
    desired_size: Option<String>,
    description: &str,
) -> PlannedAction {
    PlannedAction {
        id: format!("swaps:{name}:format"),
        description: format!("{description} on {device}"),
        operation: Operation::Format,
        risk: RiskClass::Destructive,
        destructive: true,
        context: ActionContext {
            collection: Some("swaps".to_string()),
            name: Some(name.to_string()),
            target: Some(device.to_string()),
            device: Some(device.to_string()),
            desired_size,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: "creating a swap signature overwrites existing metadata on the target"
                .to_string(),
            alternatives: vec![
                "use an additional swap file or device instead of replacing this target"
                    .to_string(),
                "verify the target contains no filesystem or encrypted data before mkswap"
                    .to_string(),
                "set preserveData=true for inspection-only planning".to_string(),
            ],
        }),
    }
}

fn add_luks_actions(actions: &mut Vec<PlannedAction>, name: &str, luks: &Value) {
    let device = string_field(luks, &["device"]);
    let device_label = device.as_deref().unwrap_or("<device>");
    let mapper_name = string_field(
        luks,
        &["target", "mapperName", "mapper-name", "mapper", "name"],
    )
    .unwrap_or_else(|| name.to_string());
    let operation = luks
        .get("operation")
        .or_else(|| luks.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let preserve_data = luks
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let context = ActionContext {
        collection: Some("luks.devices".to_string()),
        name: Some(name.to_string()),
        target: Some(mapper_name.clone()),
        device: device.clone(),
        property_assignments: property_assignments(luks),
        ..ActionContext::default()
    };
    let has_properties = luks
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| !properties.is_empty());

    match operation {
        Some(Operation::Grow) => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:grow"),
            description: format!("resize LUKS mapping {mapper_name} on {device_label}"),
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context,
            advice: Some(Advice {
                summary: "LUKS resize requires backing-device growth and mapper coordination"
                    .to_string(),
                alternatives: vec![
                    "grow the partition, LUN, or volume before resizing the LUKS mapper"
                        .to_string(),
                    "verify the mapping is open and dependent layers are paused or coordinated"
                        .to_string(),
                    "resize filesystems only after cryptsetup resize reports the new size"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Destroy | Operation::Close) => {
            actions.push(luks_close_action(
                name,
                &mapper_name,
                device_label,
                operation.expect("operation already matched"),
                context,
            ));
        }
        Some(Operation::Open) => {
            actions.push(luks_open_action(
                name,
                &mapper_name,
                device_label,
                Operation::Open,
                context,
            ));
        }
        Some(Operation::Create) if preserve_data => {
            actions.push(luks_open_action(
                name,
                &mapper_name,
                device_label,
                Operation::Create,
                context,
            ));
        }
        Some(Operation::Create | Operation::Format) => actions.push(luks_format_action(
            name,
            device.clone(),
            &mapper_name,
            "create or replace LUKS container",
        )),
        _ if !preserve_data => actions.push(luks_format_action(
            name,
            device.clone(),
            &mapper_name,
            "preserveData=false permits replacing the LUKS container",
        )),
        _ if !has_properties => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:inspect"),
            description: format!("inspect LUKS declaration {mapper_name} on {device_label}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context,
            advice: None,
        }),
        _ => {}
    }

    add_luks_property_actions(actions, name, &mapper_name, device, luks);
}

fn luks_format_action(
    name: &str,
    device: Option<String>,
    mapper_name: &str,
    description: &str,
) -> PlannedAction {
    let device_label = device.as_deref().unwrap_or("<device>");
    PlannedAction {
        id: format!("luks.devices:{name}:format"),
        description: format!("{description} on {device_label}"),
        operation: Operation::Format,
        risk: RiskClass::Destructive,
        destructive: true,
        context: ActionContext {
            collection: Some("luks.devices".to_string()),
            name: Some(name.to_string()),
            target: Some(mapper_name.to_string()),
            device,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: "formatting a LUKS container destroys access to existing encrypted data"
                .to_string(),
            alternatives: vec![
                "open and reuse the existing LUKS container when data must be preserved"
                    .to_string(),
                "back up headers with cryptsetup luksHeaderBackup before destructive work"
                    .to_string(),
                "create a new encrypted target and migrate data before switching mounts"
                    .to_string(),
            ],
        }),
    }
}

fn luks_open_action(
    name: &str,
    mapper_name: &str,
    device_label: &str,
    operation: Operation,
    context: ActionContext,
) -> PlannedAction {
    PlannedAction {
        id: format!("luks.devices:{name}:{}", operation_id(operation)),
        description: format!("open existing LUKS container {device_label} as {mapper_name}"),
        operation,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context,
        advice: Some(Advice {
            summary: "opening a LUKS mapper changes active device topology without formatting"
                .to_string(),
            alternatives: vec![
                "verify the backing device is the intended LUKS container before opening"
                    .to_string(),
                "use preserveData=false or operation=format only when replacing the header"
                    .to_string(),
                "create filesystems or LVM layers only after the mapper appears".to_string(),
            ],
        }),
    }
}

fn luks_close_action(
    name: &str,
    mapper_name: &str,
    device_label: &str,
    operation: Operation,
    context: ActionContext,
) -> PlannedAction {
    PlannedAction {
        id: format!("luks.devices:{name}:{}", operation_id(operation)),
        description: format!("close LUKS mapping {mapper_name} without formatting {device_label}"),
        operation,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context,
        advice: Some(Advice {
            summary: "closing a LUKS mapper requires dependent layers to be stopped".to_string(),
            alternatives: vec![
                "unmount filesystems and deactivate LVM volumes before closing the mapper"
                    .to_string(),
                "leave the LUKS header and backing device intact for later reopen".to_string(),
                "use preserveData=false only when reformatting is explicitly intended".to_string(),
            ],
        }),
    }
}

fn add_luks_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    mapper_name: &str,
    device: Option<String>,
    luks: &Value,
) {
    let Some(properties) = luks.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_luks_device_property_change(property);
        actions.push(PlannedAction {
            id: format!("luks.devices:{name}:set-property:{property}"),
            description: format!("set LUKS header property {property} on {mapper_name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                collection: Some("luks.devices".to_string()),
                name: Some(name.to_string()),
                target: Some(mapper_name.to_string()),
                device: device.clone(),
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                property_assignments: property_assignments(luks),
                rollback_value: metadata_string_field(
                    luks,
                    &[
                        "rollbackValue",
                        "rollback-value",
                        "rollback_value",
                        "previousValue",
                        "previous-value",
                        "previous_value",
                        "preApplyValue",
                        "pre-apply-value",
                        "pre_apply_value",
                    ],
                ),
                ..ActionContext::default()
            },
            advice,
        });
    }
}

fn classify_luks_device_property_change(property: &str) -> (RiskClass, Option<Advice>) {
    if is_luks_identity_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS header property {property} updates encrypted-container identity metadata"
                ),
                alternatives: vec![
                    "prefer updating consumers to stable by-id paths when possible".to_string(),
                    "back up the LUKS header before changing header identity metadata".to_string(),
                    "verify initrd, crypttab, and NixOS LUKS references after identity changes"
                        .to_string(),
                ],
            }),
        );
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!("LUKS header property {property} is not mapped to a safe command"),
            alternatives: vec![
                "use label, luks.label, subsystem, luks.subsystem, uuid, or luks.uuid for supported LUKS identity changes"
                    .to_string(),
                "use luksKeyslots or luksTokens declarations for access-material changes"
                    .to_string(),
                "apply unsupported LUKS header changes manually after reviewing cryptsetup documentation"
                    .to_string(),
            ],
        }),
    )
}

fn is_luks_identity_property(property: &str) -> bool {
    matches!(
        property,
        "label"
            | "luks.label"
            | "cryptsetup.label"
            | "subsystem"
            | "luks.subsystem"
            | "cryptsetup.subsystem"
            | "uuid"
            | "luks.uuid"
            | "cryptsetup.uuid"
    )
}
