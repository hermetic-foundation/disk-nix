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

fn add_destroy_guard(
    actions: &mut Vec<PlannedAction>,
    collection: &str,
    name: &str,
    object: &Value,
) {
    let destroy = object
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let preserve_data = object
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if destroy || !preserve_data {
        if collection == "exports" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("unexport NFS path {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "unexporting NFS paths can interrupt active remote clients"
                        .to_string(),
                    alternatives: vec![
                        "remove or migrate clients before unexporting the path".to_string(),
                        "switch export options to read-only before final removal".to_string(),
                        "verify no active mounts depend on the export before reload".to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "iscsiSessions" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("log out iSCSI session {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "iSCSI logout detaches remote LUN paths from the host".to_string(),
                    alternatives: vec![
                        "unmount filesystems and deactivate mappings before logout".to_string(),
                        "verify multipath, LVM, and filesystem consumers have migrated away"
                            .to_string(),
                        "disable automatic login only after dependent services no longer need the LUN"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "nfs.mounts" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("unmount NFS client mount {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "unmounting an NFS client path can interrupt local services"
                        .to_string(),
                    alternatives: vec![
                        "stop local services and automount units before unmounting".to_string(),
                        "switch the mount to read-only or noauto before final removal".to_string(),
                        "verify no open files or bind mounts still depend on the mountpoint"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "luns" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("detach LUN paths for {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "LUN host detach removes reviewed SCSI paths from this host"
                        .to_string(),
                    alternatives: vec![
                        "unmount filesystems and deactivate LVM, multipath, or dm consumers before detach"
                            .to_string(),
                        "remove a single path only after redundancy or alternate paths are healthy"
                            .to_string(),
                        "disable automatic session login only after dependent services no longer need the LUN"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "physicalVolumes" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("remove LVM physical volume metadata from {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::Destructive,
                destructive: true,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "LVM physical volume removal erases PV metadata from the device"
                        .to_string(),
                    alternatives: vec![
                        "pvmove allocated extents and vgreduce the PV before pvremove".to_string(),
                        "verify no volume group still depends on the PV".to_string(),
                        "preserve the device for recovery until backups are verified".to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "lvmCaches" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("detach LVM cache from {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::OfflineRequired,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary: "LVM cache removal must flush dirty cache state before uncaching"
                        .to_string(),
                    alternatives: vec![
                        "switch to writethrough and wait for dirty blocks to drain before lvconvert --uncache"
                            .to_string(),
                        "verify the origin LV is readable without the cache before removing cache media"
                            .to_string(),
                        "keep the cache pool intact until post-uncache verification passes".to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "luksKeyslots" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("remove LUKS keyslot {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::PotentialDataLoss,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary:
                        "removing a LUKS keyslot can lock out encrypted data if no other key works"
                            .to_string(),
                    alternatives: vec![
                        "verify another passphrase, key file, or token unlocks the device first"
                            .to_string(),
                        "take a LUKS header backup before keyslot removal".to_string(),
                        "add and test a replacement keyslot before killing the old slot"
                            .to_string(),
                    ],
                }),
            });
            return;
        }
        if collection == "luksTokens" {
            actions.push(PlannedAction {
                id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
                description: format!("remove LUKS token {name}"),
                operation: Operation::Destroy,
                risk: RiskClass::PotentialDataLoss,
                destructive: false,
                context: lifecycle_context(collection, name, object),
                advice: Some(Advice {
                    summary:
                        "removing a LUKS token can lock out automated unlock if no other path works"
                            .to_string(),
                    alternatives: vec![
                        "verify a passphrase, recovery key, or replacement token unlocks the device first"
                            .to_string(),
                        "take a LUKS header backup before token removal".to_string(),
                        "import and test a replacement token before removing the old token".to_string(),
                    ],
                }),
            });
            return;
        }

        let mut alternatives = destructive_alternatives(collection, object);
        alternatives.push("rename, detach, or unmount first when supported".to_string());
        actions.push(PlannedAction {
            id: format!("{collection}:{name}:destroy").to_ascii_lowercase(),
            description: format!("{collection} {name} may be destroyed or replaced"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: lifecycle_context(collection, name, object),
            advice: Some(Advice {
                summary: "destroying or replacing storage removes live data".to_string(),
                alternatives,
            }),
        });
    }
}

fn add_snapshot_actions(actions: &mut Vec<PlannedAction>, name: &str, snapshot: &Value) {
    let target = snapshot
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let snapshot_name = string_field(snapshot, &["name", "snapshotName", "snapshot-name"])
        .unwrap_or_else(|| name.to_string());
    let snapshot_path = string_field(snapshot, &["path", "snapshotPath", "snapshot-path"]);
    let hold = string_field(snapshot, &["hold", "holdTag"]);
    let release_hold = string_field(snapshot, &["releaseHold", "release-hold"]);
    let clone_to = string_field(snapshot, &["cloneTo", "cloneTarget", "clone"]);
    let rename_to = string_field(snapshot, &["renameTo", "renameTarget", "newName"]);
    let destroy = snapshot
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rollback = snapshot
        .get("rollback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recursive_rollback = snapshot
        .get("recursiveRollback")
        .or_else(|| snapshot.get("recursive"))
        .or_else(|| snapshot.get("zfs.rollbackRecursive"))
        .and_then(Value::as_bool);
    let read_only = snapshot
        .get("readOnly")
        .or_else(|| snapshot.get("readonly"))
        .and_then(Value::as_bool);
    let requested_operation = snapshot
        .get("operation")
        .or_else(|| snapshot.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);

    if requested_operation == Some(Operation::Rescan) {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rescan"),
            description: format!("rescan snapshot metadata for {name}"),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                snapshot_path: snapshot_path.clone(),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot rescan refreshes recovery-point metadata without mutating data"
                    .to_string(),
                alternatives: vec![
                    "use holds for retention changes instead of recreating snapshots".to_string(),
                    "clone a snapshot for inspection before rollback or destruction".to_string(),
                    "verify source dataset or subvolume relationships after metadata refresh"
                        .to_string(),
                ],
            }),
        });
    }

    if let Some(hold) = hold {
        actions.push(snapshot_hold_action(
            name,
            &snapshot_name,
            target,
            &hold,
            read_only,
            false,
        ));
    }
    if let Some(release_hold) = release_hold {
        actions.push(snapshot_hold_action(
            name,
            &snapshot_name,
            target,
            &release_hold,
            read_only,
            true,
        ));
    }
    if let Some(clone_to) = clone_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:clone:{clone_to}"),
            description: format!("clone snapshot {snapshot_name} to {clone_to}"),
            operation: Operation::Clone,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(clone_to),
                snapshot_path: snapshot_path.clone(),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot clone creates a writable ZFS dataset or Btrfs subvolume copy"
                    .to_string(),
                alternatives: vec![
                    "inspect the clone before rollback or destructive changes".to_string(),
                    "destroy the clone after migration or validation if it is no longer needed"
                        .to_string(),
                ],
            }),
        });
    }
    if let Some(rename_to) = rename_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rename:{rename_to}"),
            description: format!("rename snapshot {snapshot_name} to {rename_to}"),
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                rename_to: Some(rename_to),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary:
                    "snapshot rename preserves the recovery point while changing its reference"
                        .to_string(),
                alternatives: vec![
                    "hold the snapshot before renaming when retention jobs may race".to_string(),
                    "update replication, rollback, and cleanup references after rename".to_string(),
                ],
            }),
        });
    }

    if destroy {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:destroy"),
            description: format!("destroy snapshot {snapshot_name} for {target}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot destruction removes a recovery point".to_string(),
                alternatives: vec![
                    "keep the snapshot until replacement backups are verified".to_string(),
                    "rename or hold the snapshot before pruning".to_string(),
                ],
            }),
        });
    } else if rollback {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rollback"),
            description: format!("roll back {target} to snapshot {snapshot_name}"),
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                read_only,
                recursive_rollback,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "rollback can discard changes newer than the snapshot".to_string(),
                alternatives: vec![
                    "clone the snapshot and inspect data before rollback".to_string(),
                    "take a pre-rollback snapshot of the current state".to_string(),
                ],
            }),
        });
    } else if actions
        .iter()
        .all(|action| !action.id.starts_with(&format!("snapshot:{name}:")))
    {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:create"),
            description: format!("create snapshot {snapshot_name} for {target}"),
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: None,
        });
    }
}

