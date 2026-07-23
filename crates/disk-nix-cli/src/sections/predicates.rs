fn is_device_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::PhysicalDisk
            | NodeKind::Partition
            | NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmPhysicalVolume
            | NodeKind::LvmVolumeGroup
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::Zvol
            | NodeKind::CacheDevice
            | NodeKind::MultipathDevice
            | NodeKind::NvmeSubsystem
            | NodeKind::NvmeNamespace
            | NodeKind::LoopDevice
            | NodeKind::BcachefsDevice
            | NodeKind::BackingFile
            | NodeKind::ZramDevice
            | NodeKind::Swap
    )
}

fn is_partition_node(node: &Node) -> bool {
    node.kind == NodeKind::Partition
}

fn is_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Filesystem
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::NfsExport
    )
}

fn is_complex_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::BcachefsDevice
            | NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
    ) || node.properties.iter().any(|property| {
        property.key.starts_with("btrfs.")
            || property.key.starts_with("bcachefs.")
            || property.key.starts_with("zfs.")
    })
}

fn is_btrfs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("btrfs."))
}

fn is_bcachefs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BcachefsFilesystem | NodeKind::BcachefsDevice
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("bcachefs."))
}

fn is_zfs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("zfs."))
}

fn is_volume_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsPool
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
            | NodeKind::Lun
            | NodeKind::NfsExport
    )
}

fn is_pool_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmThinPool
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::MdRaid
    )
}

fn is_snapshot_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmSnapshot | NodeKind::BtrfsSnapshot | NodeKind::ZfsSnapshot
    )
}

fn is_mapping_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::MultipathDevice
            | NodeKind::LoopDevice
            | NodeKind::CacheDevice
            | NodeKind::BcachefsDevice
    )
}

fn is_dm_node(node: &Node) -> bool {
    node.kind == NodeKind::DeviceMapper
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("dm."))
}

fn is_encryption_node(node: &Node) -> bool {
    node.kind == NodeKind::LuksContainer
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("cryptsetup."))
}

fn is_cache_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmCache | NodeKind::CacheDevice | NodeKind::BcachefsDevice
    ) || node.properties.iter().any(|property| {
        property.key.starts_with("bcache.")
            || property.key.starts_with("bcachefs.device-")
            || property.key == "lvm.cache-mode"
            || property.key == "lvm.cache-policy"
            || property.key == "lvm.kernel-cache-mode"
            || property.key == "lvm.kernel-cache-policy"
            || property.key == "lvm.cache-metadata-format"
            || property.key == "lvm.segment-cache-mode"
            || property.key == "lvm.segment-cache-policy"
            || property.key == "lvm.cache-settings"
            || property.key.starts_with("lvm.writecache-")
            || (property.key == "zfs.vdev-role" && property.value == "cache")
    })
}

fn is_lvm_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmPhysicalVolume
            | NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("lvm."))
}

fn is_vdo_node(node: &Node) -> bool {
    node.kind == NodeKind::VdoVolume
        || node.properties.iter().any(|property| {
            property.key.starts_with("vdo.") || property.key.starts_with("lvm.vdo-")
        })
}

fn is_multipath_node(node: &Node) -> bool {
    node.kind == NodeKind::MultipathDevice
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("multipath."))
}

fn is_nvme_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::NvmeSubsystem | NodeKind::NvmeController | NodeKind::NvmeNamespace
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("nvme."))
}

fn is_raid_node(node: &Node) -> bool {
    node.kind == NodeKind::MdRaid
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("md."))
}

fn is_loop_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::LoopDevice | NodeKind::BackingFile)
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("loop."))
}

fn is_backing_file_node(node: &Node) -> bool {
    node.kind == NodeKind::BackingFile
}

fn is_swap_node(node: &Node) -> bool {
    node.kind == NodeKind::Swap
        || node.kind == NodeKind::ZramDevice
        || property_value(node, "zram.swap") == Some("true")
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("swap."))
}

fn is_zram_node(node: &Node) -> bool {
    node.kind == NodeKind::ZramDevice
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("zram."))
}

fn is_iscsi_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::IscsiSession | NodeKind::IscsiTarget | NodeKind::Lun
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("iscsi."))
}

fn is_lun_node(node: &Node) -> bool {
    node.kind == NodeKind::Lun
}

fn is_nfs_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::NfsExport | NodeKind::NfsMount)
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("nfs."))
}

fn is_mount_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Mountpoint | NodeKind::NfsMount)
}

fn is_network_storage_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::IscsiSession
            | NodeKind::IscsiTarget
            | NodeKind::Lun
            | NodeKind::NfsExport
            | NodeKind::NfsMount
    )
}

fn has_capacity_or_usage(node: &Node) -> bool {
    node.size_bytes.is_some()
        || node.usage.as_ref().is_some_and(|usage| {
            usage.used_bytes.is_some()
                || usage.free_bytes.is_some()
                || usage.allocated_bytes.is_some()
        })
}
