fn classify_local_filesystem_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Create if collection == "disks" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "creating or replacing a disk partition table can hide existing data"
                    .to_string(),
                alternatives: destructive_alternatives(collection, object),
            }),
        ),
        Operation::Check if collection == "filesystems" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "filesystem consistency checks require a stable source device"
                    .to_string(),
                alternatives: vec![
                    "prefer read-only checks before any repair attempt".to_string(),
                    "unmount or quiesce the filesystem when the checker requires it".to_string(),
                    "capture current topology and recent backups before maintenance".to_string(),
                ],
            }),
        ),
        Operation::Repair if collection == "filesystems" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "filesystem repair mutates metadata and must be reviewed offline"
                    .to_string(),
                alternatives: vec![
                    "run a read-only check first and review the reported damage".to_string(),
                    "restore from backup or snapshot when repair risk is unacceptable".to_string(),
                    "repair a cloned block device before touching the production source"
                        .to_string(),
                ],
            }),
        ),
        Operation::Scrub if collection == "filesystems" => {
            match string_field(object, &["fsType", "type"]).as_deref() {
                Some("btrfs") => (
                    RiskClass::Online,
                    false,
                    Some(Advice {
                        summary: "Btrfs scrub verifies checksums and repairs redundant data online"
                            .to_string(),
                        alternatives: vec![
                            "run a read-only filesystem check when metadata corruption is suspected"
                                .to_string(),
                            "verify device health and backups before scrubbing degraded filesystems"
                                .to_string(),
                            "monitor scrub status until completion".to_string(),
                        ],
                    }),
                ),
                Some("bcachefs") => (
                    RiskClass::Online,
                    false,
                    Some(Advice {
                        summary: "bcachefs scrub verifies filesystem data and metadata online"
                            .to_string(),
                        alternatives: vec![
                            "review bcachefs fs usage and device health before scrubbing"
                                .to_string(),
                            "run offline filesystem checks when metadata corruption is suspected"
                                .to_string(),
                            "monitor scrub output until completion".to_string(),
                        ],
                    }),
                ),
                _ => (
                    RiskClass::Unsupported,
                    false,
                    Some(Advice {
                        summary:
                            "filesystem scrub command mapping is currently available for Btrfs and bcachefs"
                                .to_string(),
                        alternatives: vec![
                            "use filesystem check for ext or XFS consistency validation"
                                .to_string(),
                            "model ZFS scrubs through pool lifecycle declarations".to_string(),
                            "run filesystem-specific scrub tooling manually after review"
                                .to_string(),
                        ],
                    }),
                ),
            }
        }
        Operation::Trim if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "filesystem trim discards unused blocks on the mounted filesystem"
                    .to_string(),
                alternatives: vec![
                    "verify discard passthrough on encrypted or virtual block layers first"
                        .to_string(),
                    "prefer scheduled fstrim for steady-state maintenance".to_string(),
                    "run trim outside latency-sensitive windows on thin or remote storage"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "filesystem rescan refreshes mount and graph inventory without changing data"
                    .to_string(),
                alternatives: vec![
                    "use rescan before mount, remount, trim, check, or repair planning when current state may be stale"
                        .to_string(),
                    "use filesystem-specific check or scrub operations when integrity validation is needed"
                        .to_string(),
                    "persist steady-state mount declarations through NixOS fileSystems"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "filesystem remount updates local mount options without rewriting data"
                        .to_string(),
                alternatives: vec![
                    "prefer remounting with reviewed options before unmounting a busy path"
                        .to_string(),
                    "persist long-lived option changes through NixOS fileSystems".to_string(),
                    "verify active services tolerate option changes such as ro, rw, or discard"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "partitions" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "partition creation changes on-disk metadata and requires kernel reread coordination"
                        .to_string(),
                alternatives: vec![
                    "verify the target disk, free region, and partition table before applying"
                        .to_string(),
                    "prefer stable /dev/disk/by-id paths for disk selection".to_string(),
                    "run partprobe or reboot if the kernel cannot reread the table".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "disks" || collection == "partitions" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "partition-table rescan refreshes kernel disk and partition inventory"
                        .to_string(),
                alternatives: vec![
                    "use grow when partition geometry must change before the reread".to_string(),
                    "pause dependent consumers when the kernel cannot reread an active table"
                        .to_string(),
                    "verify stable by-id and by-partuuid paths after the rescan".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "backingFiles" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "backing file creation initializes a new sparse file-backed storage origin"
                    .to_string(),
                alternatives: vec![
                    "verify the parent filesystem has enough free space before creating sparse images"
                        .to_string(),
                    "use grow only when an existing backing file should be extended".to_string(),
                    "create loop, swap, or filesystem consumers only after the file identity is verified"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "btrfsSubvolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "Btrfs subvolume creation is reversible but changes namespace layout"
                    .to_string(),
                alternatives: vec![
                    "create the subvolume at an empty reviewed path".to_string(),
                    "prefer read-only snapshots or clones for migrations".to_string(),
                    "verify parent mount and qgroup policy before creation".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "btrfsSubvolumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "Btrfs subvolume rescan refreshes subvolume metadata and read-only state"
                        .to_string(),
                alternatives: vec![
                    "use property updates only when read-only enforcement must change"
                        .to_string(),
                    "inspect qgroup and snapshot relationships before destructive cleanup"
                        .to_string(),
                    "verify consumers still mount the intended subvolume path".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "btrfsQgroups" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary:
                    "Btrfs qgroup rescan refreshes quota hierarchy, limits, and usage"
                        .to_string(),
                alternatives: vec![
                    "use limit property updates only when quota enforcement must change"
                        .to_string(),
                    "inspect qgroup usage before tightening referenced or exclusive limits"
                        .to_string(),
                    "verify quota accounting and subvolume relationships before qgroup removal"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "zvols" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "zvol creation allocates a block volume inside an existing ZFS pool"
                    .to_string(),
                alternatives: vec![
                    "verify pool free space and refreservation policy before creation".to_string(),
                    "use sparse volumes only when overcommit is intentional".to_string(),
                    "create consumers only after the zvol appears by stable /dev/zvol path"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "zvols" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "zvol rescan refreshes ZFS volume properties and block graph state"
                    .to_string(),
                alternatives: vec![
                    "use grow only when volsize must change".to_string(),
                    "inspect dependent guests, LUNs, and filesystems before changing consumers"
                        .to_string(),
                    "snapshot or clone the zvol before destructive cleanup".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "pools" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "ZFS pool creation writes pool labels to every selected device"
                    .to_string(),
                alternatives: vec![
                    "verify every vdev device is empty or fully backed up before creation"
                        .to_string(),
                    "import an existing pool instead of recreating it".to_string(),
                    "use stable /dev/disk/by-id paths and review redundancy layout before zpool create"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "datasets" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "ZFS dataset creation adds a filesystem namespace inside an existing pool"
                    .to_string(),
                alternatives: vec![
                    "verify parent dataset properties before creating children".to_string(),
                    "set mountpoint, quota, reservation, and encryption policy before use"
                        .to_string(),
                    "create snapshots or consumers only after the dataset appears in zfs list"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "datasets" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "ZFS dataset rescan refreshes dataset properties, mounts, and graph state"
                    .to_string(),
                alternatives: vec![
                    "use property updates only when mountpoint, quota, or reservation policy must change"
                        .to_string(),
                    "inspect snapshots and clones before promote, rollback, or destroy work"
                        .to_string(),
                    "verify consumers still use the intended mounted dataset".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "volumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "logical volume creation allocates a new volume inside an existing volume group"
                    .to_string(),
                alternatives: vec![
                    "verify volume group free extents before creating the logical volume"
                        .to_string(),
                    "use an explicit desired size and stable LV name".to_string(),
                    "create filesystems or mappings only after the LV path appears".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "volumes" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "logical volume rescan refreshes LV attributes, size, and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "use grow only when logical volume capacity must change".to_string(),
                    "use activate or deactivate only when LV visibility must change".to_string(),
                    "verify dependent filesystems or mappings after status refresh".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "physicalVolumes" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "LVM physical volume creation writes PV metadata to the selected device"
                    .to_string(),
                alternatives: vec![
                    "inspect signatures and backups before pvcreate".to_string(),
                    "reuse an existing PV when preserving volume-group data".to_string(),
                    "add a new device to the VG instead of reinitializing an existing PV"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::AddKey if collection == "luksKeyslots" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "adding a LUKS keyslot changes access to the encrypted container"
                    .to_string(),
                alternatives: vec![
                    "back up the LUKS header before enrolling new key material".to_string(),
                    "test the new key before removing any existing recovery key".to_string(),
                    "use an explicit keyslot only when site policy requires stable slot assignment"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::ImportToken if collection == "luksTokens" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary: "importing a LUKS token changes automated unlock access".to_string(),
                alternatives: vec![
                    "back up the LUKS header before importing token metadata".to_string(),
                    "verify a recovery key or passphrase works before relying on the token"
                        .to_string(),
                    "test the token unlock path before removing older tokens".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "thinPools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "thin pool creation allocates a new LVM thin-pool data volume inside an existing volume group"
                    .to_string(),
                alternatives: vec![
                    "verify volume group free extents before creating the thin pool".to_string(),
                    "choose explicit pool size and monitor metadata utilization from first use"
                        .to_string(),
                    "review thin-volume overcommit policy before exposing consumers".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "thinPools" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "thin pool rescan refreshes data, metadata, monitoring, and graph status"
                    .to_string(),
                alternatives: vec![
                    "use grow only when data or metadata capacity must change".to_string(),
                    "verify data and metadata utilization before creating more thin volumes"
                        .to_string(),
                    "review autoextend and monitoring policy before pool exhaustion".to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "lvmCaches" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "LVM cache attachment converts an origin LV to use a reviewed cache pool"
                        .to_string(),
                alternatives: vec![
                    "attach cache only after the cache pool LV and origin LV are both verified"
                        .to_string(),
                    "use writethrough mode first when data safety is more important than write latency"
                        .to_string(),
                    "snapshot or back up the origin LV before enabling writeback cache".to_string(),
                ],
            }),
        ),
        Operation::AddDevice if collection == "lvmCaches" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "LVM cache attachment changes origin LV write paths through a cache pool"
                        .to_string(),
                alternatives: vec![
                    "verify the cache pool LV belongs to the same volume group as the origin"
                        .to_string(),
                    "start in writethrough mode when rollback safety matters".to_string(),
                    "keep the origin LV snapshot or backup until cache verification passes"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "lvmCaches" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LVM cache rescan refreshes origin, cache mode, policy, and dirty-data reports"
                    .to_string(),
                alternatives: vec![
                    "use property updates when cache mode or cache policy must change".to_string(),
                    "use remove-device only after dirty cache data has drained".to_string(),
                    "verify origin LV readability before any later cache detach or replacement"
                        .to_string(),
                ],
            }),
        ),
        _ => return None,
    })
}