fn snapshot_hold_action(
    action_name: &str,
    snapshot_name: &str,
    target: &str,
    tag: &str,
    read_only: Option<bool>,
    release: bool,
) -> PlannedAction {
    let (verb, property) = if release {
        ("release hold on", "zfs.releaseHold")
    } else {
        ("hold", "zfs.hold")
    };
    PlannedAction {
        id: format!(
            "snapshot:{action_name}:{}:{tag}",
            if release { "release-hold" } else { "hold" }
        ),
        description: format!("{verb} snapshot {snapshot_name} for {target} with tag {tag}"),
        operation: Operation::SetProperty,
        risk: RiskClass::Safe,
        destructive: false,
        context: ActionContext {
            collection: Some("snapshots".to_string()),
            name: Some(snapshot_name.to_string()),
            target: Some(target.to_string()),
            property: Some(property.to_string()),
            property_value: Some(tag.to_string()),
            read_only,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: if release {
                "releasing a snapshot hold allows later pruning by the same tag".to_string()
            } else {
                "holding a snapshot prevents accidental ZFS snapshot destruction by tag".to_string()
            },
            alternatives: if release {
                vec![
                    "keep the hold until replacement backups or replication are verified"
                        .to_string(),
                    "list active holds before releasing retention protection".to_string(),
                ]
            } else {
                vec![
                    "use a stable tag name that identifies the retention policy".to_string(),
                    "replicate or back up the snapshot before removing retention holds".to_string(),
                ]
            },
        }),
    }
}

