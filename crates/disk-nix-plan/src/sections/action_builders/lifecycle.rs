fn filesystem_shrink_action(
    name: &str,
    mountpoint: &str,
    fs_type: &str,
    device: Option<String>,
    desired_size: Option<String>,
) -> PlannedAction {
    let (risk, advice) = match fs_type {
        "xfs" => (
            RiskClass::Unsupported,
            Advice {
                summary: "XFS does not support shrinking in place".to_string(),
                alternatives: vec![
                    "create a new smaller filesystem and migrate data".to_string(),
                    "snapshot or back up the current filesystem before migration".to_string(),
                    "switch the mount to the replacement filesystem after verification".to_string(),
                ],
            },
        ),
        "btrfs" => (
            RiskClass::PotentialDataLoss,
            Advice {
                summary:
                    "Btrfs shrink requires enough data and metadata slack before resizing"
                        .to_string(),
                alternatives: vec![
                    "run a balance to reduce allocated chunks before shrink".to_string(),
                    "remove or replace devices only after checking filesystem usage".to_string(),
                    "take a snapshot or backup before resizing".to_string(),
                ],
            },
        ),
        "ext2" | "ext3" | "ext4" => (
            RiskClass::PotentialDataLoss,
            Advice {
                summary: format!("{fs_type} shrink requires offline filesystem checks"),
                alternatives: vec![
                    "unmount the filesystem and run fsck before resize".to_string(),
                    "take and verify a backup before shrinking".to_string(),
                    "create a new smaller filesystem and migrate data when downtime is not acceptable"
                        .to_string(),
                ],
            },
        ),
        _ => (
            RiskClass::PotentialDataLoss,
            Advice {
                summary:
                    "shrinking can require offline checks and filesystem-specific migration paths"
                        .to_string(),
                alternatives: vec![
                    "prefer grow-only policies for live systems".to_string(),
                    "create a new smaller filesystem and migrate data when shrink support is absent"
                        .to_string(),
                    "take and verify a backup before any shrink attempt".to_string(),
                ],
            },
        ),
    };

    PlannedAction {
        id: format!("filesystem:{name}:shrink"),
        description: format!("allow shrink evaluation for {fs_type} filesystem at {mountpoint}"),
        operation: Operation::Shrink,
        risk,
        destructive: false,
        context: filesystem_context(name, mountpoint, fs_type, device, desired_size),
        advice: Some(advice),
    }
}

fn add_lifecycle_actions(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    add_requested_operation(actions, collection, name, object);
    add_device_membership_actions(actions, collection, name, object);
    add_property_actions(actions, collection, name, object);
    add_destroy_guard(actions, collection, name, object);
}

fn add_requested_operation(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    let Some(operation) = object
        .get("operation")
        .or_else(|| object.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation)
    else {
        return;
    };
    let (risk, destructive, advice) = classify_operation(collection, operation, object);
    let operation_name = match operation {
        Operation::AddKey
        | Operation::RemoveKey
        | Operation::ImportToken
        | Operation::RemoveToken => operation_id(operation).to_string(),
        _ => format!("{operation:?}").to_ascii_lowercase(),
    };
    actions.push(PlannedAction {
        id: format!("{collection}:{name}:{operation_name}").to_ascii_lowercase(),
        description: format!(
            "plan {} operation for {collection} {name}",
            operation_label(operation)
        ),
        operation,
        risk,
        destructive,
        context: lifecycle_context(collection, name, object),
        advice,
    });
}

