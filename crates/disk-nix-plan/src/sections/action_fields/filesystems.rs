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
