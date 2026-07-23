fn classify_unsupported_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Import | Operation::Export => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} is currently only supported for ZFS pools, LVM volume groups, and NFS exports",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use pools.<name>.operation for ZFS pool import or export".to_string(),
                    "use volumeGroups.<name>.operation for LVM VG import or export".to_string(),
                    "use exports.<path>.operation = \"export\" for NFS export publication"
                        .to_string(),
                    "use domain-specific attach, detach, mount, or unmount operations where available"
                        .to_string(),
                ],
            }),
        ),
        Operation::Unexport => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: "unexport operations are currently only supported for exports".to_string(),
                alternatives: vec![
                    "use operation = \"unexport\" on exports declarations for NFS server export lifecycle"
                        .to_string(),
                    "use operation = \"unmount\" on nfs.mounts declarations for NFS client mounts"
                        .to_string(),
                    "use destroy only where a storage domain has not yet gained explicit lifecycle verbs"
                        .to_string(),
                ],
            }),
        ),
        Operation::Attach | Operation::Detach => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for LUNs and NVMe namespaces",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"attach\" or \"detach\" on luns declarations for host-side LUN path lifecycle"
                        .to_string(),
                    "use operation = \"attach\" or \"detach\" on nvmeNamespaces declarations for namespace/controller lifecycle"
                        .to_string(),
                    "use operation = \"login\" or \"logout\" on iscsiSessions declarations for target session lifecycle"
                        .to_string(),
                    "use domain-specific add-device, remove-device, mount, unmount, import, or export operations where available"
                        .to_string(),
                ],
            }),
        ),
        Operation::Activate | Operation::Deactivate => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} is currently only supported for LVM volumes, thin pools, snapshots, and volume groups",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use volumes, thinPools, lvmSnapshots, or volumeGroups for LVM activation lifecycle"
                        .to_string(),
                    "use mount, login, attach, or import operations for non-LVM domains where available"
                        .to_string(),
                ],
            }),
        ),
        Operation::Assemble | Operation::Start | Operation::Stop => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are not implemented for {collection}",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"assemble\" only on mdRaids declarations for now".to_string(),
                    "use operation = \"start\" or \"stop\" on vdoVolumes declarations for VDO activation lifecycle"
                        .to_string(),
                    "use subsystem-specific import, export, activate, or deactivate operations where supported"
                        .to_string(),
                ],
            }),
        ),
        Operation::Login | Operation::Logout => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for iscsiSessions",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"login\" or \"logout\" on iscsiSessions declarations for iSCSI session lifecycle"
                        .to_string(),
                    "use create/destroy only where a storage domain has not yet gained explicit lifecycle verbs"
                        .to_string(),
                ],
            }),
        ),
        Operation::Mount | Operation::Unmount => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for filesystems and nfs.mounts",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"mount\" or \"unmount\" on filesystems declarations for local filesystem mount lifecycle"
                        .to_string(),
                    "use operation = \"mount\" or \"unmount\" on nfs.mounts declarations for NFS client mount lifecycle"
                        .to_string(),
                    "use service or automount-specific workflows for domains outside the modeled mount collections"
                        .to_string(),
                ],
            }),
        ),
        Operation::Open | Operation::Close => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luks.devices",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use luks.devices.<name>.operation for encrypted mapper open or close"
                        .to_string(),
                    "use activate, deactivate, import, export, mount, or remount for other storage domains"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: "rescan operations are currently supported for filesystems, disks, partitions, snapshots, LUNs, iSCSI sessions, NFS exports/mounts, NVMe namespaces, multipath maps, loop devices, backing files, ZFS datasets/zvols, Btrfs subvolumes/qgroups, LVM PV/VG/LV/snapshot/cache/thin-pool metadata, MD RAID metadata, VDO status, and bcache status"
                    .to_string(),
                alternatives: vec![
                    "use filesystems.<name>.operation = \"rescan\" to refresh local mount and graph inventory"
                        .to_string(),
                    "use disks.<path>.operation = \"rescan\" to reread a partition table"
                        .to_string(),
                    "use partitions.<name>.operation = \"rescan\" to refresh a reviewed backing disk"
                        .to_string(),
                    "use luns.<name>.operation = \"rescan\" to refresh reviewed SCSI paths"
                        .to_string(),
                    "use iscsiSessions.<target>.operation = \"rescan\" to refresh existing target sessions"
                        .to_string(),
                    "use exports.<path>.operation = \"rescan\" to refresh NFS export inventory"
                        .to_string(),
                    "use nfs.mounts.<mountpoint>.operation = \"rescan\" to refresh NFS client mount state"
                        .to_string(),
                    "use nvmeNamespaces.<controller>.operation = \"rescan\" to refresh namespace inventory"
                        .to_string(),
                    "use multipathMaps.<name>.operation = \"rescan\" to reload reviewed path maps"
                        .to_string(),
                    "use loopDevices.<path>.operation = \"rescan\" to refresh loop mapping inventory"
                        .to_string(),
                    "use backingFiles.<path>.operation = \"rescan\" to refresh file-backed storage origin inventory"
                        .to_string(),
                    "use dmMaps.<name>.operation = \"rescan\" to refresh device-mapper table and status metadata"
                        .to_string(),
                    "use physicalVolumes or volumeGroups operation = \"rescan\" to refresh LVM metadata"
                        .to_string(),
                    "use volumes.<vg/lv>.operation = \"rescan\" to refresh LVM logical volume status"
                        .to_string(),
                    "use lvmCaches.<origin>.operation = \"rescan\" to refresh LVM cache status and utilization"
                        .to_string(),
                    "use thinPools.<pool>.operation = \"rescan\" to refresh LVM thin-pool utilization"
                        .to_string(),
                    "use lvmSnapshots.<vg/lv>.operation = \"rescan\" to refresh LVM snapshot status"
                        .to_string(),
                    "use snapshots.<name>.operation = \"rescan\" to refresh snapshot metadata and holds"
                        .to_string(),
                    "use btrfsSubvolumes.<path>.operation = \"rescan\" to refresh subvolume metadata and read-only state"
                        .to_string(),
                    "use datasets.<name>.operation = \"rescan\" to refresh ZFS dataset properties and graph state"
                        .to_string(),
                    "use zvols.<name>.operation = \"rescan\" to refresh ZFS volume properties and block graph state"
                        .to_string(),
                    "use mdRaids.<name>.operation = \"rescan\" to refresh MD RAID metadata inventory"
                        .to_string(),
                    "use vdoVolumes.<name>.operation = \"rescan\" to refresh VDO status and utilization"
                        .to_string(),
                    "use caches.<device>.operation = \"rescan\" to refresh bcache state and dirty-data counters"
                        .to_string(),
                    "use btrfsQgroups.<id>.operation = \"rescan\" with target = <mountpoint> to refresh quota hierarchy and usage"
                        .to_string(),
                ],
            }),
        ),
        Operation::AddKey | Operation::RemoveKey => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luksKeyslots",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use luksKeyslots.<name>.operation for LUKS keyslot add or remove lifecycle"
                        .to_string(),
                    "use luks.devices.<name>.operation for encrypted mapper open or close"
                        .to_string(),
                    "use set-property for LUKS label, UUID, or key rotation updates".to_string(),
                ],
            }),
        ),
        Operation::ImportToken | Operation::RemoveToken => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are currently only supported for luksTokens",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use luksTokens.<name>.operation for LUKS token import or remove lifecycle"
                        .to_string(),
                    "verify a fallback keyslot or recovery passphrase before changing tokens"
                        .to_string(),
                    "use luksKeyslots declarations when changing passphrase/key-file access"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount => (
            RiskClass::Unsupported,
            false,
            Some(Advice {
                summary: format!(
                    "{} operations are not implemented for {collection}",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "use operation = \"remount\" on filesystems or nfs.mounts declarations"
                        .to_string(),
                    "use a filesystem-specific mount or service restart workflow for other remount needs"
                        .to_string(),
                ],
            }),
        ),
        Operation::Shrink
        | Operation::Check
        | Operation::Repair
        | Operation::Scrub
        | Operation::Trim
        | Operation::RemoveDevice
        | Operation::Rollback => (
            RiskClass::PotentialDataLoss,
            false,
            Some(Advice {
                summary: format!(
                    "{} can require evacuation, rollback, or offline validation",
                    operation_label(operation)
                ),
                alternatives: vec![
                    "prefer grow, add, replace, or clone operations where possible".to_string(),
                    "verify backups and health before applying".to_string(),
                    "stage the change against a clone or replacement target first".to_string(),
                ],
            }),
        ),
        Operation::Format | Operation::Destroy => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: format!(
                    "{} on {collection} removes or overwrites existing storage",
                    operation_label(operation)
                ),
                alternatives: destructive_alternatives(collection, object),
            }),
        ),
        _ => return None,
    })
}