fn add_device_membership_actions(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    if let Some(devices) = object.get("addDevices").and_then(Value::as_array) {
        for device in devices.iter().filter_map(Value::as_str) {
            let (risk, advice) = classify_add_device(collection);
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:add-device:{device}"),
                description: format!("add device {device} to {collection} {name}"),
                operation: Operation::AddDevice,
                risk,
                destructive: false,
                context: ActionContext {
                    device: Some(device.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice,
            });
        }
    }

    if let Some(devices) = object.get("removeDevices").and_then(Value::as_array) {
        for device in devices.iter().filter_map(Value::as_str) {
            let (risk, advice) = classify_remove_device(collection);
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:remove-device:{device}"),
                description: format!("remove device {device} from {collection} {name}"),
                operation: Operation::RemoveDevice,
                risk,
                destructive: false,
                context: ActionContext {
                    device: Some(device.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice: Some(advice),
            });
        }
    }

    if let Some(replacements) = object.get("replaceDevices").and_then(Value::as_object) {
        for (from, to) in replacements
            .iter()
            .filter_map(|(from, to)| to.as_str().map(|replacement| (from.as_str(), replacement)))
        {
            let (risk, advice) = classify_replace_device(collection);
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:replace-device:{from}"),
                description: format!("replace device {from} with {to} in {collection} {name}"),
                operation: Operation::ReplaceDevice,
                risk,
                destructive: false,
                context: ActionContext {
                    device: Some(from.to_string()),
                    replacement: Some(to.to_string()),
                    ..lifecycle_context(collection, name, object)
                },
                advice: Some(advice),
            });
        }
    }
}

fn add_property_actions(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_property_change(collection, property, value);
        actions.push(PlannedAction {
            id: format!("{collection}:{name}:set-property:{property}"),
            description: format!("set property {property} on {collection} {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..lifecycle_context(collection, name, object)
            },
            advice,
        });
    }
}

fn classify_property_change(
    collection: &str,
    property: &str,
    value: &Value,
) -> (RiskClass, Option<Advice>) {
    if collection == "btrfsSubvolumes" && !is_btrfs_subvolume_property_supported(property) {
        return (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!("Btrfs subvolume property {property} is not mapped to a safe command"),
                alternatives: vec![
                    "use readOnly, readonly, ro, btrfs.readonly, or btrfs.ro for read-only toggles"
                        .to_string(),
                    "apply unsupported Btrfs subvolume property changes manually after reviewing btrfs property documentation"
                        .to_string(),
                ],
            }),
        );
    }

    if collection == "vdoVolumes" {
        return classify_vdo_property_change(property, value);
    }

    if collection == "lvmCaches" {
        return (
            RiskClass::Safe,
            Some(Advice {
                summary: format!(
                    "LVM cache property {property} changes cache behavior on the origin LV"
                ),
                alternatives: vec![
                    "prefer writethrough mode before cache detach or replacement".to_string(),
                    "verify dirty cache data is drained before disabling writeback".to_string(),
                    "review lvs cache fields after changing cache policy or mode".to_string(),
                ],
            }),
        );
    }

    if collection == "luksKeyslots" {
        return classify_luks_keyslot_property_change(property, value);
    }

    if collection == "luksTokens" {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS access property {property} updates encrypted-container access material"
                ),
                alternatives: vec![
                    "verify at least one independent recovery key before changing key material"
                        .to_string(),
                    "add and test replacement access before removing the old keyslot or token"
                        .to_string(),
                    "back up the LUKS header before access changes".to_string(),
                ],
            }),
        );
    }

    (RiskClass::Safe, None)
}

fn classify_luks_keyslot_property_change(
    property: &str,
    value: &Value,
) -> (RiskClass, Option<Advice>) {
    match luks_keyslot_property_kind(property) {
        Some(LuksKeyslotPropertyKind::KeyFile) => (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS keyslot property {property} updates encrypted-container key material"
                ),
                alternatives: vec![
                    "verify at least one independent recovery key before changing key material"
                        .to_string(),
                    "add and test replacement access before removing the old keyslot".to_string(),
                    "back up the LUKS header before keyslot access changes".to_string(),
                ],
            }),
        ),
        Some(LuksKeyslotPropertyKind::Priority) => {
            let normalized = normalize_storage_property_name(&property_value(value));
            if matches!(normalized.as_str(), "prefer" | "normal" | "ignore") {
                (
                    RiskClass::OfflineRequired,
                    Some(Advice {
                        summary: format!(
                            "LUKS keyslot property {property} updates keyslot priority metadata"
                        ),
                        alternatives: vec![
                            "back up the LUKS header before changing keyslot metadata".to_string(),
                            "verify another keyslot or recovery passphrase unlocks the device first"
                                .to_string(),
                            "use prefer, normal, or ignore for cryptsetup keyslot priority"
                                .to_string(),
                        ],
                    }),
                )
            } else {
                (
                    RiskClass::Unsupported,
                    Some(Advice {
                        summary: format!(
                            "LUKS keyslot priority value {} is not supported",
                            property_value(value)
                        ),
                        alternatives: vec![
                            "use prefer, normal, or ignore for LUKS keyslot priority".to_string(),
                            "inspect cryptsetup luksDump output before changing keyslot metadata"
                                .to_string(),
                        ],
                    }),
                )
            }
        }
        None => (
            RiskClass::Unsupported,
            Some(Advice {
                summary: format!("LUKS keyslot property {property} is not mapped to a safe command"),
                alternatives: vec![
                    "use keyFile for reviewed key material rotation".to_string(),
                    "use priority with prefer, normal, or ignore for keyslot priority metadata"
                        .to_string(),
                    "apply unsupported LUKS keyslot metadata changes manually after reviewing cryptsetup documentation"
                        .to_string(),
                ],
            }),
        ),
    }
}

