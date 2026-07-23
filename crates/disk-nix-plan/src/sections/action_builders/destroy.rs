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
