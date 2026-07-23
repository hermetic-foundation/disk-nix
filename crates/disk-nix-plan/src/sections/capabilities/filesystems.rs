fn capability_group_filesystems() -> Vec<Capability> {
    vec![
        Capability {
            node_kind: NodeKind::LoopDevice,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "loop device rescan refreshes mapping inventory without mutation"
                    .to_string(),
                alternatives: vec![
                    "use grow only when backing size changed".to_string(),
                    "detach only after consumers are stopped".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LoopDevice,
            operation: Operation::Destroy,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "loop detach requires inactive consumers".to_string(),
                alternatives: vec![
                    "unmount filesystems before detach".to_string(),
                    "preserve the backing file for remapping".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BackingFile,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "backing file creation initializes a new sparse file-backed storage origin"
                    .to_string(),
                alternatives: vec![
                    "verify the parent filesystem has enough free space first".to_string(),
                    "attach loop devices or swap only after the file is verified".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BackingFile,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "backing file growth extends file-backed storage before consumer refresh"
                    .to_string(),
                alternatives: vec![
                    "verify host filesystem free space before extending sparse or preallocated images"
                        .to_string(),
                    "refresh loop, swap, filesystem, or mapping consumers after growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BackingFile,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "backing file rescan refreshes size, allocation, and consumer relationships"
                    .to_string(),
                alternatives: vec![
                    "use grow when the file-backed origin capacity must change".to_string(),
                    "inspect consumers before detaching loop devices or disabling swap".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::DeviceMapper,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "device-mapper rescan refreshes map identity, dependencies, table, and status metadata"
                    .to_string(),
                alternatives: vec![
                    "use LUKS, LVM, VDO, multipath, or cache declarations for domain-specific mutations"
                        .to_string(),
                    "review dmsetup status before changing dependent filesystems or volumes"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::DeviceMapper,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "device-mapper rename changes the visible mapper path without deleting mapped data"
                    .to_string(),
                alternatives: vec![
                    "update dependent LUKS, LVM, VDO, multipath, filesystem, mount, and service declarations before applying"
                        .to_string(),
                    "prefer the owning LUKS, LVM, VDO, multipath, or cache declaration when the map is domain-managed"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::DeviceMapper,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "device-mapper removal deletes the live map and can make dependent data inaccessible"
                    .to_string(),
                alternatives: vec![
                    "use domain-specific LUKS, LVM, VDO, multipath, or cache teardown when available"
                        .to_string(),
                    "review dmsetup status and dependent mounts before removal".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating a ZFS pool writes labels to member devices".to_string(),
                alternatives: vec![
                    "verify devices are empty before zpool create".to_string(),
                    "import an existing pool instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: None,
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Import,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS pool import activates an existing pool on this host".to_string(),
                alternatives: vec![
                    "import read-only first when validating moved storage".to_string(),
                    "verify hostid, cachefile, mountpoints, and encryption keys".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Export,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS pool export cleanly detaches a pool without deleting it".to_string(),
                alternatives: vec![
                    "export instead of destroying a pool that will be moved".to_string(),
                    "stop services, shares, and LUN mappings before export".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS pool device replacement must preserve pool health through resilver"
                    .to_string(),
                alternatives: vec![
                    "attach or add replacement capacity before removing a weak vdev".to_string(),
                    "monitor zpool status until resilver completes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "ZFS pool device removal depends on pool topology and evacuation support"
                    .to_string(),
                alternatives: vec![
                    "replace the device instead when removal is not supported".to_string(),
                    "verify pool free space and health before starting evacuation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "ZFS pool property updates use zpool set on the reviewed pool"
                    .to_string(),
                alternatives: vec![
                    "inspect current pool properties before changing behavior".to_string(),
                    "prefer reversible property changes before topology changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsPool,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a ZFS pool removes the pool and all contained datasets"
                    .to_string(),
                alternatives: vec![
                    "export the pool when moving it between systems".to_string(),
                    "take recursive snapshots and verify backups before destruction".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "adding a Btrfs device expands the mounted filesystem device set"
                    .to_string(),
                alternatives: vec![
                    "verify the new block device identity before adding it".to_string(),
                    "run a filtered balance after adding capacity when profiles need reshaping"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "Btrfs device replacement must preserve live filesystem availability"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removing a failing device".to_string(),
                    "monitor btrfs replace status until the operation completes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsFilesystem,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary:
                    "removing a Btrfs device requires enough remaining data and metadata space"
                        .to_string(),
                alternatives: vec![
                    "run a filtered balance before removal".to_string(),
                    "add replacement capacity before removing the old device".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "bcachefs member growth expands a mounted filesystem after backing capacity changes"
                    .to_string(),
                alternatives: vec![
                    "verify the member device and target size before resizing".to_string(),
                    "refresh bcachefs usage after growth before resizing consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "bcachefs rescan refreshes filesystem and member-device usage metadata"
                    .to_string(),
                alternatives: vec![
                    "run rescan before device replacement or removal planning".to_string(),
                    "review per-device free and data-type byte accounting".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::AddDevice,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "adding a bcachefs member expands the mounted filesystem device set"
                    .to_string(),
                alternatives: vec![
                    "verify the new block device identity before adding it".to_string(),
                    "rereplicate data after topology changes when replicas or durability changed"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::ReplaceDevice,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "bcachefs replacement should add new capacity, rereplicate data, then remove the old member"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before evacuating old media".to_string(),
                    "inspect rereplication status before removing the old member".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BcachefsFilesystem,
            operation: Operation::RemoveDevice,
            risk: RiskClass::PotentialDataLoss,
            advice: Some(Advice {
                summary: "bcachefs member removal requires enough remaining capacity and replicas"
                    .to_string(),
                alternatives: vec![
                    "add replacement capacity before removal".to_string(),
                    "rereplicate data and verify free metadata capacity before final removal"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs subvolume creation changes namespace layout".to_string(),
                alternatives: vec![
                    "create at an empty reviewed path".to_string(),
                    "verify qgroup policy before creation".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "deleting a Btrfs subvolume removes its live tree".to_string(),
                alternatives: vec![
                    "take a read-only snapshot before deletion".to_string(),
                    "rename the subvolume before final removal".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "Btrfs subvolume property updates support read-only toggles".to_string(),
                alternatives: vec![
                    "use readOnly, readonly, ro, btrfs.readonly, or btrfs.ro".to_string(),
                    "review unsupported subvolume properties manually before changing them"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs subvolume rescan refreshes metadata without changing data"
                    .to_string(),
                alternatives: vec![
                    "use read-only property updates only when enforcement must change"
                        .to_string(),
                    "inspect qgroups and snapshots before cleanup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsSubvolume,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "Btrfs subvolume rename stages a path move before deletion".to_string(),
                alternatives: vec![
                    "update mounts and qgroups before moving the path".to_string(),
                    "validate consumers on the renamed subvolume before deleting old paths"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs qgroup creation changes quota hierarchy for a mounted filesystem"
                    .to_string(),
                alternatives: vec![
                    "enable quota accounting and inspect existing qgroups before creation"
                        .to_string(),
                    "create qgroups before assigning subvolume limits".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "Btrfs qgroup limit changes alter referenced or exclusive quota enforcement"
                    .to_string(),
                alternatives: vec![
                    "inspect current referenced and exclusive usage before tightening limits"
                        .to_string(),
                    "raise limits temporarily before migrations or balance operations".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "Btrfs qgroup rescan refreshes quota usage and hierarchy"
                    .to_string(),
                alternatives: vec![
                    "inspect referenced and exclusive usage before tightening limits"
                        .to_string(),
                    "use property updates only when quota enforcement must change".to_string(),
                    "verify quota accounting before deleting or replacing qgroups".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::BtrfsQgroup,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a Btrfs qgroup removes quota policy for that group"
                    .to_string(),
                alternatives: vec![
                    "clear limits or move subvolumes to a replacement qgroup first".to_string(),
                    "verify quota hierarchy and usage after removing the qgroup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zvol creation consumes ZFS pool capacity".to_string(),
                alternatives: vec![
                    "verify free pool capacity before creation".to_string(),
                    "decide sparse versus reserved allocation before exposing the block device"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zvol growth changes volsize for downstream block consumers".to_string(),
                alternatives: vec![
                    "verify pool free space before changing volsize".to_string(),
                    "rescan dependent block consumers after growth".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "zvol property updates use zfs set on the reviewed block volume"
                    .to_string(),
                alternatives: vec![
                    "verify dependent guests or LUN exports before changing zvol behavior"
                        .to_string(),
                    "snapshot or clone the zvol before risky property changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "zvol rescan refreshes volume properties and block graph state"
                    .to_string(),
                alternatives: vec![
                    "use grow only when volsize must change".to_string(),
                    "review dependent guests and LUN exports before changing consumers"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "zvol rename preserves block data while changing the ZFS volume name"
                    .to_string(),
                alternatives: vec![
                    "detach or rescan downstream LUN, VM, and filesystem consumers first"
                        .to_string(),
                    "validate consumers on the renamed zvol before removing old references"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Promote,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "zvol clone promotion changes clone dependency ownership".to_string(),
                alternatives: vec![
                    "inspect origin and consumers before promoting the clone".to_string(),
                    "validate downstream LUN, VM, and filesystem consumers after promotion"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::Zvol,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "destroying a zvol removes the block volume and its data".to_string(),
                alternatives: vec![
                    "snapshot or clone the zvol before destruction".to_string(),
                    "detach downstream LUN, VM, or filesystem consumers first".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "ZFS dataset creation consumes pool namespace and inherits parent policy"
                    .to_string(),
                alternatives: vec![
                    "review inherited mountpoint, quota, reservation, and encryption properties"
                        .to_string(),
                    "create under the intended parent dataset before exposing consumers"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            advice: Some(Advice {
                summary: "ZFS dataset property updates use zfs set on the reviewed dataset"
                    .to_string(),
                alternatives: vec![
                    "review inherited quota, reservation, mountpoint, and encryption policy first"
                        .to_string(),
                    "snapshot datasets before property changes that affect consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "ZFS dataset rescan refreshes properties, mounts, and graph state"
                    .to_string(),
                alternatives: vec![
                    "use property updates only when dataset policy must change".to_string(),
                    "inspect snapshots and clones before destructive cleanup".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS dataset rename preserves data while changing its dataset name"
                    .to_string(),
                alternatives: vec![
                    "update mountpoints, shares, and services before rename".to_string(),
                    "validate consumers on the renamed dataset before destroying old references"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::ZfsDataset,
            operation: Operation::Promote,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "ZFS dataset clone promotion changes clone dependency ownership"
                    .to_string(),
                alternatives: vec![
                    "inspect clone origin and dependent snapshots before promotion".to_string(),
                    "validate mounts, shares, and services against the promoted dataset"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating a VDO volume writes metadata to the backing device".to_string(),
                alternatives: vec![
                    "inspect existing signatures before creation".to_string(),
                    "migrate data or grow an existing VDO volume instead of recreating it"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::VdoVolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing a VDO volume destroys the deduplicated block layer".to_string(),
                alternatives: vec![
                    "migrate data away from the VDO device before removal".to_string(),
                    "deactivate dependent filesystems and mappings before vdo remove".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "physical volume creation writes LVM metadata to the device".to_string(),
                alternatives: vec![
                    "inspect existing signatures before pvcreate".to_string(),
                    "reuse the existing PV when preserving VG data".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "physical volume growth refreshes LVM capacity after backing growth"
                    .to_string(),
                alternatives: vec![
                    "grow backing storage before pvresize".to_string(),
                    "verify VG free extents after pvresize".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "physical volume rescan refreshes the LVM device cache".to_string(),
                alternatives: vec![
                    "rescan the underlying block path before refreshing LVM metadata".to_string(),
                    "use grow when pvresize is required after backing capacity changes".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmPhysicalVolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "physical volume removal erases LVM metadata from the device".to_string(),
                alternatives: vec![
                    "pvmove and vgreduce before pvremove".to_string(),
                    "verify no volume group still uses the PV".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Create,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "logical volume creation consumes free extents in a volume group"
                    .to_string(),
                alternatives: vec![
                    "verify VG free space before allocation".to_string(),
                    "choose explicit LV size and naming before formatting consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "logical volume growth consumes free extents from the volume group"
                    .to_string(),
                alternatives: vec![
                    "verify volume group free space before lvextend".to_string(),
                    "grow the filesystem only after the LV reports the new size".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "logical volume status refresh reads LV attributes and graph relationships"
                    .to_string(),
                alternatives: vec![
                    "grow only when capacity must change".to_string(),
                    "activate or deactivate only when availability must change".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "removing a logical volume destroys its contents".to_string(),
                alternatives: vec![
                    "snapshot the LV before removal".to_string(),
                    "rename or deactivate the LV while validating consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Activate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "logical volume activation exposes an existing LV without creating it"
                    .to_string(),
                alternatives: vec![
                    "inspect LV metadata and dependent mappings before activation".to_string(),
                    "activate only the reviewed LV needed by consumers".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "logical volume deactivation hides an LV without deleting data".to_string(),
                alternatives: vec![
                    "unmount filesystems and stop services before deactivation".to_string(),
                    "deactivate instead of removing an LV when preserving data".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmLogicalVolume,
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "logical volume rename changes the LV path without deleting data"
                    .to_string(),
                alternatives: vec![
                    "update crypttab, fileSystems, LUN exports, and services before rename"
                        .to_string(),
                    "validate consumers with the renamed LV before removing old declarations"
                        .to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Create,
            risk: RiskClass::Destructive,
            advice: Some(Advice {
                summary: "creating an LVM volume group writes metadata to member physical volumes"
                    .to_string(),
                alternatives: vec![
                    "inspect pvs and block identity before creation".to_string(),
                    "extend an existing volume group instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Grow,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "volume group growth adds reviewed physical volumes to the VG"
                    .to_string(),
                alternatives: vec![
                    "inspect the candidate PV before vgextend".to_string(),
                    "extend the existing VG instead of recreating it".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            advice: Some(Advice {
                summary: "volume group rescan refreshes LVM metadata and active LV tables"
                    .to_string(),
                alternatives: vec![
                    "run block and PV rescans first when storage paths changed".to_string(),
                    "verify LV activation state and VG free extents after refresh".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Import,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group import reactivates an exported VG without recreating it"
                    .to_string(),
                alternatives: vec![
                    "inspect PV identities and VG UUIDs before vgimport".to_string(),
                    "activate consumers only after imported metadata is verified".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Export,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group export prepares a VG for movement without deleting data"
                    .to_string(),
                alternatives: vec![
                    "deactivate logical volumes before vgexport".to_string(),
                    "export instead of removing a VG that will be moved".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Activate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group activation makes contained LVs available".to_string(),
                alternatives: vec![
                    "inspect PV membership and VG metadata before vgchange -ay".to_string(),
                    "activate only reviewed VGs needed by the host".to_string(),
                ],
            }),
        },
        Capability {
            node_kind: NodeKind::LvmVolumeGroup,
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            advice: Some(Advice {
                summary: "volume group deactivation makes contained LVs unavailable without deletion"
                    .to_string(),
                alternatives: vec![
                    "stop mounts, mappings, and services before vgchange -an".to_string(),
                    "deactivate instead of removing a VG when preserving storage".to_string(),
                ],
            }),
        },
    ]
}
