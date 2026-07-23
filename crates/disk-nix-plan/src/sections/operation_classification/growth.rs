fn classify_growth_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Grow if collection == "mdRaids" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "MD RAID grow or reshape requires redundancy, bitmap, and resync coordination"
                    .to_string(),
                alternatives: vec![
                    "add replacement members and wait for sync before reshaping".to_string(),
                    "verify backups and array health before changing size or member count"
                        .to_string(),
                    "monitor /proc/mdstat until reshape and filesystem growth are complete"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "multipathMaps" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "multipath map growth requires path rescan and map resize coordination"
                    .to_string(),
                alternatives: vec![
                    "rescan every backing SCSI path before resizing the map".to_string(),
                    "verify all expected paths are active before growing consumers".to_string(),
                    "reload multipath maps and confirm no stale path reports the old size".to_string(),
                ],
            }),
        ),
        Operation::Destroy if collection == "multipathMaps" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "multipath map removal flushes the host map without deleting target-side data"
                    .to_string(),
                alternatives: vec![
                    "unmount filesystems and deactivate LVM, dm, and service consumers before flushing the map"
                        .to_string(),
                    "remove or drain individual failed paths first when alternate paths must remain active"
                        .to_string(),
                    "use a rescan or reload when the map should stay present and only path metadata changed"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "thinPools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "LVM thin pool growth must account for both data and metadata usage"
                        .to_string(),
                alternatives: vec![
                    "extend thin pool metadata before data exhaustion".to_string(),
                    "verify autoextend thresholds and monitored status before growth".to_string(),
                    "review thin volume overcommit before adding more virtual capacity"
                        .to_string(),
                ],
            }),
        ),
        Operation::Scrub if collection == "pools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "pool scrub verifies data and repairs redundant copies online".to_string(),
                alternatives: vec![
                    "review pool health before starting a scrub".to_string(),
                    "schedule scrubs outside latency-sensitive windows".to_string(),
                    "monitor scrub, resilver, or repair status until completion".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "zvols" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "zvol growth updates volsize and requires consumer capacity verification"
                    .to_string(),
                alternatives: vec![
                    "verify pool free space before increasing volsize".to_string(),
                    "rescan dependent guests, LUNs, or filesystems after growth".to_string(),
                    "grow dependent partitions and filesystems only after the zvol reports the new size"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan
            if collection == "luns"
                || collection == "targetLuns"
                || collection == "iscsiSessions"
                || collection == "nvmeNamespaces"
                || collection == "multipathMaps" =>
        {
            (
                RiskClass::Online,
                false,
                Some(Advice {
                    summary:
                        "host rescan refreshes existing storage paths without deleting target data"
                            .to_string(),
                    alternatives: vec![
                        "use grow when the target-side capacity changed and consumers must be resized"
                            .to_string(),
                        "declare stable path devices so apply can verify each refreshed path"
                            .to_string(),
                        "verify multipath and dependent volumes after the rescan".to_string(),
                    ],
                }),
            )
        }
        Operation::Rescan if collection == "physicalVolumes" || collection == "volumeGroups" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "LVM rescan refreshes PV cache and VG metadata without deleting data"
                        .to_string(),
                alternatives: vec![
                    "use grow when backing device capacity changed and PV or LV sizes must be updated"
                        .to_string(),
                    "rescan block paths before refreshing LVM metadata on newly visible devices"
                        .to_string(),
                    "verify VG free extents and LV activation state after the metadata refresh"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "targetLuns" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "target-side LUN growth changes capacity on the storage target"
                    .to_string(),
                alternatives: vec![
                    "grow the target object before host rescans and consumer resizes".to_string(),
                    "verify snapshots, replication, and thin provisioning limits before increasing capacity"
                        .to_string(),
                    "stage host-side luns and multipath rescans after the target reports the new size"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "luns" || collection == "iscsiSessions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "network LUN growth must be coordinated with the storage target and host rescan"
                        .to_string(),
                alternatives: vec![
                    "grow the target LUN before resizing consumers".to_string(),
                    "rescan SCSI paths and verify multipath before filesystem growth".to_string(),
                    "confirm every dependent filesystem or volume sees the new capacity"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "partitions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "partition growth may require inactive consumers and a kernel partition table reread"
                    .to_string(),
                alternatives: vec![
                    "grow the backing disk or LUN before resizing the partition".to_string(),
                    "verify dependent LUKS, LVM, and filesystem layers before resizing consumers"
                        .to_string(),
                    "schedule a reboot when active consumers prevent partition table reread"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "vdoVolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "VDO growth must distinguish logical size from backing physical capacity"
                    .to_string(),
                alternatives: vec![
                    "grow physical backing storage before VDO physical growth".to_string(),
                    "grow logical size only after confirming pool utilization and slab health"
                        .to_string(),
                    "verify vdostats and dependent filesystems after the grow".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "physicalVolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LVM physical volume growth refreshes PV size after backing storage grows"
                    .to_string(),
                alternatives: vec![
                    "grow the backing partition, LUN, or disk before pvresize".to_string(),
                    "verify VG free extents before extending logical volumes".to_string(),
                    "coordinate dependent LV and filesystem growth after pvresize".to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "loopDevices" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "loop device growth refreshes the mapping after backing size changes"
                    .to_string(),
                alternatives: vec![
                    "grow the backing file or block device before refreshing the loop mapping"
                        .to_string(),
                    "resize dependent partitions or filesystems only after losetup reports the new size"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow if collection == "nvmeNamespaces" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "NVMe namespace growth is represented as host rescan after controller-side changes"
                    .to_string(),
                alternatives: vec![
                    "resize or recreate the namespace on the controller before host rescan"
                        .to_string(),
                    "rescan the controller and verify namespace capacity before growing consumers"
                        .to_string(),
                    "prefer replacement namespace migration when controller resize is unsupported"
                        .to_string(),
                ],
            }),
        ),
        Operation::Attach if collection == "nvmeNamespaces" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NVMe namespace attach exposes an existing namespace to selected controllers"
                    .to_string(),
                alternatives: vec![
                    "attach an existing namespace instead of creating one when preserving data"
                        .to_string(),
                    "verify namespace id and controller list with nvme list-ns before attach"
                        .to_string(),
                    "rescan the controller and verify dependent consumers after attachment"
                        .to_string(),
                ],
            }),
        ),
        Operation::Grow | Operation::AddDevice | Operation::Rebalance => {
            (RiskClass::Online, false, None)
        }
        Operation::ReplaceDevice => {
            let (risk, advice) = classify_replace_device(collection);
            (risk, false, Some(advice))
        }
        Operation::Snapshot => (RiskClass::Reversible, false, None),
        _ => return None,
    })
}