fn parse_operation(value: &str) -> Option<Operation> {
    match value {
        "create" => Some(Operation::Create),
        "format" => Some(Operation::Format),
        "grow" => Some(Operation::Grow),
        "shrink" => Some(Operation::Shrink),
        "check" => Some(Operation::Check),
        "repair" => Some(Operation::Repair),
        "scrub" => Some(Operation::Scrub),
        "trim" => Some(Operation::Trim),
        "rescan" | "re-scan" => Some(Operation::Rescan),
        "replace-device" | "replaceDevice" => Some(Operation::ReplaceDevice),
        "add-device" | "addDevice" => Some(Operation::AddDevice),
        "remove-device" | "removeDevice" => Some(Operation::RemoveDevice),
        "add-key" | "addKey" | "add-keyslot" | "addKeyslot" => Some(Operation::AddKey),
        "remove-key" | "removeKey" | "remove-keyslot" | "removeKeyslot" | "kill-slot"
        | "killSlot" => Some(Operation::RemoveKey),
        "import-token" | "importToken" => Some(Operation::ImportToken),
        "remove-token" | "removeToken" => Some(Operation::RemoveToken),
        "set-property" | "setProperty" => Some(Operation::SetProperty),
        "snapshot" => Some(Operation::Snapshot),
        "clone" => Some(Operation::Clone),
        "promote" => Some(Operation::Promote),
        "import" => Some(Operation::Import),
        "export" => Some(Operation::Export),
        "unexport" | "un-export" => Some(Operation::Unexport),
        "attach" => Some(Operation::Attach),
        "detach" => Some(Operation::Detach),
        "activate" => Some(Operation::Activate),
        "deactivate" => Some(Operation::Deactivate),
        "assemble" => Some(Operation::Assemble),
        "start" => Some(Operation::Start),
        "stop" => Some(Operation::Stop),
        "login" | "log-in" | "logIn" => Some(Operation::Login),
        "logout" | "log-out" | "logOut" => Some(Operation::Logout),
        "open" => Some(Operation::Open),
        "close" => Some(Operation::Close),
        "mount" => Some(Operation::Mount),
        "unmount" | "un-mount" | "umount" => Some(Operation::Unmount),
        "remount" => Some(Operation::Remount),
        "rename" => Some(Operation::Rename),
        "rebalance" => Some(Operation::Rebalance),
        "rollback" => Some(Operation::Rollback),
        "destroy" => Some(Operation::Destroy),
        _ => None,
    }
}

fn operation_id(operation: Operation) -> &'static str {
    match operation {
        Operation::Create => "create",
        Operation::Format => "format",
        Operation::Grow => "grow",
        Operation::Shrink => "shrink",
        Operation::Check => "check",
        Operation::Repair => "repair",
        Operation::Scrub => "scrub",
        Operation::Trim => "trim",
        Operation::Rescan => "rescan",
        Operation::ReplaceDevice => "replace-device",
        Operation::AddDevice => "add-device",
        Operation::RemoveDevice => "remove-device",
        Operation::AddKey => "add-key",
        Operation::RemoveKey => "remove-key",
        Operation::ImportToken => "import-token",
        Operation::RemoveToken => "remove-token",
        Operation::SetProperty => "set-property",
        Operation::Snapshot => "snapshot",
        Operation::Clone => "clone",
        Operation::Promote => "promote",
        Operation::Import => "import",
        Operation::Export => "export",
        Operation::Unexport => "unexport",
        Operation::Attach => "attach",
        Operation::Detach => "detach",
        Operation::Activate => "activate",
        Operation::Deactivate => "deactivate",
        Operation::Assemble => "assemble",
        Operation::Start => "start",
        Operation::Stop => "stop",
        Operation::Login => "login",
        Operation::Logout => "logout",
        Operation::Open => "open",
        Operation::Close => "close",
        Operation::Mount => "mount",
        Operation::Unmount => "unmount",
        Operation::Remount => "remount",
        Operation::Rename => "rename",
        Operation::Rebalance => "rebalance",
        Operation::Rollback => "rollback",
        Operation::Destroy => "destroy",
    }
}