fn classify_vdo_property_change(property: &str, value: &Value) -> (RiskClass, Option<Advice>) {
    let property_name = normalize_storage_property_name(property);
    let normalized_value = normalize_storage_property_name(&property_value(value));
    let safe_advice = || {
        Some(Advice {
            summary: format!("VDO property {property} updates an existing VDO volume in place"),
            alternatives: vec![
                "verify vdo status and vdostats before and after the property update".to_string(),
                "prefer changing the existing VDO volume over recreating it when preserving data"
                    .to_string(),
                "review dependent filesystems and mappings before changing write policy"
                    .to_string(),
            ],
        })
    };
    let unsupported_advice = |summary: String, alternatives: Vec<String>| {
        (
            RiskClass::Unsupported,
            Some(Advice {
                summary,
                alternatives,
            }),
        )
    };

    match property_name.as_str() {
        "writepolicy" | "write-policy" | "vdo-write-policy" => {
            if matches!(normalized_value.as_str(), "auto" | "sync" | "async") {
                (RiskClass::Safe, safe_advice())
            } else {
                unsupported_advice(
                    format!(
                        "VDO write policy value {} is not supported",
                        property_value(value)
                    ),
                    vec![
                        "use auto, sync, or async for VDO writePolicy updates".to_string(),
                        "inspect the backing storage cache behavior before choosing sync or async"
                            .to_string(),
                    ],
                )
            }
        }
        "compression" | "vdo-compression" => {
            if is_vdo_boolean_property_value(&normalized_value) {
                (RiskClass::Safe, safe_advice())
            } else {
                unsupported_advice(
                    format!(
                        "VDO compression value {} is not mapped to enable or disable",
                        property_value(value)
                    ),
                    vec![
                        "use enabled/disabled, true/false, yes/no, or on/off for compression"
                            .to_string(),
                        "leave compression unchanged until the intended boolean state is explicit"
                            .to_string(),
                    ],
                )
            }
        }
        "deduplication" | "dedupe" | "vdo-deduplication" | "vdo-dedupe" => {
            if is_vdo_boolean_property_value(&normalized_value) {
                (RiskClass::Safe, safe_advice())
            } else {
                unsupported_advice(
                    format!(
                        "VDO deduplication value {} is not mapped to enable or disable",
                        property_value(value)
                    ),
                    vec![
                        "use enabled/disabled, true/false, yes/no, or on/off for deduplication"
                            .to_string(),
                        "inspect VDO space savings before changing deduplication state".to_string(),
                    ],
                )
            }
        }
        _ => unsupported_advice(
            format!("VDO property {property} is not mapped to a safe command"),
            vec![
                "use writePolicy, compression, or deduplication for supported VDO updates"
                    .to_string(),
                "apply unsupported VDO property changes manually after reviewing VDO tooling"
                    .to_string(),
            ],
        ),
    }
}

fn is_vdo_boolean_property_value(value: &str) -> bool {
    matches!(
        value,
        "enabled"
            | "enable"
            | "true"
            | "yes"
            | "on"
            | "disabled"
            | "disable"
            | "false"
            | "no"
            | "off"
    )
}

fn normalize_storage_property_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("vdo.")
        .trim_start_matches("zfs.")
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn is_btrfs_subvolume_property_supported(property: &str) -> bool {
    matches!(
        property,
        "ro" | "readonly" | "readOnly" | "btrfs.readonly" | "btrfs.ro"
    )
}

fn property_value(value: &Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string())
}
