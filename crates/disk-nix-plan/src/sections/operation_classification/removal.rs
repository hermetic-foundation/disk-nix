fn classify_removal_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Destroy if collection == "loopDevices" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "detaching a loop device requires consumers to be unmounted or stopped"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before detach".to_string(),
                    "keep the backing file intact and recreate the loop mapping after validation"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Unexport if collection == "exports" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "unexporting NFS paths can interrupt active remote clients".to_string(),
                alternatives: vec![
                    "remove or migrate clients before unexporting the path".to_string(),
                    "switch export options to read-only before final removal".to_string(),
                    "verify no active mounts depend on the export before reload".to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Unmount if collection == "nfs.mounts" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "unmounting an NFS client path can interrupt local services".to_string(),
                alternatives: vec![
                    "stop local services and automount units before unmounting".to_string(),
                    "switch the mount to read-only or noauto before final removal".to_string(),
                    "verify no open files or bind mounts still depend on the mountpoint"
                        .to_string(),
                ],
            }),
        ),
        Operation::Unmount if collection == "filesystems" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "unmounting a filesystem can interrupt local services without deleting data"
                    .to_string(),
                alternatives: vec![
                    "stop dependent services, automount units, user sessions, and bind mounts before unmounting"
                        .to_string(),
                    "switch the mount to read-only or noauto first when a staged removal is safer"
                        .to_string(),
                    "verify no open files still reference the mountpoint before applying"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Logout if collection == "iscsiSessions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "iSCSI logout detaches remote LUN paths from the host".to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before logout".to_string(),
                    "verify multipath, LVM, and filesystem consumers have migrated away"
                        .to_string(),
                    "disable automatic login only after dependent services no longer need the LUN"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Detach if collection == "targetLuns" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary:
                    "target-side LUN unmapping or removal can make remote storage unavailable"
                        .to_string(),
                alternatives: vec![
                    "unmap from initiators before deleting target-side storage".to_string(),
                    "detach host paths and verify no multipath, LVM, filesystem, or guest consumers remain"
                        .to_string(),
                    "preserve or snapshot the backing object until post-removal verification passes"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::Detach if collection == "luns" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "LUN host detach removes reviewed SCSI paths from this host".to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate LVM, multipath, or dm consumers before detach"
                        .to_string(),
                    "remove a single path only after redundancy or alternate paths are healthy"
                        .to_string(),
                    "disable automatic session login only after dependent services no longer need the LUN"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "lvmCaches" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
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
        ),
        Operation::Destroy | Operation::RemoveKey if collection == "luksKeyslots" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: "removing a LUKS keyslot can lock out encrypted data if no other key works"
                    .to_string(),
                alternatives: vec![
                    "verify another passphrase, key file, or token unlocks the device first".to_string(),
                    "take a LUKS header backup before keyslot removal".to_string(),
                    "add and test a replacement keyslot before killing the old slot".to_string(),
                ],
            }),
        ),
        Operation::Destroy | Operation::RemoveToken if collection == "luksTokens" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: "removing a LUKS token can lock out automated unlock".to_string(),
                alternatives: vec![
                    "verify another token, keyslot, or passphrase unlocks the device first"
                        .to_string(),
                    "take a LUKS header backup before token removal".to_string(),
                    "import and test a replacement token before removing the old token".to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "physicalVolumes" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "LVM physical volume removal erases PV metadata from the device"
                    .to_string(),
                alternatives: vec![
                    "pvmove allocated extents and vgreduce the PV before pvremove".to_string(),
                    "verify no volume group still depends on the PV".to_string(),
                    "preserve the device for recovery until backups are verified".to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "nvmeNamespaces" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "NVMe namespace deletion removes the namespace from the controller"
                    .to_string(),
                alternatives: vec![
                    "detach the namespace from selected controllers before deletion".to_string(),
                    "migrate or snapshot data before deleting the namespace".to_string(),
                    "use host detach or rescan workflows when target-side data should remain"
                        .to_string(),
                ],
            }),
        ),
        Operation::Detach if collection == "nvmeNamespaces" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "NVMe namespace detach removes host/controller access without deleting the namespace"
                    .to_string(),
                alternatives: vec![
                    "detach from selected controllers before deleting only when data removal is intended"
                        .to_string(),
                    "unmount filesystems and deactivate LVM, dm, or multipath consumers before detach"
                        .to_string(),
                    "use rescan when namespace visibility changed outside disk-nix".to_string(),
                ],
            }),
        ),
        Operation::Rollback if collection == "lvmSnapshots" => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: "merging an LVM snapshot rolls the origin back to older contents"
                    .to_string(),
                alternatives: vec![
                    "take a fresh snapshot of the current origin before merge".to_string(),
                    "mount or clone the snapshot for inspection before rollback".to_string(),
                    "schedule downtime when the origin must be deactivated for merge".to_string(),
                ],
            }),
        ),
        _ => return None,
    })
}
