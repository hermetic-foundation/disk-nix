type OperationClassification = (RiskClass, bool, Option<Advice>);

fn classify_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> OperationClassification {
    if let Some(classification) = classify_local_filesystem_operation(collection, operation, object) {
        return classification;
    }
    if let Some(classification) = classify_local_device_operation(collection, operation, object) {
        return classification;
    }
    if let Some(classification) = classify_network_storage_operation(collection, operation, object) {
        return classification;
    }
    if let Some(classification) = classify_lifecycle_operation(collection, operation, object) {
        return classification;
    }
    if let Some(classification) = classify_growth_operation(collection, operation, object) {
        return classification;
    }
    if let Some(classification) = classify_removal_operation(collection, operation, object) {
        return classification;
    }
    classify_unsupported_operation(collection, operation, object)
        .expect("operation classifier fallback is exhaustive")
}

include!("operation_classification/local_filesystems.rs");
include!("operation_classification/local_devices.rs");
include!("operation_classification/network_storage.rs");
include!("operation_classification/lifecycle.rs");
include!("operation_classification/growth.rs");
include!("operation_classification/removal.rs");
include!("operation_classification/unsupported.rs");

fn classify_replace_device(collection: &str) -> (RiskClass, Advice) {
    if collection == "caches" || collection == "lvmCaches" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "cache replacement must account for dirty or writeback data".to_string(),
                alternatives: vec![
                    "flush dirty data before replacing the cache device".to_string(),
                    "detach or disable writeback caching before removing the source".to_string(),
                    "verify the origin or backing volume before re-enabling the cache".to_string(),
                ],
            },
        )
    } else if collection == "mdRaids" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary:
                    "MD RAID replacement must preserve redundancy through fail, add, and resync"
                        .to_string(),
                alternatives: vec![
                    "add a spare and wait for sync before failing the old member".to_string(),
                    "replace one member at a time while the array is healthy".to_string(),
                    "verify /proc/mdstat and mdadm --detail before removing the old device"
                        .to_string(),
                ],
            },
        )
    } else if collection == "multipathMaps" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "multipath path replacement must preserve live path redundancy"
                    .to_string(),
                alternatives: vec![
                    "add and verify the replacement path before deleting the old path".to_string(),
                    "fail or disable one path at a time while other paths remain active"
                        .to_string(),
                    "reload maps only after every expected path is visible".to_string(),
                ],
            },
        )
    } else if collection == "volumeGroups" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "LVM physical volume replacement must migrate extents before vgreduce"
                    .to_string(),
                alternatives: vec![
                    "vgextend the replacement PV before running pvmove".to_string(),
                    "keep the old PV available until pvmove completes and LVs are verified"
                        .to_string(),
                    "use pvs and vgs reports to confirm no allocated extents remain before vgreduce"
                        .to_string(),
                ],
            },
        )
    } else {
        (
            RiskClass::Reversible,
            Advice {
                summary: "replacement should preserve data when the source remains available"
                    .to_string(),
                alternatives: vec![
                    "attach the replacement and resilver or rebalance before detaching the source"
                        .to_string(),
                    "keep the original device untouched until post-apply verification passes"
                        .to_string(),
                ],
            },
        )
    }
}

fn classify_add_device(collection: &str) -> (RiskClass, Option<Advice>) {
    if collection == "lvmCaches" {
        (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "LVM cache attachment changes origin LV I/O through cache media"
                    .to_string(),
                alternatives: vec![
                    "verify the cache pool LV belongs to the same volume group as the origin"
                        .to_string(),
                    "start in writethrough mode when rollback safety matters".to_string(),
                    "keep the origin LV snapshot or backup until cache verification passes"
                        .to_string(),
                ],
            }),
        )
    } else {
        (RiskClass::Online, None)
    }
}

fn classify_remove_device(collection: &str) -> (RiskClass, Advice) {
    if collection == "caches" || collection == "lvmCaches" {
        (
            RiskClass::OfflineRequired,
            Advice {
                summary: "cache detach must flush dirty data before removing cache media"
                    .to_string(),
                alternatives: vec![
                    "switch writeback caches to writethrough before detach".to_string(),
                    "wait for dirty data to drain before removing the cache device".to_string(),
                    "keep backing storage online and verify it remains readable after detach"
                        .to_string(),
                ],
            },
        )
    } else {
        (
            RiskClass::PotentialDataLoss,
            Advice {
                summary: "device removal requires enough remaining data and metadata capacity"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removing the old device".to_string(),
                    "rebalance or evacuate data before removal".to_string(),
                    "verify redundancy and current health before applying".to_string(),
                ],
            },
        )
    }
}

fn destructive_alternatives(collection: &str, object: &Value) -> Vec<String> {
    let mut alternatives = vec![
        "take and verify a backup before destructive changes".to_string(),
        "migrate data to replacement storage first".to_string(),
    ];

    match collection {
        "pools" | "datasets" | "zvols" => {
            alternatives.push("take a recursive snapshot before destroy or rollback".to_string());
            alternatives
                .push("rename or unmount the dataset while validating consumers".to_string());
        }
        "btrfsSubvolumes" => {
            alternatives
                .push("take a read-only snapshot before deleting the subvolume".to_string());
            alternatives
                .push("rename the subvolume and validate consumers before removal".to_string());
        }
        "volumes" | "volumeGroups" | "thinPools" | "luns" | "targetLuns" | "mdRaids"
        | "multipathMaps" => {
            alternatives
                .push("grow or attach replacement capacity instead of reformatting".to_string());
        }
        "loopDevices" => {
            alternatives
                .push("detach the loop device without deleting its backing file".to_string());
            alternatives.push("unmount consumers before changing the backing file".to_string());
        }
        "lvmSnapshots" => {
            alternatives.push("merge or mount the snapshot before deleting it".to_string());
            alternatives.push(
                "create a replacement snapshot before pruning old recovery points".to_string(),
            );
        }
        "vdoVolumes" => {
            alternatives
                .push("grow the VDO logical or physical size instead of recreating it".to_string());
            alternatives
                .push("migrate data to a replacement VDO volume before removal".to_string());
        }
        "disks" | "partitions" => {
            alternatives.push(
                "preserve the existing partition table and add capacity elsewhere".to_string(),
            );
            alternatives.push("clone the disk before changing partition metadata".to_string());
        }
        "exports" => {
            alternatives
                .push("disable clients or switch exports before removing the source".to_string());
        }
        _ => {}
    }

    if object
        .get("preserveData")
        .and_then(Value::as_bool)
        .is_some_and(|preserve| !preserve)
    {
        alternatives.push("set preserveData=true for non-destructive planning".to_string());
    }

    alternatives
}

fn operation_label(operation: Operation) -> &'static str {
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
        Operation::ReplaceDevice => "replace device",
        Operation::AddDevice => "add device",
        Operation::RemoveDevice => "remove device",
        Operation::AddKey => "add key",
        Operation::RemoveKey => "remove key",
        Operation::ImportToken => "import token",
        Operation::RemoveToken => "remove token",
        Operation::SetProperty => "set property",
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
