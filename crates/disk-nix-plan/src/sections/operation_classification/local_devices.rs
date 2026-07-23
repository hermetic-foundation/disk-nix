fn classify_local_device_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Create if collection == "volumeGroups" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "volume group creation writes LVM metadata to the selected physical volume"
                    .to_string(),
                alternatives: vec![
                    "verify the physical volume contains no data that must be preserved"
                        .to_string(),
                    "extend an existing volume group when preserving consumers is possible"
                        .to_string(),
                    "use stable /dev/disk/by-id paths and inspect pvs before vgcreate".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "vdoVolumes" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "VDO volume creation writes VDO metadata to the selected backing device"
                    .to_string(),
                alternatives: vec![
                    "verify the backing device identity and existing signatures before creation"
                        .to_string(),
                    "grow or migrate an existing VDO volume when preserving data is required"
                        .to_string(),
                    "choose logical size, compression, and deduplication policy before use"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "mdRaids" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "creating an MD RAID array writes array metadata to every member device"
                    .to_string(),
                alternatives: vec![
                    "verify every member device is empty or fully backed up before creation"
                        .to_string(),
                    "assemble and inspect an existing array instead of recreating it".to_string(),
                    "add replacement members to an existing redundant array when preserving data"
                        .to_string(),
                ],
            }),
        ),
        Operation::Assemble if collection == "mdRaids" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "MD RAID assemble activates an existing array from reviewed member devices"
                    .to_string(),
                alternatives: vec![
                    "assemble existing arrays instead of recreating them when metadata already exists"
                        .to_string(),
                    "verify member identities and event counts with mdadm --examine before assemble"
                        .to_string(),
                    "mount or activate consumers only after mdadm reports the array clean or recovering"
                        .to_string(),
                ],
            }),
        ),
        Operation::Stop if collection == "mdRaids" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "MD RAID stop makes the array unavailable without removing member metadata"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before stopping the array"
                        .to_string(),
                    "prefer stop over destroy when preserving member metadata for later assembly"
                        .to_string(),
                    "verify no open consumers remain with lsblk, findmnt, and dmsetup before stopping"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "mdRaids" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "MD RAID rescan refreshes array metadata inventory without assembling arrays"
                        .to_string(),
                alternatives: vec![
                    "use assemble when existing member metadata should activate an array"
                        .to_string(),
                    "inspect member event counts with mdadm --examine before assembly or replacement"
                        .to_string(),
                    "verify /proc/mdstat and dependent consumers after devices reappear"
                        .to_string(),
                ],
            }),
        ),
        Operation::Start if collection == "vdoVolumes" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "VDO start activates an existing VDO volume without rewriting metadata"
                    .to_string(),
                alternatives: vec![
                    "verify the VDO backing device is present before starting".to_string(),
                    "start dependent filesystems, LVM layers, or mounts only after VDO status is healthy"
                        .to_string(),
                    "use create only when intentionally initializing new VDO metadata".to_string(),
                ],
            }),
        ),
        Operation::Stop if collection == "vdoVolumes" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "VDO stop deactivates the volume while preserving VDO metadata".to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate mappings before stopping VDO".to_string(),
                    "prefer stop over remove when the VDO volume should be started again later"
                        .to_string(),
                    "verify no open consumers remain with lsblk, findmnt, and dmsetup before stopping"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "vdoVolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "VDO rescan refreshes status, utilization, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use grow when logical or physical VDO capacity must change".to_string(),
                    "use start or stop only when intentionally changing activation state"
                        .to_string(),
                    "verify vdostats before growing filesystems or dependent volumes"
                    .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "caches" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "bcache rescan refreshes cache state, dirty-data, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use add-device or remove-device only when cache-set attachment must change"
                        .to_string(),
                    "verify dirty data is zero before any later detach or replacement".to_string(),
                    "use cache property updates when changing cache mode or writeback behavior"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "loopDevices" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "loop device creation maps an existing backing file or block device"
                    .to_string(),
                alternatives: vec![
                    "use a stable backing file path and explicit loop device name when needed"
                        .to_string(),
                    "verify the backing file is not concurrently managed elsewhere".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "loopDevices" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "loop device rescan refreshes mapping inventory without changing size"
                    .to_string(),
                alternatives: vec![
                    "use grow only after the backing file or block device size has changed"
                        .to_string(),
                    "inspect dependent filesystems and mappings before detach".to_string(),
                    "keep stable /dev/loop* targets for executable refresh plans".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "backingFiles" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "backing file growth extends a file-backed storage origin before consumer refresh"
                    .to_string(),
                alternatives: vec![
                    "grow file-backed storage before refreshing loop devices or swap signatures"
                        .to_string(),
                    "prefer adding a replacement image and migrating consumers when shrinking is needed"
                        .to_string(),
                    "verify sparse-file allocation and host filesystem free space before growth"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "backingFiles" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "backing file rescan refreshes file size, allocation, and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "use grow only when the file-backed storage origin must be extended"
                        .to_string(),
                    "refresh loop devices after backing file size changes".to_string(),
                    "inspect dependent swap, loop, filesystem, or mapping consumers before detach"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "dmMaps" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "device-mapper rescan refreshes dmsetup map, dependency, table, and status metadata"
                    .to_string(),
                alternatives: vec![
                    "use dmMaps.<name>.operation = \"rescan\" before editing dependent LUKS, LVM, VDO, or multipath layers"
                        .to_string(),
                    "review dmsetup table and status output before any destructive mapper replacement"
                        .to_string(),
                    "use domain-specific LUKS, LVM, VDO, or multipath declarations for mutating mapper lifecycle"
                        .to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "dmMaps" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "device-mapper removal deletes the live map and can make dependent data inaccessible"
                    .to_string(),
                alternatives: vec![
                    "prefer LUKS, LVM, VDO, multipath, or cache-specific close/deactivate/detach declarations when the map is owned by another domain"
                        .to_string(),
                    "run dmMaps.<name>.operation = \"rescan\" and review dmsetup status before removal"
                        .to_string(),
                    "unmount filesystems and stop services before removing the mapper".to_string(),
                ],
            }),
        ),
        _ => return None,
    })
}
