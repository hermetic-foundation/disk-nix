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
