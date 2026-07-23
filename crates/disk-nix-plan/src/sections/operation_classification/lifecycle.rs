fn classify_lifecycle_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Create | Operation::SetProperty => (RiskClass::Safe, false, None),
        Operation::Clone => (
            RiskClass::Reversible,
            false,
            Some(Advice {
                summary: format!("{collection} clone creates a dependent writable copy"),
                alternatives: vec![
                    "inspect the clone before using it for rollback or migration".to_string(),
                    "destroy temporary clones after validation".to_string(),
                ],
            }),
        ),
        Operation::Promote => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: format!("{collection} promote makes a clone independent of its origin"),
                alternatives: vec![
                    "inspect origin and dependent snapshots before promoting".to_string(),
                    "validate mounts, shares, LUN mappings, and services against the promoted clone"
                        .to_string(),
                    "keep the original dataset until the promoted clone is verified".to_string(),
                ],
            }),
        ),
        Operation::Import if collection == "pools" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "ZFS pool import makes an existing pool active on this host".to_string(),
                alternatives: vec![
                    "import read-only first when validating a moved or recovered pool".to_string(),
                    "verify hostid, cachefile, mountpoints, and encryption keys before import"
                        .to_string(),
                    "prefer import over recreating a pool when preserving data".to_string(),
                ],
            }),
        ),
        Operation::Export if collection == "pools" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "ZFS pool export cleanly detaches a pool without deleting data".to_string(),
                alternatives: vec![
                    "export a pool instead of destroying it when moving hosts".to_string(),
                    "stop mounts, shares, LUN exports, and services before export".to_string(),
                    "verify all writes are complete and pool health is reviewed first".to_string(),
                ],
            }),
        ),
        Operation::Import if collection == "volumeGroups" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LVM volume group import reactivates an exported VG on this host"
                    .to_string(),
                alternatives: vec![
                    "inspect PV identities and VG UUIDs before vgimport".to_string(),
                    "prefer vgimport over vgcreate when preserving existing logical volumes"
                        .to_string(),
                    "activate and mount consumers only after the imported VG is verified"
                        .to_string(),
                ],
            }),
        ),
        Operation::Export if collection == "volumeGroups" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LVM volume group export marks a VG inactive for movement without deleting data"
                    .to_string(),
                alternatives: vec![
                    "export a VG instead of removing it when moving disks between hosts"
                        .to_string(),
                    "deactivate logical volumes and stop mounts or services before vgexport"
                        .to_string(),
                    "verify metadata backups before changing VG activation state".to_string(),
                ],
            }),
        ),
        Operation::Activate
            if collection == "volumes"
                || collection == "thinPools"
                || collection == "lvmSnapshots"
                || collection == "volumeGroups" =>
        {
            (
                RiskClass::OfflineRequired,
                false,
                Some(Advice {
                    summary: format!(
                        "{collection} activation makes an existing LVM object available without creating it"
                    ),
                    alternatives: vec![
                        "inspect LVM metadata and dependent mappings before activation"
                            .to_string(),
                        "activate only the reviewed VG or LV needed for consumers".to_string(),
                        "verify filesystems, mounts, and services after activation".to_string(),
                    ],
                }),
            )
        }
        Operation::Deactivate
            if collection == "volumes"
                || collection == "thinPools"
                || collection == "lvmSnapshots"
                || collection == "volumeGroups" =>
        {
            (
                RiskClass::OfflineRequired,
                false,
                Some(Advice {
                    summary: format!(
                        "{collection} deactivation makes an existing LVM object unavailable without deleting it"
                    ),
                    alternatives: vec![
                        "unmount filesystems and stop services before deactivation".to_string(),
                        "deactivate instead of removing storage when preserving data".to_string(),
                        "verify no dm, filesystem, LUN, or service consumers remain active"
                            .to_string(),
                    ],
                }),
            )
        }
        Operation::Rescan if collection == "lvmSnapshots" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LVM snapshot rescan refreshes origin, COW usage, and graph status"
                    .to_string(),
                alternatives: vec![
                    "merge only after inspecting the snapshot contents and origin state"
                        .to_string(),
                    "activate the snapshot for recovery inspection instead of removing it"
                        .to_string(),
                    "verify snapshot fullness before relying on it as a recovery point"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rename => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: format!("{collection} rename retargets a storage object without deleting it"),
                alternatives: vec![
                    "rename first and validate consumers before destroying old paths".to_string(),
                    "update mounts, exports, LUN mappings, and services before applying".to_string(),
                    "keep snapshots or backups until consumers use the renamed object".to_string(),
                ],
            }),
        ),
        _ => return None,
    })
}
