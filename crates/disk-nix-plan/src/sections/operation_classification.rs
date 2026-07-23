fn classify_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> (RiskClass, bool, Option<Advice>) {
    match operation {
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
        Operation::Create | Operation::Export if collection == "exports" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS export creation publishes an existing path to selected clients"
                    .to_string(),
                alternatives: vec![
                    "export read-only first when client behavior is unknown".to_string(),
                    "restrict clients and options before enabling write access".to_string(),
                    "verify the source path and ownership before reloading exports".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "exports" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS export rescan refreshes exported path and client visibility"
                    .to_string(),
                alternatives: vec![
                    "use option property updates only when client access semantics must change"
                        .to_string(),
                    "verify active clients before unexporting or tightening access".to_string(),
                    "persist long-lived exports through NixOS services.nfs.server.exports"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Login if collection == "iscsiSessions" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "iSCSI session login discovers target portals and attaches remote LUNs"
                    .to_string(),
                alternatives: vec![
                    "verify the portal and target IQN before logging in".to_string(),
                    "prefer stable multipath and by-id consumers before resizing filesystems"
                        .to_string(),
                    "keep NixOS open-iscsi session declarations aligned with imperative login"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Mount if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "mounting an NFS client path changes host namespace state".to_string(),
                alternatives: vec![
                    "use x-systemd.automount or nofail when the server may be unavailable"
                        .to_string(),
                    "verify DNS, routing, firewall, and export permissions before mounting"
                        .to_string(),
                    "prefer declarative NixOS fileSystems for steady-state client mounts"
                        .to_string(),
                ],
            }),
        ),
        Operation::Mount if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "mounting a filesystem changes local namespace state without formatting storage"
                    .to_string(),
                alternatives: vec![
                    "verify the source device, filesystem type, and mountpoint before mounting"
                        .to_string(),
                    "prefer x-systemd.automount, nofail, or service ordering when dependencies may be unavailable"
                        .to_string(),
                    "persist long-lived mounts through the matching NixOS fileSystems entry"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "remounting an NFS client path updates local mount options without deleting remote data"
                    .to_string(),
                alternatives: vec![
                    "prefer remounting with reviewed options before unmounting a busy path"
                        .to_string(),
                    "use NixOS fileSystems for the steady-state mount options".to_string(),
                    "verify active services tolerate option changes such as ro, rw, or timeouts"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS mount rescan refreshes local mount source, options, and client stats"
                    .to_string(),
                alternatives: vec![
                    "use remount only when local mount options must change".to_string(),
                    "verify server reachability before unmounting busy client paths".to_string(),
                    "persist long-lived mounts through NixOS fileSystems".to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Attach if collection == "luns" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LUN host attach makes an existing target-side LUN visible to this host"
                    .to_string(),
                alternatives: vec![
                    "create or grow the target-side LUN before host attach".to_string(),
                    "declare stable by-path devices so apply can verify every expected path"
                        .to_string(),
                    "keep multipath and filesystem consumers disabled until paths are verified"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Attach if collection == "targetLuns" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "target-side LUN provisioning allocates or maps storage on an external target"
                        .to_string(),
                alternatives: vec![
                    "use an array-specific provider or storage runbook before host-side attach"
                        .to_string(),
                    "record the backing object, initiator mapping, LUN number, and expected size before rescanning hosts"
                        .to_string(),
                    "prefer mapping an existing reviewed LUN when preserving data is required"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "nvmeNamespaces" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "NVMe namespace creation allocates controller-managed namespace capacity"
                    .to_string(),
                alternatives: vec![
                    "attach an existing namespace when preserving data is required".to_string(),
                    "verify controller namespace inventory before create-ns".to_string(),
                    "declare namespaceId and controllers before attaching the created namespace"
                        .to_string(),
                ],
            }),
        ),
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
    }
}

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
